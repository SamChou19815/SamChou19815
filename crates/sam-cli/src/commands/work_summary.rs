//! `sam work-summary` — round up unreleased work across git repos under ~/Desktop.
//!
//! Scans for git repositories and reports the ones with uncommitted changes,
//! unpushed commits, or no upstream at all — i.e. work that only exists locally.
//!
//! Some repos are managed by [jj](https://jj-vcs.github.io/) rather than git
//! directly. Those are colocated repos with a `.jj` directory, so they're still
//! queryable through git; we keep using git for the status counts and just flag
//! the repo as jj-managed in the output.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::charts::{bold, paint, Color};
use crate::cli::WorkSummaryArgs;

/// How deep below the scan root to look for repositories.
const MAX_DEPTH: usize = 4;
/// Directories never worth descending into when hunting for repos.
const SKIP_DIRS: &[&str] = &["node_modules", "target", "dist", "build", ".next"];

/// The interesting state of a single repository.
struct RepoStatus {
    name: String,
    branch: String,
    /// Number of changed (staged or unstaged) paths.
    dirty: usize,
    /// Commits on HEAD not on the upstream branch.
    ahead: u32,
    /// Commits on the upstream branch not on HEAD.
    behind: u32,
    /// Whether HEAD tracks an upstream branch at all.
    has_upstream: bool,
    /// Whether the repo is colocated with jj (has a `.jj` directory).
    is_jj: bool,
    last_commit: String,
}

impl RepoStatus {
    /// Work that exists only locally: uncommitted, unpushed, or untracked branch.
    fn is_unreleased(&self) -> bool {
        self.dirty > 0 || self.ahead > 0 || !self.has_upstream
    }
}

pub fn run(args: WorkSummaryArgs) -> Result<()> {
    let root = match args.path {
        Some(path) => path,
        None => default_root()?,
    };
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let repos = find_repos(&root);
    println!(
        "{}",
        bold(&paint(
            &format!("Unreleased work under {}", root.display()),
            Color::Blue,
        ))
    );
    println!();

    if repos.is_empty() {
        println!("{}", paint("No git repositories found.", Color::Dim));
        return Ok(());
    }

    let mut pending = 0;
    for repo in &repos {
        let Ok(status) = repo_status(repo) else {
            continue;
        };
        if !status.is_unreleased() {
            continue;
        }
        pending += 1;
        print_status(&status);
    }

    if pending == 0 {
        println!(
            "{}",
            paint(
                &format!("All {} repo(s) are committed and pushed. ✨", repos.len()),
                Color::Green,
            )
        );
    } else {
        println!(
            "{}",
            paint(
                &format!("{pending} of {} repo(s) have unreleased work.", repos.len()),
                Color::Dim,
            )
        );
    }
    Ok(())
}

/// `~/Desktop`, resolved for the current user.
fn default_root() -> Result<PathBuf> {
    let dirs = directories::UserDirs::new().context("could not determine the home directory")?;
    Ok(dirs
        .desktop_dir()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| dirs.home_dir().join("Desktop")))
}

/// Find git repositories under `root`, without descending into a repo or into
/// noisy build directories, and bounded by [`MAX_DEPTH`].
fn find_repos(root: &Path) -> Vec<PathBuf> {
    let mut repos = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if dir.join(".git").exists() {
            repos.push(dir);
            continue;
        }
        if depth >= MAX_DEPTH {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref()) {
                continue;
            }
            stack.push((entry.path(), depth + 1));
        }
    }
    repos.sort();
    repos
}

/// Collect the status of one repository, using jj for colocated jj repos and
/// git otherwise.
fn repo_status(dir: &Path) -> Result<RepoStatus> {
    if dir.join(".jj").exists() {
        return jj_status(dir);
    }
    git_status(dir)
}

/// The directory's final path component, used as the repo's display name.
fn repo_name(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| dir.display().to_string())
}

/// Collect the status of a plain git repository.
fn git_status(dir: &Path) -> Result<RepoStatus> {
    let name = repo_name(dir);
    let branch = git(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|_| "?".into());
    let dirty = git(dir, &["status", "--porcelain"])
        .map(|out| out.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);

    // `--left-right --count A...B` prints "<behind> <ahead>"; it fails when HEAD
    // has no upstream, which we treat as unreleased work in its own right.
    let (ahead, behind, has_upstream) =
        match git(dir, &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"]) {
            Ok(out) => {
                let mut counts = out.split_whitespace();
                let behind = counts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let ahead = counts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                (ahead, behind, true)
            }
            Err(_) => (0, 0, false),
        };
    let last_commit =
        git(dir, &["log", "-1", "--format=%cr · %s"]).unwrap_or_else(|_| "no commits yet".into());

    Ok(RepoStatus {
        name,
        branch,
        dirty,
        ahead,
        behind,
        has_upstream,
        is_jj: false,
        last_commit,
    })
}

/// Collect the status of a jj-colocated repo via jj.
///
/// A colocated repo has no git tracking branch (jj manages bookmark→remote
/// relationships itself), so git's `@{upstream}` view reports every such repo as
/// having "no upstream" — a false positive. Asking jj directly avoids that:
/// "dirty" is the working-copy commit's changes, and "unpushed" is the commits
/// reachable from local bookmarks (or the working copy) that aren't on any remote.
fn jj_status(dir: &Path) -> Result<RepoStatus> {
    let name = repo_name(dir);
    // The nearest bookmark at or behind @, falling back to @'s change id.
    let branch = jj(
        dir,
        &[
            "log",
            "--no-graph",
            "-r",
            "latest(::@ & bookmarks(), 1)",
            "-T",
            "bookmarks.map(|b| b.name()).join(\",\")",
        ],
    )
    .ok()
    .filter(|s| !s.is_empty())
    .or_else(|| jj(dir, &["log", "--no-graph", "-r", "@", "-T", "change_id.short()"]).ok())
    .unwrap_or_else(|| "?".into());

    // Working-copy changes are jj's analog of uncommitted work.
    let dirty = jj(dir, &["diff", "--name-only", "-r", "@"])
        .map(|out| out.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);

    // Commits reachable from local bookmarks (or @) that aren't on any remote,
    // excluding the working copy itself (counted as `dirty`) and empties.
    let ahead = jj(
        dir,
        &[
            "log",
            "--no-graph",
            "-r",
            "remote_bookmarks()..(bookmarks() | @) & ~empty() & ~@",
            "-T",
            "\"x\\n\"",
        ],
    )
    .map(|out| out.lines().filter(|l| !l.trim().is_empty()).count() as u32)
    .unwrap_or(0);

    let last_commit = jj(
        dir,
        &[
            "log",
            "--no-graph",
            "-r",
            "latest(::@ & ~empty() & ~description(\"\"), 1)",
            "-T",
            "committer.timestamp().ago() ++ \" · \" ++ description.first_line()",
        ],
    )
    .ok()
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "no commits yet".into());

    Ok(RepoStatus {
        name,
        branch,
        dirty,
        ahead,
        // jj tracks remotes itself, so git's upstream/behind notions don't apply;
        // mark upstream present to suppress the misleading "no upstream" flag.
        behind: 0,
        has_upstream: true,
        is_jj: true,
        last_commit,
    })
}

/// Run `git -C <dir> <args>`, returning trimmed stdout or an error on failure.
fn git(dir: &Path, args: &[&str]) -> Result<String> {
    capture("git", Some(dir), args)
}

/// Run `jj -R <dir> <args>`, returning trimmed stdout or an error on failure.
fn jj(dir: &Path, args: &[&str]) -> Result<String> {
    let mut full = vec!["-R"];
    let dir = dir.to_string_lossy();
    full.push(&dir);
    full.extend_from_slice(args);
    capture("jj", None, &full)
}

/// Run `<program> [-C <dir>] <args>`, returning trimmed stdout or an error.
fn capture(program: &str, c_dir: Option<&Path>, args: &[&str]) -> Result<String> {
    let mut command = Command::new(program);
    if let Some(dir) = c_dir {
        command.arg("-C").arg(dir);
    }
    let output = command
        .args(args)
        .output()
        .with_context(|| format!("running {program}"))?;
    if !output.status.success() {
        bail!("{program} {} failed", args.join(" "));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn print_status(s: &RepoStatus) {
    println!(
        "{}  {}{}",
        bold(&paint(&s.name, Color::Cyan)),
        paint(&format!("({})", s.branch), Color::Dim),
        if s.is_jj {
            paint(" [jj]", Color::Magenta)
        } else {
            String::new()
        },
    );

    let mut bits = Vec::new();
    if s.dirty > 0 {
        bits.push(paint(&format!("{} uncommitted", s.dirty), Color::Red));
    }
    if s.ahead > 0 {
        bits.push(paint(&format!("{} unpushed", s.ahead), Color::Yellow));
    }
    if s.behind > 0 {
        bits.push(paint(&format!("{} behind", s.behind), Color::Blue));
    }
    if !s.has_upstream {
        bits.push(paint("no upstream", Color::Yellow));
    }
    let separator = paint(" · ", Color::Dim);
    println!("  {}", bits.join(&separator));
    println!("  {}", paint(&s.last_commit, Color::Dim));
    println!();
}
