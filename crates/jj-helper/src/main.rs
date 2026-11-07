//! jj-helper: a terminal take on VisualJJ. Shape commits visually (every
//! rebase/squash/abandon is previewed as an after-graph before it runs) and
//! drive GitHub (stacked) PRs the way VisualJJ's `jj branching` commands do.

mod dag;
mod github;
mod graph;
mod jj;
mod ops;
mod stack;
mod tui;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand, ValueEnum};

use crate::dag::Dag;
use crate::github::GitHub;
use crate::jj::Jj;
use crate::stack::SubmitOptions;

#[derive(Parser)]
#[command(
    name = "jj-helper",
    version,
    about = "Visual TUI for Jujutsu with GitHub stacked PRs"
)]
struct Cli {
    /// Repository to operate on (defaults to the current directory).
    #[arg(short = 'R', long, global = true)]
    repository: Option<PathBuf>,

    /// Revisions to show.
    #[arg(short, long, global = true, default_value = jj::DEFAULT_REVSET)]
    revisions: String,

    /// Git remote hosting the PRs.
    #[arg(
        long,
        global = true,
        default_value = "origin",
        env = "JJ_HELPER_REMOTE"
    )]
    remote: String,

    /// Link PRs into a GitHub native stack (overrides the repo's
    /// `visualjj.stacking-github-stacks`).
    #[arg(long, global = true, overrides_with = "no_native_stack")]
    native_stack: bool,

    /// Don't link PRs into a GitHub native stack (overrides the repo's
    /// `visualjj.stacking-github-stacks`).
    #[arg(long, global = true, overrides_with = "native_stack")]
    no_native_stack: bool,

    /// Write the stack list into PR descriptions (overrides the repo's
    /// `visualjj.stacking-update-description`).
    #[arg(long, global = true, overrides_with = "no_stack_info")]
    stack_info: bool,

    /// Don't write the stack list into PR descriptions.
    #[arg(long, global = true, overrides_with = "stack_info")]
    no_stack_info: bool,

    /// Open new PRs as drafts.
    #[arg(long, global = true)]
    draft: bool,

    /// Merge method used when shipping (defaults to your GitHub default).
    #[arg(long, global = true, value_enum, env = "JJ_HELPER_MERGE_METHOD")]
    merge_method: Option<MergeMethod>,

    /// Fetch automatically every N minutes in the TUI (0 disables).
    #[arg(long, default_value_t = 5, env = "JJ_HELPER_AUTO_FETCH")]
    auto_fetch: u64,

    /// After each fetch, rebase your stacks onto the new trunk (undone if it
    /// would introduce conflicts). Toggle in the TUI with T.
    #[arg(long, env = "JJ_HELPER_AUTO_REBASE")]
    auto_rebase: bool,

    /// Reload the graph every N seconds while idle (0 disables).
    #[arg(long, default_value_t = 10)]
    refresh: u64,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Clone, Copy, ValueEnum)]
enum MergeMethod {
    Squash,
    Merge,
    Rebase,
}

impl MergeMethod {
    fn as_str(self) -> &'static str {
        match self {
            MergeMethod::Squash => "squash",
            MergeMethod::Merge => "merge",
            MergeMethod::Rebase => "rebase",
        }
    }
}

fn flag_or(left: bool, right: bool) -> Option<bool> {
    match (left, right) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    }
}

#[derive(Subcommand)]
enum Command {
    /// Interactive TUI (default).
    Tui,
    /// Print the commit graph with PR status.
    Log,
    /// Create/update the PR for REV (VisualJJ's Create/Update Request).
    Submit {
        #[arg(default_value = "@")]
        rev: String,
        /// Submit every change from trunk to REV, bottom up.
        #[arg(long)]
        stack: bool,
        /// Only print the plan.
        #[arg(long)]
        dry_run: bool,
        /// Title for the PR (defaults to the description's first line).
        #[arg(long)]
        title: Option<String>,
        /// Create the bookmark with this exact name.
        #[arg(long)]
        new_bookmark: Option<String>,
    },
    /// Merge REV's PR (and everything below it in its stack) via merge-async.
    Ship {
        #[arg(default_value = "@-")]
        rev: String,
    },
    /// Fetch, optionally rebasing stacks onto the new trunk.
    Sync {
        #[arg(long)]
        rebase: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let jj = Jj::new(cli.repository.clone());
    let submit = SubmitOptions::resolve(
        &jj,
        cli.remote.clone(),
        flag_or(cli.native_stack, cli.no_native_stack),
        flag_or(cli.stack_info, cli.no_stack_info),
        cli.draft,
    );
    match cli.command.unwrap_or(Command::Tui) {
        Command::Tui => {
            let minutes = |m: u64| (m > 0).then(|| Duration::from_secs(m * 60));
            let secs = |s: u64| (s > 0).then(|| Duration::from_secs(s));
            tui::run(
                jj,
                tui::Config {
                    revset: cli.revisions,
                    remote: cli.remote,
                    submit,
                    merge_method: cli.merge_method.map(|m| m.as_str().to_string()),
                    auto_fetch: minutes(cli.auto_fetch),
                    auto_rebase: cli.auto_rebase,
                    refresh: secs(cli.refresh),
                },
            )
        }
        Command::Log => print_log(&jj, &cli.revisions, &cli.remote),
        Command::Submit {
            rev,
            stack,
            dry_run,
            title,
            new_bookmark,
        } => {
            let gh = GitHub::for_remote(&jj, &cli.remote)?;
            let trunk = jj.trunk_branch();
            let commits = if stack {
                stack_commits(&jj, &rev)?
            } else {
                let commits = jj.log_commits(&rev)?;
                match commits.len() {
                    1 => commits,
                    0 => bail!("{rev} resolved to nothing"),
                    _ => bail!("{rev} must resolve to exactly one commit"),
                }
            };
            let mut branches: Vec<String> =
                commits.iter().flat_map(|c| c.bookmarks.clone()).collect();
            branches.extend(
                commits
                    .iter()
                    .map(|c| stack::default_bookmark(&c.change_id)),
            );
            let prs = gh.prs_for_branches(&branches)?;
            let mut plan = stack::plan(commits, &prs, &trunk, &cli.remote)?;
            if let (Some(title), Some(e)) = (title.as_deref(), plan.entries.last_mut()) {
                if let stack::Action::Create { title: t, .. } = &mut e.action {
                    *t = title.to_string();
                }
            }
            if let (Some(name), Some(e)) = (new_bookmark.as_deref(), plan.entries.last_mut()) {
                if let stack::Action::Create {
                    new_bookmark: n, ..
                } = &mut e.action
                {
                    *n = Some(name.to_string());
                    e.bookmark = name.to_string();
                }
            }
            for l in plan.describe() {
                println!("{l}");
            }
            for c in plan.commands() {
                println!("  {c}");
            }
            if dry_run {
                return Ok(());
            }
            let numbers = stack::execute(&jj, &gh, &plan, &submit, &mut |l| eprintln!("{l}"))?;
            for n in &numbers {
                println!("{n}");
            }
            Ok(())
        }
        Command::Ship { rev } => {
            let gh = GitHub::for_remote(&jj, &cli.remote)?;
            let method = match cli.merge_method {
                Some(m) => m.as_str().to_string(),
                // VisualJJ ships with the viewer's default merge method.
                None => gh.viewer_default_merge_method()?.to_lowercase(),
            };
            let commits = jj.log_commits(&rev)?;
            let [commit] = commits.as_slice() else {
                bail!("{rev} must resolve to exactly one commit")
            };
            let prs = gh.prs_for_branches(&commit.bookmarks)?;
            let Some(pr) = commit
                .bookmarks
                .iter()
                .filter_map(|b| prs.get(b))
                .find(|p| p.is_open())
            else {
                bail!("no open PR for {}", commit.short_change);
            };
            let msg = stack::ship(&jj, &gh, pr, commit, &method, &cli.remote, &mut |l| {
                eprintln!("{l}")
            })?;
            println!("{msg}");
            Ok(())
        }
        Command::Sync { rebase } => {
            let msg =
                stack::fetch_and_rebase(&jj, &cli.remote, rebase || cli.auto_rebase, &mut |l| {
                    eprintln!("{l}")
                })?;
            println!("{msg}");
            Ok(())
        }
    }
}

/// Changes from trunk (exclusive) to `rev`, bottom first — for `--stack`.
fn stack_commits(jj: &Jj, rev: &str) -> Result<Vec<crate::jj::Commit>> {
    let mut commits = jj.log_commits(&format!("trunk()..({rev})"))?;
    commits.reverse();
    while commits
        .last()
        .is_some_and(|c| c.empty && c.description.trim().is_empty())
    {
        commits.pop();
    }
    if commits.is_empty() {
        bail!("nothing between trunk and this commit to submit");
    }
    Ok(commits)
}

fn print_log(jj: &Jj, revset: &str, remote: &str) -> Result<()> {
    let repo = jj.load_graph(revset)?;
    let dag = Dag::from_repo(&repo);
    let prs = GitHub::for_remote(jj, remote)
        .ok()
        .and_then(|gh| {
            let branches: Vec<String> = dag
                .nodes
                .values()
                .filter(|n| !n.commit.trunk)
                .flat_map(|n| n.commit.bookmarks.clone())
                .collect();
            gh.prs_for_branches(&branches).ok()
        })
        .unwrap_or_default();
    print!("{}", plain_graph(&dag, &prs));
    Ok(())
}

/// Uncolored rendering used by `jj-helper log` (and handy in tests).
pub fn plain_graph(dag: &Dag, prs: &HashMap<String, github::PrInfo>) -> String {
    let mut out = String::new();
    for l in graph::layout(dag) {
        let n = &dag.nodes[&l.owner];
        // One VisualJJ-style line per change: pills, then the description,
        // then the change-id chip.
        let text = match l.kind {
            graph::Kind::Node => {
                let mut parts = vec![];
                if n.commit.is_wc {
                    parts.push("You are here · editing".to_string());
                }
                parts.extend(n.commit.bookmarks.iter().cloned());
                let pr = n.commit.bookmarks.iter().filter_map(|b| prs.get(b)).next();
                if let Some(pr) = pr {
                    let mut badge = format!("#{}", pr.number);
                    if let Some(s) = &pr.stack {
                        badge.push_str(&format!(" [{}/{}]", s.position, s.size));
                    }
                    parts.push(badge);
                }
                parts.push(if n.commit.title().is_empty() {
                    "Describe…".to_string()
                } else {
                    n.commit.title().to_string()
                });
                parts.push(tui::render::relative_time(&n.commit.timestamp));
                parts.push(n.commit.short_change.clone());
                if let Some(note) = &n.note {
                    parts.push(format!("◀ {note}"));
                }
                parts.join(" ")
            }
            graph::Kind::Connector => String::new(),
        };
        out.push_str(format!("{}{}", l.graph, text).trim_end());
        out.push('\n');
    }
    out
}
