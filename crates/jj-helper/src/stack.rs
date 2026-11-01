//! Stacked-PR workflow: one PR per commit between trunk and the selected
//! commit, each based on the PR below it, linked as a GitHub native stack
//! and annotated with a stack footer in the description.

use std::collections::HashMap;

use anyhow::{bail, Result};
use serde_json::json;

use crate::github::{GitHub, MergeOutcome, PrInfo};
use crate::jj::{Commit, Jj};

pub const FOOTER_START: &str = "<!-- jj-helper:stack -->";
pub const FOOTER_END: &str = "<!-- /jj-helper:stack -->";

#[derive(Clone, Debug)]
pub struct SubmitOptions {
    pub remote: String,
    pub native_stack: bool,
    pub footer: bool,
    pub draft: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrAction {
    Create {
        base: String,
    },
    Retarget {
        number: u64,
        from: String,
        to: String,
    },
    Keep {
        number: u64,
    },
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub commit: Commit,
    pub bookmark: String,
    /// Bookmark must be created or moved to this commit.
    pub set_bookmark: bool,
    pub pr: Option<PrInfo>,
    pub action: PrAction,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub trunk_branch: String,
    /// Bottom (closest to trunk) first.
    pub entries: Vec<Entry>,
}

impl Plan {
    pub fn describe(&self, native_stack: bool) -> Vec<String> {
        let mut lines = vec![format!(
            "{} PR(s) on top of {}:",
            self.entries.len(),
            self.trunk_branch
        )];
        for (i, e) in self.entries.iter().enumerate().rev() {
            let what = match &e.action {
                PrAction::Create { base } => format!("create PR {} → {base}", e.bookmark),
                PrAction::Retarget { number, from, to } => {
                    format!("PR #{number}: retarget {from} → {to}, push")
                }
                PrAction::Keep { number } => format!("PR #{number}: push updates"),
            };
            lines.push(format!(
                "{}. {} {} — {what}",
                i + 1,
                e.commit.short_change,
                e.commit.title()
            ));
        }
        if native_stack && self.entries.len() > 1 {
            lines.push(format!(
                "Link {} PRs as a GitHub native stack",
                self.entries.len()
            ));
        }
        lines
    }

    pub fn note_for(&self, e: &Entry) -> String {
        match &e.action {
            PrAction::Create { base } => format!("new PR {} → {base}", e.bookmark),
            PrAction::Retarget { number, to, .. } => format!("#{number} retarget → {to}"),
            PrAction::Keep { number } => format!("#{number} update"),
        }
    }
}

/// Commits from trunk (exclusive) up to `rev`, bottom first; must be linear.
pub fn stack_commits(jj: &Jj, rev: &str) -> Result<Vec<Commit>> {
    let mut commits = jj.log_commits(&format!("trunk()..({rev})"))?;
    commits.reverse();
    // A trailing empty, undescribed working copy isn't part of the stack.
    while commits
        .last()
        .is_some_and(|c| c.empty && c.description.trim().is_empty())
    {
        commits.pop();
    }
    if commits.is_empty() {
        bail!("nothing between trunk and this commit to submit");
    }
    for (i, c) in commits.iter().enumerate() {
        if c.parents.len() != 1 {
            bail!(
                "{} is a merge commit; stacked PRs need a linear stack",
                c.short_change
            );
        }
        if i > 0 && c.parents[0] != commits[i - 1].commit_id {
            bail!(
                "the stack under {} isn't linear (multiple roots); rebase it into one line first",
                c.short_change
            );
        }
        if c.title().is_empty() {
            bail!(
                "{} has no description; describe it first (d)",
                c.short_change
            );
        }
        if c.conflict {
            bail!(
                "{} has conflicts; resolve them before submitting",
                c.short_change
            );
        }
    }
    Ok(commits)
}

pub fn default_bookmark(change_id: &str) -> String {
    format!("push-{}", &change_id[..change_id.len().min(12)])
}

/// Picks bookmarks and PR actions. `prs` maps branch name → latest PR.
pub fn plan(commits: Vec<Commit>, prs: &HashMap<String, PrInfo>, trunk_branch: &str) -> Plan {
    let mut entries: Vec<Entry> = vec![];
    for c in commits {
        let candidates: Vec<&String> = c
            .bookmarks
            .iter()
            .filter(|b| b.as_str() != trunk_branch)
            .collect();
        let with_pr = candidates
            .iter()
            .find(|b| prs.get(b.as_str()).is_some_and(|p| p.is_open()));
        let (bookmark, set_bookmark) = match (with_pr, candidates.first()) {
            (Some(b), _) => ((*b).clone(), false),
            (None, Some(b)) => ((*b).clone(), false),
            (None, None) => (default_bookmark(&c.change_id), true),
        };
        let base = entries
            .last()
            .map(|e| e.bookmark.clone())
            .unwrap_or_else(|| trunk_branch.to_string());
        let pr = prs.get(&bookmark).filter(|p| p.is_open()).cloned();
        let action = match &pr {
            Some(p) if p.base_ref != base => PrAction::Retarget {
                number: p.number,
                from: p.base_ref.clone(),
                to: base.clone(),
            },
            Some(p) => PrAction::Keep { number: p.number },
            None => PrAction::Create { base },
        };
        entries.push(Entry {
            commit: c,
            bookmark,
            set_bookmark,
            pr,
            action,
        });
    }
    Plan {
        trunk_branch: trunk_branch.to_string(),
        entries,
    }
}

/// Replaces (or with `numbers.len() < 2`, removes) the stack footer.
pub fn with_footer(body: &str, numbers: &[u64], this: u64) -> String {
    let stripped = match (body.find(FOOTER_START), body.find(FOOTER_END)) {
        (Some(s), Some(e)) if e > s => format!("{}{}", &body[..s], &body[e + FOOTER_END.len()..]),
        _ => body.to_string(),
    };
    let stripped = stripped.trim_end().to_string();
    if numbers.len() < 2 {
        return stripped;
    }
    let mut footer = format!(
        "{FOOTER_START}\n---\nStack ({} PRs, top first):\n",
        numbers.len()
    );
    for n in numbers.iter().rev() {
        if *n == this {
            footer.push_str(&format!("* ➡️ **#{n}** (this PR)\n"));
        } else {
            footer.push_str(&format!("* #{n}\n"));
        }
    }
    footer.push_str(FOOTER_END);
    if stripped.is_empty() {
        footer
    } else {
        format!("{stripped}\n\n{footer}")
    }
}

/// Runs a submit plan. Returns PR numbers bottom first.
pub fn execute(
    jj: &Jj,
    gh: &GitHub,
    plan: &Plan,
    opts: &SubmitOptions,
    log: &mut dyn FnMut(String),
) -> Result<Vec<u64>> {
    for e in plan.entries.iter().filter(|e| e.set_bookmark) {
        log(format!(
            "bookmark {} → {}",
            e.bookmark, e.commit.short_change
        ));
        jj.run(&[
            "bookmark",
            "set",
            &e.bookmark,
            "-r",
            &e.commit.commit_id,
            "--allow-backwards",
        ])?;
    }
    let mut push = vec![
        "git".to_string(),
        "push".into(),
        "--remote".into(),
        opts.remote.clone(),
    ];
    for e in &plan.entries {
        push.push("-b".into());
        push.push(e.bookmark.clone());
    }
    log(format!("jj {}", crate::jj::shell_join(&push)));
    let out = jj.run(&push)?;
    for l in out.summary().lines() {
        log(format!("  {l}"));
    }

    // GitHub won't retarget PRs locked in a stack; dissolve stacks whose
    // shape no longer matches before touching bases.
    let desired_existing: Vec<u64> = plan
        .entries
        .iter()
        .filter_map(|e| e.pr.as_ref().map(|p| p.number))
        .collect();
    let mut kept_stack: Option<(u64, Vec<u64>)> = None;
    if opts.native_stack {
        let mut seen = vec![];
        for e in &plan.entries {
            let Some(pr) = &e.pr else { continue };
            let Some(info) = &pr.stack else { continue };
            if seen.contains(&info.number) {
                continue;
            }
            seen.push(info.number);
            let stack = match gh.get_stack(info.number) {
                Ok(s) => s,
                Err(err) => {
                    log(format!(
                        "warning: couldn't read stack #{}: {err}",
                        info.number
                    ));
                    continue;
                }
            };
            let current = stack.open_numbers();
            let retargets = plan
                .entries
                .iter()
                .any(|e| matches!(e.action, PrAction::Retarget { number, .. } if current.contains(&number)));
            let prefix_ok = current.len() <= desired_existing.len()
                && desired_existing[..current.len()] == current[..];
            if prefix_ok && !retargets && kept_stack.is_none() {
                kept_stack = Some((stack.number, current));
            } else {
                log(format!(
                    "dissolving stack #{} ({current:?}) to restack",
                    stack.number
                ));
                if let Err(err) = gh.unstack(stack.number) {
                    log(format!("warning: unstack #{} failed: {err}", stack.number));
                }
            }
        }
    }

    let mut numbers = vec![];
    let mut bodies: HashMap<u64, String> = HashMap::new();
    for e in &plan.entries {
        match &e.action {
            PrAction::Keep { number } => {
                numbers.push(*number);
                bodies.insert(
                    *number,
                    e.pr.as_ref().map(|p| p.body.clone()).unwrap_or_default(),
                );
            }
            PrAction::Retarget { number, to, .. } => {
                log(format!("PR #{number}: base → {to}"));
                gh.update_pr(*number, json!({ "base": to }))?;
                numbers.push(*number);
                bodies.insert(
                    *number,
                    e.pr.as_ref().map(|p| p.body.clone()).unwrap_or_default(),
                );
            }
            PrAction::Create { base } => {
                let body = e.commit.body();
                let (n, url) =
                    gh.create_pr(&e.bookmark, base, e.commit.title(), &body, opts.draft)?;
                log(format!("created PR #{n} {url}"));
                numbers.push(n);
                bodies.insert(n, body);
            }
        }
    }

    if opts.native_stack && numbers.len() > 1 {
        let result = match &kept_stack {
            Some((_, current)) if *current == numbers => {
                log("native stack already up to date".into());
                Ok(())
            }
            Some((stack, current)) if numbers.starts_with(current) => {
                log(format!(
                    "adding {:?} to stack #{stack}",
                    &numbers[current.len()..]
                ));
                gh.add_to_stack(*stack, &numbers[current.len()..])
                    .map(|_| ())
            }
            other => {
                if let Some((stack, _)) = other {
                    let _ = gh.unstack(*stack);
                }
                log(format!("creating native stack {numbers:?}"));
                gh.create_stack(&numbers)
                    .map(|s| log(format!("stack #{} created", s.number)))
            }
        };
        if let Err(err) = result {
            log(format!("warning: native stack not updated: {err}"));
        }
    }

    if opts.footer {
        for n in &numbers {
            let body = bodies.get(n).cloned().unwrap_or_default();
            let new_body = with_footer(&body, &numbers, *n);
            if new_body.trim() != body.trim() {
                gh.update_pr(*n, json!({ "body": new_body }))?;
            }
        }
    }
    Ok(numbers)
}

/// Merges the PR on `commit` via the async merge API (merging everything
/// below it in a native stack), then fetches.
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
            "PR #{} merged{}",
            pr.number,
            sha.map(|s| format!(" as {}", &s[..s.len().min(10)]))
                .unwrap_or_default()
        ),
        MergeOutcome::Enqueued => format!("PR #{} added to the merge queue", pr.number),
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
        c.change_id = format!("{id}{}", "z".repeat(31));
        c.bookmarks = bookmarks.iter().map(|s| s.to_string()).collect();
        c
    }

    fn pr(number: u64, head: &str, base: &str) -> PrInfo {
        PrInfo {
            number,
            title: String::new(),
            url: String::new(),
            body: String::new(),
            state: PrState::Open,
            draft: false,
            head_ref: head.into(),
            base_ref: base.into(),
            head_oid: String::new(),
            review_decision: None,
            reviews: vec![],
            checks: None,
            merge_queue: None,
            stack: None,
        }
    }

    #[test]
    fn plans_creates_and_retargets() {
        let prs = HashMap::from([
            ("feat-a".to_string(), pr(5, "feat-a", "main")),
            ("feat-b".to_string(), pr(6, "feat-b", "main")),
        ]);
        let p = plan(
            vec![
                commit("a", &["feat-a"]),
                commit("b", &["feat-b"]),
                commit("c", &[]),
            ],
            &prs,
            "main",
        );
        assert_eq!(p.entries[0].action, PrAction::Keep { number: 5 });
        assert_eq!(
            p.entries[1].action,
            PrAction::Retarget {
                number: 6,
                from: "main".into(),
                to: "feat-a".into()
            }
        );
        assert_eq!(p.entries[2].bookmark, "push-czzzzzzzzzzz");
        assert!(p.entries[2].set_bookmark);
        assert_eq!(
            p.entries[2].action,
            PrAction::Create {
                base: "feat-b".into()
            }
        );
    }

    #[test]
    fn summarizes_fetch() {
        let out = "git: From x\ngit:    a..b  main -> origin/main\nbookmark: main@origin [updated] tracked";
        assert_eq!(fetch_summary(out), "fetched: main@origin [updated] tracked");
        assert_eq!(fetch_summary("Nothing changed."), "fetched; nothing new");
    }

    #[test]
    fn footer_roundtrip() {
        let body = with_footer("Hello", &[1, 2, 3], 2);
        assert!(body.starts_with("Hello\n\n<!-- jj-helper:stack -->"));
        assert!(body.contains("* #3\n* ➡️ **#2** (this PR)\n* #1\n"));
        let again = with_footer(&body, &[1, 2, 3], 2);
        assert_eq!(body, again);
        assert_eq!(with_footer(&body, &[2], 2), "Hello");
    }
}
