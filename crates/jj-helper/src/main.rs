//! jj-helper: a terminal take on VisualJJ. Shape commits visually (every
//! rebase/squash/abandon is previewed as an after-graph before it runs) and
//! drive GitHub stacked PRs, including GitHub's native PR stacks.

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

    /// Don't link PRs into a GitHub native stack (keeps plain base-chained PRs).
    #[arg(long, global = true, env = "JJ_HELPER_NO_NATIVE_STACK")]
    no_native_stack: bool,

    /// Don't write the stack list into PR descriptions.
    #[arg(long, global = true)]
    no_stack_footer: bool,

    /// Open new PRs as drafts.
    #[arg(long, global = true)]
    draft: bool,

    /// Merge method used when shipping.
    #[arg(long, global = true, value_enum, default_value_t = MergeMethod::Squash, env = "JJ_HELPER_MERGE_METHOD")]
    merge_method: MergeMethod,

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

#[derive(Subcommand)]
enum Command {
    /// Interactive TUI (default).
    Tui,
    /// Print the commit graph with PR status.
    Log,
    /// Create/update one PR per commit from trunk to REV and link them as a stack.
    Submit {
        #[arg(default_value = "@")]
        rev: String,
        /// Only print the plan.
        #[arg(long)]
        dry_run: bool,
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
    let submit = SubmitOptions {
        remote: cli.remote.clone(),
        native_stack: !cli.no_native_stack,
        footer: !cli.no_stack_footer,
        draft: cli.draft,
    };
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
                    merge_method: cli.merge_method.as_str().to_string(),
                    auto_fetch: minutes(cli.auto_fetch),
                    auto_rebase: cli.auto_rebase,
                    refresh: secs(cli.refresh),
                },
            )
        }
        Command::Log => print_log(&jj, &cli.revisions, &cli.remote),
        Command::Submit { rev, dry_run } => {
            let gh = GitHub::for_remote(&jj, &cli.remote)?;
            let commits = stack::stack_commits(&jj, &rev)?;
            let mut branches: Vec<String> =
                commits.iter().flat_map(|c| c.bookmarks.clone()).collect();
            branches.extend(
                commits
                    .iter()
                    .map(|c| stack::default_bookmark(&c.change_id)),
            );
            let prs = gh.prs_for_branches(&branches)?;
            let plan = stack::plan(commits, &prs, &jj.trunk_branch());
            for l in plan.describe(submit.native_stack) {
                println!("{l}");
            }
            if dry_run {
                return Ok(());
            }
            let numbers = stack::execute(&jj, &gh, &plan, &submit, &mut |l| eprintln!("{l}"))?;
            let list: Vec<String> = numbers.iter().map(|n| format!("#{n}")).collect();
            println!("PRs (bottom first): {}", list.join(" ← "));
            Ok(())
        }
        Command::Ship { rev } => {
            let gh = GitHub::for_remote(&jj, &cli.remote)?;
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
            let msg = stack::ship(
                &jj,
                &gh,
                pr,
                commit,
                cli.merge_method.as_str(),
                &cli.remote,
                &mut |l| eprintln!("{l}"),
            )?;
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
        let text = match l.kind {
            graph::Kind::Node => {
                let mut parts = vec![n.commit.short_change.clone()];
                parts.extend(n.commit.bookmarks.iter().cloned());
                let pr = n.commit.bookmarks.iter().filter_map(|b| prs.get(b)).next();
                if let Some(pr) = pr {
                    let mut badge = format!("#{}", pr.number);
                    if let Some(s) = &pr.stack {
                        badge.push_str(&format!(" [{}/{}]", s.position, s.size));
                    }
                    parts.push(badge);
                }
                parts.push(n.commit.short_commit.clone());
                if let Some(note) = &n.note {
                    parts.push(format!("◀ {note}"));
                }
                parts.join(" ")
            }
            graph::Kind::Detail => {
                let t = n.commit.title();
                if t.is_empty() {
                    "(no description set)".to_string()
                } else {
                    t.to_string()
                }
            }
            graph::Kind::Connector => String::new(),
        };
        out.push_str(format!("{}{}", l.graph, text).trim_end());
        out.push('\n');
    }
    out
}
