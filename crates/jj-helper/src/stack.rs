//! Stacked-PR workflow, ported command-for-command from the VisualJJ
//! extension's `jj branching` commands (reverse engineered from its bundled
//! `jj` 0.46 fork, driven against a mock GitHub):
//!
//! - **Create request** — `jj branching submit -c <change>`: push the change
//!   (`jj git push --change`, bookmark `push-<change id>`, i.e. jj's own
//!   `templates.git_push_bookmark`), open one PR against the trunk branch via
//!   GraphQL `createPullRequest` (title = first description line, empty body),
//!   then `jj new` if the submitted change was the working copy.
//! - **Update request** — `jj branching push --remote origin --bookmark B`:
//!   live-read the PRs the push could affect, refuse pushes GitHub would act
//!   on, `jj git push --bookmark B`, then park dependents whose base moved
//!   sideways.
//! - **Stack stage** (runs after both): retarget the PR onto the nearest
//!   ancestor with an open PR (REST `PATCH /pulls/{n}`), rebuild the GitHub
//!   native stack (`POST /stacks/{n}/unstack`, `POST /stacks`) and rewrite PR
//!   descriptions (`mutation updatePullRequest`) with the stack list between
//!   `VISUALJJ_STACK_V1` markers.
//!
//! The stages are off unless `visualjj.stacking-github-stacks` /
//! `visualjj.stacking-update-description` are set in the repo's jj config —
//! the same keys, with the same defaults, VisualJJ's settings write.

use std::collections::HashMap;

use anyhow::{bail, Result};

use crate::github::{GitHub, MergeOutcome, PrInfo, PushPrecheck};
use crate::jj::{Commit, Jj};

pub const STACK_START: &str = "<!-- VISUALJJ_STACK_V1 START -->";
pub const STACK_END: &str = "<!-- VISUALJJ_STACK_V1 END -->";
/// VisualJJ's "GitHub native stacked PRs" setting (jj config key).
pub const CFG_NATIVE_STACK: &str = "visualjj.stacking-github-stacks";
/// VisualJJ's "Stack info in PR descriptions" setting (jj config key).
pub const CFG_STACK_INFO: &str = "visualjj.stacking-update-description";

#[derive(Clone, Debug)]
pub struct SubmitOptions {
    pub remote: String,
    pub native_stack: bool,
    pub stack_info: bool,
    pub draft: bool,
}

impl SubmitOptions {
    /// `native_stack`/`stack_info` fall back to the repo's jj config, so
    /// jj-helper and the VisualJJ extension agree on the same repository.
    pub fn resolve(
        jj: &Jj,
        remote: String,
        native_stack: Option<bool>,
        stack_info: Option<bool>,
        draft: bool,
    ) -> SubmitOptions {
        SubmitOptions {
            native_stack: native_stack
                .or_else(|| jj.config_bool(CFG_NATIVE_STACK))
                .unwrap_or(false),
            stack_info: stack_info
                .or_else(|| jj.config_bool(CFG_STACK_INFO))
                .unwrap_or(false),
            remote,
            draft,
        }
    }
}

/// What submitting one change does.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// `jj branching submit`: create a PR on the trunk branch.
    Create {
        /// `--new-bookmark`: an exact name instead of `push-<change id>`.
        new_bookmark: Option<String>,
        /// `--title`: defaults to the description's first line.
        title: String,
    },
    /// `jj branching push --bookmark`: push updates to PR `number`.
    Update { number: u64 },
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub commit: Commit,
    /// Bookmark carrying the PR (created by the push for `Create`).
    pub bookmark: String,
    pub action: Action,
}

/// One VisualJJ submit/push per entry, bottom (closest to trunk) first.
#[derive(Clone, Debug)]
pub struct Plan {
    pub remote: String,
    pub trunk_branch: String,
    pub entries: Vec<Entry>,
}

impl Plan {
    pub fn describe(&self) -> Vec<String> {
        let mut lines = vec![];
        for (i, e) in self.entries.iter().enumerate().rev() {
            let what = match &e.action {
                Action::Create { title, .. } => {
                    format!("create PR {} → {}: {title}", e.bookmark, self.trunk_branch)
                }
                Action::Update { number } => {
                    format!("push updates to PR #{number} ({})", e.bookmark)
                }
            };
            lines.push(format!("{}. {} — {what}", i + 1, e.commit.short_change));
        }
        lines
    }

    /// The commands this plan runs, in order (GitHub calls summarized).
    pub fn commands(&self) -> Vec<String> {
        let mut out = vec![];
        for e in &self.entries {
            match &e.action {
                Action::Create { new_bookmark, .. } => {
                    let mut c = format!(
                        "jj git push --remote {} --change {}",
                        self.remote, e.commit.change_id
                    );
                    if let Some(n) = new_bookmark {
                        c.push_str(&format!(
                            " --config 'templates.git_push_bookmark=\"\\\"{n}\\\"\"'"
                        ));
                    }
                    out.push(c);
                    out.push(format!(
                        "GitHub: createPullRequest(base: {}, head: {})",
                        self.trunk_branch, e.bookmark
                    ));
                    if e.commit.is_wc {
                        out.push(format!("jj new {}", e.commit.change_id));
                    }
                }
                Action::Update { number } => {
                    out.push(format!(
                        "GitHub: live read of PR #{number} and its dependents"
                    ));
                    out.push(format!(
                        "jj git push --remote {} --bookmark {}",
                        self.remote, e.bookmark
                    ));
                }
            }
        }
        out
    }

    pub fn note_for(&self, e: &Entry) -> String {
        match &e.action {
            Action::Create { .. } => format!("new PR {}", e.bookmark),
            Action::Update { number } => format!("#{number} update"),
        }
    }
}

/// jj's push refuses undescribed and conflicted commits, so VisualJJ's
/// submit fails on them too; fail early with a pointer to the fix.
pub fn check_submittable(commit: &Commit) -> Result<()> {
    if commit.title().is_empty() {
        bail!(
            "{} has no description; describe it first (d)",
            commit.short_change
        );
    }
    if commit.conflict {
        bail!(
            "{} has conflicts; resolve them before submitting",
            commit.short_change
        );
    }
    Ok(())
}

/// `push-<change id>`: jj's own `templates.git_push_bookmark` default, which
/// `jj git push --change` (and so VisualJJ's submit) uses.
pub fn default_bookmark(change_id: &str) -> String {
    format!("push-{}", &change_id[..change_id.len().min(12)])
}

/// Picks the bookmark and action for each change. `prs` maps branch name →
/// latest PR. A change whose bookmark carries an open PR gets its updates
/// pushed (`branching push`); otherwise a PR is created (`branching submit`).
pub fn plan(
    commits: Vec<Commit>,
    prs: &HashMap<String, PrInfo>,
    trunk_branch: &str,
    remote: &str,
) -> Result<Plan> {
    let mut entries = vec![];
    for c in commits {
        check_submittable(&c)?;
        let with_pr = c
            .bookmarks
            .iter()
            .filter(|b| b.as_str() != trunk_branch)
            .find(|b| prs.get(b.as_str()).is_some_and(|p| p.is_open()));
        let (bookmark, action) = match with_pr {
            Some(b) => (
                b.clone(),
                Action::Update {
                    number: prs[b].number,
                },
            ),
            None => (
                default_bookmark(&c.change_id),
                Action::Create {
                    new_bookmark: None,
                    title: c.title().to_string(),
                },
            ),
        };
        entries.push(Entry {
            commit: c,
            bookmark,
            action,
        });
    }
    Ok(Plan {
        remote: remote.to_string(),
        trunk_branch: trunk_branch.to_string(),
        entries,
    })
}

/// The open-PR universe the stack stages reason about (VisualJJ keeps the
/// same data in `.jj/repo/reviews.json`): head branches from the open
/// list, bases, head oids, bodies and node ids from the live read.
pub struct Universe {
    prs: Vec<PrInfo>,
}

impl Universe {
    fn load(gh: &GitHub) -> Result<Universe> {
        let open = gh.open_prs()?;
        let heads: HashMap<u64, String> = open
            .iter()
            .map(|p| (p.number, p.head_ref.clone()))
            .collect();
        let numbers: Vec<u64> = open.iter().map(|p| p.number).collect();
        let mut prs = gh.live_prs(&numbers)?;
        for pr in &mut prs {
            if let Some(h) = heads.get(&pr.number) {
                pr.head_ref = h.clone();
            }
        }
        Ok(Universe { prs })
    }

    fn get(&self, number: u64) -> Option<&PrInfo> {
        self.prs.iter().find(|p| p.number == number)
    }

    fn by_head(&self, branch: &str) -> Option<&PrInfo> {
        self.prs.iter().find(|p| p.head_ref == branch)
    }

    fn set_base(&mut self, number: u64, base: &str) {
        if let Some(p) = self.prs.iter_mut().find(|p| p.number == number) {
            p.base_ref = base.to_string();
        }
    }

    /// The PR's chain bottom→top: everything its base chain leads down to
    /// (until the trunk branch), then itself.
    fn chain(&self, number: u64, trunk: &str) -> Vec<u64> {
        let mut chain = vec![number];
        let mut branch = match self.get(number) {
            Some(p) => p.base_ref.clone(),
            None => return chain,
        };
        let mut guard = 0;
        while branch != trunk && guard < 100 {
            guard += 1;
            match self.by_head(&branch) {
                Some(p) if !chain.contains(&p.number) => {
                    chain.push(p.number);
                    branch = p.base_ref.clone();
                }
                _ => break,
            }
        }
        chain.reverse();
        chain
    }

    /// Open PRs based (transitively) on `branch`, ascending by number.
    fn dependents(&self, branch: &str) -> Vec<u64> {
        let mut out = vec![];
        let mut frontier = vec![branch.to_string()];
        while let Some(b) = frontier.pop() {
            if out.len() > 200 {
                break;
            }
            for p in &self.prs {
                if p.base_ref == b && !out.contains(&p.number) {
                    out.push(p.number);
                    frontier.push(p.head_ref.clone());
                }
            }
        }
        out.sort_unstable();
        out
    }
}

/// Runs a submit plan, one VisualJJ submit/push per entry, bottom first.
/// Returns one line per PR (bottom first).
pub fn execute(
    jj: &Jj,
    gh: &GitHub,
    plan: &Plan,
    opts: &SubmitOptions,
    log: &mut dyn FnMut(String),
) -> Result<Vec<String>> {
    let mut out = vec![];
    for e in &plan.entries {
        match &e.action {
            Action::Create {
                new_bookmark,
                title,
            } => {
                let pr = submit_change(
                    jj,
                    gh,
                    e,
                    new_bookmark.as_deref(),
                    title,
                    &plan.trunk_branch,
                    opts,
                    log,
                )?;
                out.push(format!("#{} {}", pr.number, pr.url));
                if opts.native_stack || opts.stack_info {
                    if let Err(err) =
                        submit_stack_stage(jj, gh, &pr, e, &plan.trunk_branch, opts, log)
                    {
                        log(format!("⚠ Stack stage not completed: {err}"));
                    }
                }
            }
            Action::Update { number } => {
                update_request(jj, gh, e, &plan.trunk_branch, opts, log)?;
                out.push(format!("#{number} updated"));
            }
        }
    }
    Ok(out)
}

/// `jj branching submit -c <change>` up to the PR: push, create, `jj new`.
#[allow(clippy::too_many_arguments)]
fn submit_change(
    jj: &Jj,
    gh: &GitHub,
    e: &Entry,
    new_bookmark: Option<&str>,
    title: &str,
    trunk: &str,
    opts: &SubmitOptions,
    log: &mut dyn FnMut(String),
) -> Result<PrInfo> {
    // 1. `jj git push --change`: creates the bookmark and pushes it in one
    //    operation (rolled back together if jj refuses the push).
    let mut push = vec![
        "git".to_string(),
        "push".into(),
        "--remote".into(),
        opts.remote.clone(),
        "--change".into(),
        e.commit.change_id.clone(),
    ];
    if let Some(name) = new_bookmark {
        // A TOML string holding a jj template string literal.
        let literal = name.replace('\\', "\\\\").replace('"', "\\\"");
        push.push("--config".into());
        push.push(format!("templates.git_push_bookmark=\"\\\"{literal}\\\"\""));
    }
    log(format!("jj {}", crate::jj::shell_join(&push)));
    let out = jj.run(&push)?;
    for l in out.summary().lines() {
        log(format!("  {l}"));
    }

    // 2. One PR against the trunk branch (GraphQL, like VisualJJ); the
    //    stack stage retargets it afterwards.
    let repo_id = gh.repository_id()?;
    let pr = gh.create_pull_request(&repo_id, trunk, &e.bookmark, title, "", opts.draft)?;
    log(format!("Pull request created: {}", pr.url));

    // 3. VisualJJ never leaves the working copy on a submitted change.
    if e.commit.is_wc {
        log("Creating new empty commit on top of submitted change".into());
        jj.run(&["new", &e.commit.change_id])?;
    }
    Ok(pr)
}

/// The stack stage after a submit: retarget the new PR onto the nearest
/// ancestor with an open PR, rebuild the native stack, write descriptions.
fn submit_stack_stage(
    jj: &Jj,
    gh: &GitHub,
    pr: &PrInfo,
    e: &Entry,
    trunk: &str,
    opts: &SubmitOptions,
    log: &mut dyn FnMut(String),
) -> Result<()> {
    let mut uni = Universe::load(gh)?;
    if uni.get(pr.number).is_none() {
        bail!("PR #{} is not in the live open set yet; retry", pr.number);
    }
    let desired = nearest_ancestor_pr_base(jj, &uni, &e.commit, trunk)?;
    let current = uni
        .get(pr.number)
        .map(|p| p.base_ref.clone())
        .unwrap_or_default();
    let mut retargets = vec![];
    if desired != current && desired != e.bookmark {
        uni.set_base(pr.number, &desired);
        retargets.push(Retarget {
            number: pr.number,
            from: current,
            to: desired,
        });
    }
    reconcile(jj, gh, &uni, &retargets, &[pr.number], trunk, opts, log)
}

/// `jj branching push --remote origin --bookmark <branch>`: the pre-push
/// judgement, the push, then parking + stack maintenance.
fn update_request(
    jj: &Jj,
    gh: &GitHub,
    e: &Entry,
    trunk: &str,
    opts: &SubmitOptions,
    log: &mut dyn FnMut(String),
) -> Result<()> {
    let branch = &e.bookmark;
    let pre = gh.push_precheck(branch, trunk)?;
    push_safety(jj, &pre, &e.commit.commit_id, trunk, opts)?;

    let push = [
        "git",
        "push",
        "--remote",
        &opts.remote,
        "--bookmark",
        branch,
    ];
    log(format!("jj {}", crate::jj::shell_join(&push)));
    let out = jj.run(&push)?;
    let summary = out.summary();
    for l in summary.lines() {
        log(format!("  {l}"));
    }
    if summary.contains("Nothing changed.") || !(opts.native_stack || opts.stack_info) {
        return Ok(());
    }

    // Post-push: park dependents whose base moved sideways, then
    // reconcile stacks and descriptions for everything affected.
    let mut uni = Universe::load(gh)?;
    let Some(pushed) = uni.by_head(branch).map(|p| p.number) else {
        return Ok(());
    };
    let mut affected = vec![pushed];
    let mut retargets = vec![];
    let new_tip = &e.commit.commit_id;
    for dep in uni.dependents(branch) {
        let Some(pr) = uni.get(dep).cloned() else {
            continue;
        };
        if pr.base_ref != *branch {
            // Transitive dependents follow their own parent.
            affected.push(dep);
            continue;
        }
        if is_ancestor(jj, new_tip, &pr.head_oid) {
            affected.push(dep);
            continue;
        }
        // The dependent is no longer above the moved branch: park it on
        // the deepest base down the chain that still contains its head.
        let chain = uni.chain(pushed, trunk);
        let candidates: Vec<String> = chain
            .iter()
            .rev()
            .skip(1)
            .filter_map(|n| uni.get(*n))
            .map(|p| p.head_ref.clone())
            .chain(std::iter::once(trunk.to_string()))
            .collect();
        let mut parked = None;
        for base in &candidates {
            let tip = remote_tip(jj, base, &opts.remote);
            match tip {
                Some(tip) if is_ancestor(jj, &tip, &pr.head_oid) => {
                    if base == trunk && is_ancestor(jj, &pr.head_oid, &tip) {
                        bail!(
                            "PR #{}'s head is already contained in trunk; parking it there would merge it",
                            pr.number
                        );
                    }
                    parked = Some(base.clone());
                    break;
                }
                _ => {}
            }
        }
        match parked {
            Some(base) => {
                if base != pr.base_ref {
                    uni.set_base(pr.number, &base);
                    retargets.push(Retarget {
                        number: pr.number,
                        from: pr.base_ref.clone(),
                        to: base,
                    });
                }
                affected.push(dep);
            }
            None => log(format!(
                "⚠ PR #{} is no longer above {branch} and no base below contains it; retarget it on GitHub",
                pr.number
            )),
        }
    }
    reconcile(jj, gh, &uni, &retargets, &affected, trunk, opts, log)
}

/// A base change to apply (after dissolving stacks: GitHub won't retarget
/// a PR locked in one).
struct Retarget {
    number: u64,
    from: String,
    to: String,
}

/// "Refusing to push: GitHub would act on open PRs." — the push would make
/// a PR's head contained in its base branch (GitHub merges it). With native
/// stacks on, a trunk that moved on GitHub since the last fetch is refused
/// too, as VisualJJ does.
fn push_safety(
    jj: &Jj,
    pre: &PushPrecheck,
    new_tip: &str,
    trunk: &str,
    opts: &SubmitOptions,
) -> Result<()> {
    let remote = &opts.remote;
    if opts.native_stack {
        match (&pre.trunk_oid, remote_tip(jj, trunk, remote)) {
            (Some(remote_oid), Some(local)) if *remote_oid != local => {
                bail!("The trunk branch moved on GitHub since the last fetch; fetch (f) and retry")
            }
            (None, _) => bail!(
                "The trunk branch's position on GitHub could not be read; check that {trunk} still exists there and retry"
            ),
            _ => {}
        }
    }
    for pr in &pre.head_prs {
        if let Some(base_tip) = remote_tip(jj, &pr.base_ref, remote) {
            if is_ancestor(jj, new_tip, &base_tip) {
                bail!(
                    "Refusing to push: GitHub would act on open PRs. PR #{} would be merged by GitHub: its head would become contained in its base branch {}",
                    pr.number,
                    pr.base_ref
                );
            }
        }
    }
    for pr in &pre.base_prs {
        if !pr.head_oid.is_empty() && is_ancestor(jj, &pr.head_oid, new_tip) {
            bail!(
                "Refusing to push: GitHub would act on open PRs. PR #{} would be merged by GitHub: its head would become contained in its base branch",
                pr.number
            );
        }
    }
    Ok(())
}

/// The pre-push judgement of VisualJJ's plain Push (`branching push
/// --all`), which only runs while native stacks are on: refuse a push that
/// would close an open PR (deleting its branch) or merge one (its head
/// becoming contained in its base).
pub fn push_all_safety(
    jj: &Jj,
    gh: &GitHub,
    remote: &str,
    log: &mut dyn FnMut(String),
) -> Result<()> {
    let open = gh
        .open_prs()
        .map_err(|e| anyhow::anyhow!("Failed to read live PR state before push: {e}. Retry once GitHub is reachable, or turn off native stacks to push without this check"))?;
    let numbers: Vec<u64> = open.iter().map(|p| p.number).collect();
    let live = gh
        .live_prs(&numbers)
        .map_err(|e| anyhow::anyhow!("Failed to read live PR state before push: {e}"))?;
    for mut pr in live {
        let Some(branch) = open
            .iter()
            .find(|p| p.number == pr.number)
            .map(|p| p.head_ref.clone())
        else {
            continue;
        };
        pr.head_ref = branch.clone();
        let local = jj.exists(&format!("\"{branch}\""));
        let on_remote = jj.exists(&format!("\"{branch}\"@{remote}"));
        if on_remote && !local {
            bail!(
                "Refusing to push: GitHub would act on open PRs. PR #{} would be closed by GitHub: the push deletes a branch it uses",
                pr.number
            );
        }
        if let (true, Some(base_tip)) = (local, remote_tip(jj, &pr.base_ref, remote)) {
            let tip = jj
                .commit_ids(&format!("present(\"{branch}\")"))
                .ok()
                .and_then(|v| v.first().cloned());
            if let Some(tip) = tip {
                if is_ancestor(jj, &tip, &base_tip) {
                    bail!(
                        "Refusing to push: GitHub would act on open PRs. PR #{} would be merged by GitHub: its head would become contained in its base branch {}",
                        pr.number,
                        pr.base_ref
                    );
                }
            }
        }
        log(format!("open PR #{} on {branch} unaffected", pr.number));
    }
    Ok(())
}

/// The GitHub side of the stack stage, in VisualJJ's order: dissolve stacks
/// that no longer match (GitHub refuses to retarget a stacked PR), apply
/// the retargets, create the stacks, then rewrite descriptions.
#[allow(clippy::too_many_arguments)]
fn reconcile(
    jj: &Jj,
    gh: &GitHub,
    uni: &Universe,
    retargets: &[Retarget],
    affected: &[u64],
    trunk: &str,
    opts: &SubmitOptions,
    log: &mut dyn FnMut(String),
) -> Result<()> {
    // Desired chains (bases already updated in `uni`).
    let mut chains: Vec<Vec<u64>> = vec![];
    for n in affected {
        let c = uni.chain(*n, trunk);
        if !chains.contains(&c) {
            chains.push(c);
        }
    }

    let mut matched: Vec<Vec<u64>> = vec![];
    if opts.native_stack {
        let mut seen = vec![];
        for chain in &chains {
            for n in chain {
                let Some(s) = uni.get(*n).and_then(|p| p.stack.as_ref()) else {
                    continue;
                };
                if seen.contains(&s.number) {
                    continue;
                }
                seen.push(s.number);
                let current = match gh.get_stack(s.number) {
                    Ok(st) => st,
                    Err(err) => {
                        log(format!("warning: couldn't read stack #{}: {err}", s.number));
                        continue;
                    }
                };
                if chains.contains(&current.open_numbers()) {
                    matched.push(current.open_numbers());
                    continue;
                }
                log(format!("✓ Dissolved stack #{}", current.number));
                gh.unstack(current.number)?;
            }
        }
    }

    for r in retargets {
        log(format!(
            "✓ Retargeted PR #{}: {} -> {}",
            r.number, r.from, r.to
        ));
        gh.set_pr_base(r.number, &r.to)?;
    }

    if opts.native_stack {
        // A native stack is one linear chain; once the affected PRs fork
        // (a dependent got parked), VisualJJ leaves them unstacked.
        if let [chain] = chains.as_slice() {
            if chain.len() > 1 && !matched.contains(chain) {
                log(format!("✓ Created stack over {chain:?}"));
                gh.create_stack(chain)?;
            }
        } else {
            log("Stack not updated: the PRs no longer form one linear stack".into());
        }
    }

    if opts.stack_info {
        let mut done = vec![];
        for chain in &chains {
            for n in chain {
                if done.contains(n) {
                    continue;
                }
                done.push(*n);
                update_description(jj, gh, uni, *n, trunk, log)?;
            }
        }
    }
    Ok(())
}

/// Head branch of the nearest ancestor change with an open PR, else trunk.
fn nearest_ancestor_pr_base(
    jj: &Jj,
    uni: &Universe,
    commit: &Commit,
    trunk: &str,
) -> Result<String> {
    let ancestors = jj.log_commits(&format!("::({}-) ~ ::trunk()", commit.commit_id))?;
    for c in ancestors {
        if let Some(b) = c
            .bookmarks
            .iter()
            .filter(|b| b.as_str() != trunk)
            .find(|b| uni.by_head(b).is_some())
        {
            return Ok(b.clone());
        }
    }
    Ok(trunk.to_string())
}

/// `x` is an ancestor of (or equal to) `y`, judged by the local repo.
fn is_ancestor(jj: &Jj, x: &str, y: &str) -> bool {
    jj.exists(&format!("({x}) & ::({y})"))
}

/// Commit the remote branch points at, if we have it locally.
fn remote_tip(jj: &Jj, branch: &str, remote: &str) -> Option<String> {
    jj.commit_ids(&format!("present(\"{branch}\"@{remote})"))
        .ok()
        .and_then(|v| v.first().cloned())
}

/// VisualJJ's "Updating stack info for PR #n" step for one PR.
fn update_description(
    jj: &Jj,
    gh: &GitHub,
    uni: &Universe,
    number: u64,
    trunk: &str,
    log: &mut dyn FnMut(String),
) -> Result<()> {
    let Some(pr) = uni.get(number) else {
        return Ok(());
    };
    log(format!("Updating stack info for PR #{number}"));
    let (body, pos, total) = stack_body(jj, uni, number, trunk);
    if total < 2 {
        log(format!(
            "PR #{number} is not part of a stack; nothing to update"
        ));
        return Ok(());
    }
    log(format!("Stack position: {pos} of {total}"));
    if body == pr.body {
        log(format!("✓ PR #{number} already up to date (skipped)"));
        return Ok(());
    }
    gh.update_pull_request_body(&pr.node_id, &body)?;
    log(format!("✓ Updated PR #{number} description"));
    Ok(())
}

/// The description VisualJJ writes, verbatim, and the PR's position:
///
/// ```text
/// <!-- VISUALJJ_STACK_V1 START -->
/// Stack of PRs (i of N):
/// - #top
/// - ▶ #this [(k commits)](this/files) ◀
/// - #bottom
/// <!-- VISUALJJ_STACK_V1 END -->
///
/// <the PR's own description, if any>
/// ```
///
/// The list is the PR's chain to trunk plus every open PR above it, top
/// first.
pub fn stack_body(jj: &Jj, uni: &Universe, this: u64, trunk: &str) -> (String, usize, usize) {
    let chain = uni.chain(this, trunk);
    let mut members = chain.clone();
    if let Some(head) = uni.get(this).map(|p| p.head_ref.clone()) {
        for dep in uni.dependents(&head) {
            if !members.contains(&dep) {
                members.push(dep);
            }
        }
    }
    let pos = chain.len();
    let total = members.len();
    let mut list = format!("Stack of PRs ({pos} of {total}):\n");
    for n in members.iter().rev() {
        if *n == this {
            let k = commit_count(jj, uni, &chain, this);
            list.push_str(&format!(
                "- ▶ #{n} [({k} commit{})]({n}/files) ◀\n",
                if k == 1 { "" } else { "s" }
            ));
        } else {
            list.push_str(&format!("- #{n}\n"));
        }
    }
    let block = format!("{STACK_START}\n{list}{STACK_END}");
    let own = strip_stack_info(uni.get(this).map(|p| p.body.as_str()).unwrap_or(""));
    let body = if own.is_empty() {
        block
    } else {
        format!("{block}\n\n{own}")
    };
    (body, pos, total)
}

/// Removes a previous stack block from a PR body.
pub fn strip_stack_info(body: &str) -> String {
    let stripped = match (body.find(STACK_START), body.find(STACK_END)) {
        (Some(s), Some(e)) if e > s => format!("{}{}", &body[..s], &body[e + STACK_END.len()..]),
        _ => body.to_string(),
    };
    stripped.trim().to_string()
}

/// Commits the PR contributes over the PR below it (or trunk).
fn commit_count(jj: &Jj, uni: &Universe, chain: &[u64], this: u64) -> usize {
    let Some(head) = uni
        .get(this)
        .map(|p| p.head_oid.clone())
        .filter(|h| !h.is_empty())
    else {
        return 1;
    };
    let below = chain
        .iter()
        .position(|n| *n == this)
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| uni.get(chain[i]))
        .map(|p| p.head_oid.clone());
    let revset = match below {
        Some(b) => format!("::({head}) ~ ::({b})"),
        None => format!("::({head}) ~ ::trunk()"),
    };
    jj.commit_ids(&revset).map(|v| v.len().max(1)).unwrap_or(1)
}

/// `jj branching ship-pr`: merge through the async merge API (and so
/// everything below it in a native stack), then fetch.
pub fn ship(
    jj: &Jj,
    gh: &GitHub,
    pr: &PrInfo,
    commit: &Commit,
    method: &str,
    remote: &str,
    log: &mut dyn FnMut(String),
) -> Result<String> {
    if !pr.is_open() {
        bail!("PR #{} is not open", pr.number);
    }
    if pr.head_oid != commit.commit_id {
        bail!(
            "local {} differs from PR #{}'s head; push it first (P) so you ship what you see",
            commit.short_change,
            pr.number
        );
    }
    log(format!(
        "merging PR #{} ({method}) via merge-async",
        pr.number
    ));
    let outcome = gh.merge_async(pr.number, method, &pr.head_oid, log)?;
    let msg = match outcome {
        MergeOutcome::Merged(sha) => format!(
            "Pull request {} merged{}",
            pr.number,
            sha.map(|s| format!(" as {}", &s[..s.len().min(10)]))
                .unwrap_or_default()
        ),
        MergeOutcome::Enqueued => format!(
            "PR #{} queued for merge; it will show as merged once GitHub is done",
            pr.number
        ),
    };
    log(msg.clone());
    jj.run(&["git", "fetch", "--remote", remote])?;
    Ok(msg)
}

/// Condenses `jj git fetch` output (which interleaves git's progress lines)
/// to the bookmark updates.
pub fn fetch_summary(out: &str) -> String {
    let updates: Vec<&str> = out
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("bookmark: "))
        .collect();
    match updates.len() {
        0 => "fetched; nothing new".into(),
        1..=3 => format!("fetched: {}", updates.join(", ")),
        n => format!("fetched: {n} bookmarks updated"),
    }
}

/// Revset of local stacks sitting on an outdated trunk.
pub const STALE_ROOTS: &str =
    "roots((mutable() & mine()) ~ ::trunk()) & children(::trunk() ~ trunk())";

/// Fetch, then (optionally) rebase the user's stacks onto the new trunk —
/// undone automatically if it would introduce conflicts.
pub fn fetch_and_rebase(
    jj: &Jj,
    remote: &str,
    rebase: bool,
    log: &mut dyn FnMut(String),
) -> Result<String> {
    let out = jj.run(&["git", "fetch", "--remote", remote])?;
    let msg = fetch_summary(&out.summary());
    if !rebase {
        return Ok(msg);
    }
    let roots = jj.commit_ids(STALE_ROOTS)?;
    if roots.is_empty() {
        return Ok(format!("{msg}; stacks already on trunk"));
    }
    let count_conflicts = || {
        jj.commit_ids("conflicts() & mutable()")
            .map(|v| v.len())
            .unwrap_or(0)
    };
    let before = count_conflicts();
    let set = roots.join(" | ");
    log(format!("rebasing {} stack(s) onto trunk", roots.len()));
    jj.run(&["rebase", "-s", &set, "-o", "trunk()"])?;
    let after = count_conflicts();
    if after > before {
        jj.run(&["undo"])?;
        return Ok(format!(
            "{msg}; auto-rebase skipped (would add {} conflict(s))",
            after - before
        ));
    }
    Ok(format!(
        "{msg}; rebased {} stack(s) onto trunk",
        roots.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::PrState;

    fn commit(id: &str, bookmarks: &[&str]) -> Commit {
        let mut c = Commit::synthetic(id, &format!("title {id}\n\nbody {id}"));
        c.change_id = format!("{id}{}", "z".repeat(32 - id.len()));
        c.bookmarks = bookmarks.iter().map(|s| s.to_string()).collect();
        c
    }

    fn pr(number: u64, head: &str, base: &str) -> PrInfo {
        PrInfo {
            number,
            node_id: format!("node{number}"),
            title: format!("PR {number}"),
            url: format!("https://github.com/o/r/pull/{number}"),
            body: String::new(),
            state: PrState::Open,
            draft: false,
            head_ref: head.into(),
            base_ref: base.into(),
            // Not a real commit: commit_count falls back to 1.
            head_oid: String::new(),
            review_decision: None,
            reviews: vec![],
            checks: None,
            signals: vec![],
            merge_queue: None,
            stack: None,
        }
    }

    #[test]
    fn plans_update_for_open_prs_and_create_otherwise() {
        let prs = HashMap::from([
            ("feat-a".to_string(), pr(5, "feat-a", "main")),
            ("feat-b".to_string(), pr(6, "feat-b", "main")),
        ]);
        let p = plan(
            vec![
                commit("a", &["feat-a"]),
                commit("b", &["feat-b", "other"]),
                commit("c", &["unrelated"]),
            ],
            &prs,
            "main",
            "origin",
        )
        .unwrap();
        assert_eq!(p.entries[0].action, Action::Update { number: 5 });
        assert_eq!(p.entries[0].bookmark, "feat-a");
        assert_eq!(p.entries[1].action, Action::Update { number: 6 });
        // No PR on any bookmark → VisualJJ creates push-<change id>, never
        // reuses an unrelated bookmark.
        assert_eq!(p.entries[2].bookmark, "push-czzzzzzzzzzz");
        assert_eq!(
            p.entries[2].action,
            Action::Create {
                new_bookmark: None,
                title: "title c".into()
            }
        );
        assert_eq!(
            p.commands()[0],
            "GitHub: live read of PR #5 and its dependents"
        );
        assert!(p.commands().contains(
            &"jj git push --remote origin --change czzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".to_string()
        ));
    }

    #[test]
    fn refuses_undescribed_commits() {
        let c = Commit::synthetic("x", "");
        assert!(plan(vec![c], &HashMap::new(), "main", "origin").is_err());
    }

    #[test]
    fn default_bookmark_matches_jj_push_change() {
        assert_eq!(
            default_bookmark("pmrxztoxxsswuqlztvzylpsrwsoukxtq"),
            "push-pmrxztoxxssw"
        );
    }

    #[test]
    fn chain_and_dependents_follow_base_refs() {
        let uni = Universe {
            prs: vec![
                pr(1, "push-a", "main"),
                pr(2, "push-b", "push-a"),
                pr(3, "push-c", "push-b"),
                pr(4, "other", "main"),
                pr(5, "push-d", "push-a"),
            ],
        };
        assert_eq!(uni.chain(3, "main"), vec![1, 2, 3]);
        assert_eq!(uni.chain(1, "main"), vec![1]);
        assert_eq!(uni.chain(5, "main"), vec![1, 5]);
        assert_eq!(uni.dependents("push-a"), vec![2, 3, 5]);
        assert_eq!(uni.dependents("push-b"), vec![3]);
    }

    #[test]
    fn stack_body_matches_visualjj_format() {
        let jj = Jj::new(None);
        let uni = Universe {
            prs: vec![
                pr(100, "push-a", "main"),
                pr(101, "push-b", "push-a"),
                pr(102, "push-c", "push-a"),
            ],
        };
        // Bottom PR lists everything above it (observed: "1 of 3").
        let (body, pos, total) = stack_body(&jj, &uni, 100, "main");
        assert_eq!((pos, total), (1, 3));
        assert_eq!(
            body,
            "<!-- VISUALJJ_STACK_V1 START -->\nStack of PRs (1 of 3):\n- #102\n- #101\n- ▶ #100 [(1 commit)](100/files) ◀\n<!-- VISUALJJ_STACK_V1 END -->"
        );
        // A forked sibling sees only its own chain (observed: "2 of 2").
        let (body, pos, total) = stack_body(&jj, &uni, 102, "main");
        assert_eq!((pos, total), (2, 2));
        assert!(
            body.contains("Stack of PRs (2 of 2):\n- ▶ #102 [(1 commit)](102/files) ◀\n- #100\n")
        );
    }

    #[test]
    fn stack_body_prepends_block_to_own_description() {
        let jj = Jj::new(None);
        let mut b = pr(101, "push-b", "push-a");
        b.body = format!("{STACK_START}\nold\n{STACK_END}\n\nMy own description.\n\nMore.");
        let uni = Universe {
            prs: vec![pr(100, "push-a", "main"), b],
        };
        let (body, _, _) = stack_body(&jj, &uni, 101, "main");
        assert_eq!(
            body,
            "<!-- VISUALJJ_STACK_V1 START -->\nStack of PRs (2 of 2):\n- ▶ #101 [(1 commit)](101/files) ◀\n- #100\n<!-- VISUALJJ_STACK_V1 END -->\n\nMy own description.\n\nMore."
        );
        assert_eq!(strip_stack_info(&body), "My own description.\n\nMore.");
    }

    #[test]
    fn summarizes_fetch() {
        let out = "git: From x\ngit:    a..b  main -> origin/main\nbookmark: main@origin [updated] tracked";
        assert_eq!(fetch_summary(out), "fetched: main@origin [updated] tracked");
        assert_eq!(fetch_summary("Nothing changed."), "fetched; nothing new");
    }
}
