//! Thin wrapper around the `jj` CLI: running commands and loading the commit
//! graph shown in the TUI.
//!
//! Everything goes through the user's own `jj` binary (no jj-lib linkage), so
//! jj-helper always agrees with whatever jj version and config the user runs.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

/// Default set of revisions shown, mirroring jj's own default `revsets.log`.
pub const DEFAULT_REVSET: &str =
    "present(@) | ancestors(immutable_heads().., 2) | present(trunk())";

/// One JSON object per line. `json(self)` covers the commit's core fields;
/// the rest are hand-assembled because `json()` only accepts serializable
/// values (not lists of templates).
const LOG_TEMPLATE: &str = r#""{\"commit\":" ++ json(self)
++ ",\"short_change\":" ++ json(stringify(change_id.shortest(8)))
++ ",\"change_prefix_len\":" ++ change_id.shortest().prefix().len()
++ ",\"short_commit\":" ++ json(stringify(commit_id.short(8)))
++ ",\"bookmarks\":[" ++ local_bookmarks.map(|b| json(b.name())).join(",") ++ "]"
++ ",\"remote_bookmarks\":[" ++ remote_bookmarks.filter(|b| b.remote() != "git").map(|b| json(stringify(b.name() ++ "@" ++ b.remote()))).join(",") ++ "]"
++ ",\"tags\":[" ++ tags.map(|b| json(b.name())).join(",") ++ "]"
++ ",\"working_copies\":[" ++ working_copies.map(|w| json(w.name())).join(",") ++ "]"
++ ",\"is_wc\":" ++ json(current_working_copy)
++ ",\"immutable\":" ++ json(immutable)
++ ",\"empty\":" ++ json(empty)
++ ",\"conflict\":" ++ json(conflict)
++ ",\"divergent\":" ++ json(divergent)
++ ",\"trunk\":" ++ json(self.contained_in("present(trunk())"))
++ ",\"mine\":" ++ json(self.contained_in("mine()"))
++ "}\n""#;

#[derive(Debug, Clone, Deserialize)]
struct RawSignature {
    name: String,
    email: String,
    timestamp: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RawCommitCore {
    commit_id: String,
    parents: Vec<String>,
    change_id: String,
    description: String,
    author: RawSignature,
}

#[derive(Debug, Clone, Deserialize)]
struct RawCommit {
    commit: RawCommitCore,
    short_change: String,
    change_prefix_len: usize,
    short_commit: String,
    bookmarks: Vec<String>,
    remote_bookmarks: Vec<String>,
    tags: Vec<String>,
    working_copies: Vec<String>,
    is_wc: bool,
    immutable: bool,
    empty: bool,
    conflict: bool,
    divergent: bool,
    trunk: bool,
    mine: bool,
}

/// A commit as displayed in the graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub commit_id: String,
    pub change_id: String,
    /// Change id shortened to 8 chars, of which the first
    /// `change_prefix_len` are the unique prefix (highlighted like jj does).
    pub short_change: String,
    pub change_prefix_len: usize,
    pub short_commit: String,
    pub parents: Vec<String>,
    pub description: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: String,
    pub bookmarks: Vec<String>,
    pub remote_bookmarks: Vec<String>,
    pub tags: Vec<String>,
    pub working_copies: Vec<String>,
    pub is_wc: bool,
    pub immutable: bool,
    pub empty: bool,
    pub conflict: bool,
    pub divergent: bool,
    pub trunk: bool,
    pub mine: bool,
}

impl Commit {
    pub fn title(&self) -> &str {
        self.description.lines().next().unwrap_or("").trim()
    }

    /// The description minus its first line, trimmed.
    pub fn body(&self) -> String {
        let mut lines = self.description.lines();
        lines.next();
        lines.collect::<Vec<_>>().join("\n").trim().to_string()
    }

    /// Placeholder for a synthetic commit used in previews.
    pub fn synthetic(id: &str, description: &str) -> Commit {
        Commit {
            commit_id: id.to_string(),
            change_id: id.to_string(),
            short_change: "(new)".to_string(),
            change_prefix_len: 0,
            short_commit: String::new(),
            parents: vec![],
            description: description.to_string(),
            author_name: String::new(),
            author_email: String::new(),
            timestamp: String::new(),
            bookmarks: vec![],
            remote_bookmarks: vec![],
            tags: vec![],
            working_copies: vec![],
            is_wc: false,
            immutable: false,
            empty: true,
            conflict: false,
            divergent: false,
            trunk: false,
            mine: true,
        }
    }
}

impl From<RawCommit> for Commit {
    fn from(r: RawCommit) -> Self {
        Commit {
            commit_id: r.commit.commit_id,
            change_id: r.commit.change_id,
            short_change: r.short_change,
            change_prefix_len: r.change_prefix_len,
            short_commit: r.short_commit,
            parents: r.commit.parents,
            description: r.commit.description,
            author_name: r.commit.author.name,
            author_email: r.commit.author.email,
            timestamp: r.commit.author.timestamp,
            bookmarks: r.bookmarks,
            remote_bookmarks: r.remote_bookmarks,
            tags: r.tags,
            working_copies: r.working_copies,
            is_wc: r.is_wc,
            immutable: r.immutable,
            empty: r.empty,
            conflict: r.conflict,
            divergent: r.divergent,
            trunk: r.trunk,
            mine: r.mine,
        }
    }
}

/// Parents of a commit restricted to the displayed set. `elided` edges skip
/// over commits outside the set (drawn dashed); `truncated` means some
/// ancestry continues past the bottom of the displayed set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphEdges {
    pub parents: Vec<(String, bool)>,
    pub truncated: bool,
}

/// Snapshot of the repo as loaded from jj, in jj's display order (children
/// before parents).
#[derive(Debug, Clone, Default)]
pub struct RepoGraph {
    pub commits: Vec<Commit>,
    pub edges: HashMap<String, GraphEdges>,
}

#[derive(Debug, Clone)]
pub struct Jj {
    pub repo: Option<PathBuf>,
}

/// Output of a finished jj invocation.
#[derive(Debug, Clone)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    /// The human-oriented summary jj prints (jj writes it to stderr).
    pub fn summary(&self) -> String {
        let text = if self.stderr.trim().is_empty() {
            &self.stdout
        } else {
            &self.stderr
        };
        text.trim().to_string()
    }
}

impl Jj {
    pub fn new(repo: Option<PathBuf>) -> Self {
        Jj { repo }
    }

    pub fn command(&self) -> Command {
        let mut cmd = Command::new(std::env::var("JJ_HELPER_JJ").unwrap_or_else(|_| "jj".into()));
        if let Some(repo) = &self.repo {
            cmd.arg("-R").arg(repo);
        }
        cmd.args(["--no-pager", "--color", "never"]);
        // Never let jj open an editor behind the TUI's back.
        cmd.env("JJ_EDITOR", "false");
        cmd
    }

    /// Runs jj and fails with its stderr on a non-zero exit.
    pub fn run<S: AsRef<str>>(&self, args: &[S]) -> Result<Output> {
        let args: Vec<&str> = args.iter().map(|a| a.as_ref()).collect();
        let out = self
            .command()
            .args(&args)
            .stdin(Stdio::null())
            .output()
            .context("failed to run jj (is it installed and on PATH?)")?;
        let output = Output {
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        };
        if !out.status.success() {
            bail!("jj {} failed:\n{}", shell_join(&args), output.stderr.trim());
        }
        Ok(output)
    }

    /// Runs jj attached to the terminal (for interactive commands such as
    /// `jj split`). The caller must have released the terminal first.
    pub fn run_interactive(&self, args: &[String]) -> Result<()> {
        let mut cmd = Command::new(std::env::var("JJ_HELPER_JJ").unwrap_or_else(|_| "jj".into()));
        if let Some(repo) = &self.repo {
            cmd.arg("-R").arg(repo);
        }
        let status = cmd.args(args).status().context("failed to run jj")?;
        if !status.success() {
            bail!("jj {} exited with {status}", shell_join(args));
        }
        Ok(())
    }

    pub fn workspace_root(&self) -> Result<PathBuf> {
        Ok(PathBuf::from(self.run(&["root"])?.stdout.trim()))
    }

    pub fn log_commits(&self, revset: &str) -> Result<Vec<Commit>> {
        let out = self.run(&["log", "--no-graph", "-r", revset, "-T", LOG_TEMPLATE])?;
        parse_log(&out.stdout)
    }

    /// Commit ids matching `revset`, in jj's order.
    pub fn commit_ids(&self, revset: &str) -> Result<Vec<String>> {
        let out = self.run(&[
            "log",
            "--no-graph",
            "--ignore-working-copy",
            "-r",
            revset,
            "-T",
            "commit_id ++ \"\\n\"",
        ])?;
        Ok(out
            .stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    /// Loads the displayed graph. Parents outside the displayed set are
    /// resolved to their nearest displayed ancestors (one small query per
    /// boundary commit, run in parallel).
    pub fn load_graph(&self, revset: &str) -> Result<RepoGraph> {
        let commits = self.log_commits(revset)?;
        let visible: HashSet<&str> = commits.iter().map(|c| c.commit_id.as_str()).collect();
        let mut edges = HashMap::new();
        let mut boundary = vec![];
        for c in &commits {
            let mut e = GraphEdges::default();
            let mut missing = vec![];
            for p in &c.parents {
                if visible.contains(p.as_str()) {
                    e.parents.push((p.clone(), false));
                } else {
                    missing.push(p.clone());
                }
            }
            if !missing.is_empty() {
                boundary.push((c.commit_id.clone(), missing));
            }
            edges.insert(c.commit_id.clone(), e);
        }
        // Capped so a pathological revset can't spawn hundreds of jj calls.
        let resolved: Vec<(String, Result<Vec<String>>)> = std::thread::scope(|s| {
            let handles: Vec<_> = boundary
                .iter()
                .take(48)
                .map(|(id, missing)| {
                    s.spawn(move || {
                        let rs = format!("heads(({revset}) & ::({}))", missing.join(" | "));
                        (id.clone(), self.commit_ids(&rs))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("graph thread panicked"))
                .collect()
        });
        let resolved: HashMap<String, Result<Vec<String>>> = resolved.into_iter().collect();
        for (id, missing) in &boundary {
            let e = edges.get_mut(id).expect("edge entry exists");
            let ancestors = match resolved.get(id) {
                Some(Ok(a)) => a.clone(),
                _ => vec![],
            };
            for a in &ancestors {
                if !e.parents.iter().any(|(p, _)| p == a) {
                    e.parents.push((a.clone(), true));
                }
            }
            // Root commit has no parents at all; anything else that lost a
            // parent to the cut continues below the window.
            if ancestors.is_empty() && !missing.is_empty() {
                e.truncated = true;
            }
        }
        Ok(RepoGraph { commits, edges })
    }

    /// `(name, url)` pairs from `jj git remote list`.
    pub fn git_remotes(&self) -> Result<Vec<(String, String)>> {
        let out = self.run(&["git", "remote", "list"])?;
        Ok(out
            .stdout
            .lines()
            .filter_map(|l| {
                let mut parts = l.split_whitespace();
                Some((parts.next()?.to_string(), parts.next()?.to_string()))
            })
            .collect())
    }

    /// Name of the bookmark `trunk()` resolves to (e.g. `main`).
    pub fn trunk_branch(&self) -> String {
        if let Ok(out) = self.run(&[
            "log",
            "--no-graph",
            "--ignore-working-copy",
            "-r",
            "present(trunk())",
            "-T",
            "remote_bookmarks.filter(|b| b.remote() != \"git\").map(|b| b.name()).join(\"\\n\") ++ \"\\n\" ++ local_bookmarks.map(|b| b.name()).join(\"\\n\")",
        ]) {
            for name in out.stdout.lines().map(str::trim) {
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        "main".to_string()
    }

    pub fn diff_text(&self, rev: &str) -> Result<String> {
        let stat = self
            .run(&["diff", "--ignore-working-copy", "-r", rev, "--stat"])?
            .stdout;
        let diff = self
            .run(&["diff", "--ignore-working-copy", "-r", rev, "--git"])?
            .stdout;
        Ok(format!("{stat}\n{diff}"))
    }

    pub fn interdiff_text(&self, from: &str, to: &str) -> Result<String> {
        let stat = self
            .run(&[
                "interdiff",
                "--ignore-working-copy",
                "--from",
                from,
                "--to",
                to,
                "--stat",
            ])?
            .stdout;
        let diff = self
            .run(&[
                "interdiff",
                "--ignore-working-copy",
                "--from",
                from,
                "--to",
                to,
                "--git",
            ])?
            .stdout;
        Ok(format!("{stat}\n{diff}"))
    }

    /// Commit ids of previous versions of a change, newest first, excluding
    /// the current one.
    pub fn predecessors(&self, rev: &str) -> Result<Vec<String>> {
        let out = self.run(&[
            "evolog",
            "--ignore-working-copy",
            "--no-graph",
            "-r",
            rev,
            "-T",
            "commit.commit_id() ++ \"\\n\"",
        ])?;
        Ok(out
            .stdout
            .lines()
            .skip(1)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    pub fn exists(&self, rev: &str) -> bool {
        self.commit_ids(&format!("present({rev})"))
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }
}

pub fn parse_log(stdout: &str) -> Result<Vec<Commit>> {
    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            serde_json::from_str::<RawCommit>(l)
                .map(Commit::from)
                .map_err(|e| anyhow!("failed to parse jj log output: {e}\n{l}"))
        })
        .collect()
}

/// Quotes args for display (and for pasting into a shell).
pub fn shell_join<S: AsRef<str>>(args: &[S]) -> String {
    args.iter()
        .map(|a| {
            let a = a.as_ref();
            if !a.is_empty()
                && a.chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_./@:=+,()".contains(c))
            {
                a.to_string()
            } else {
                format!("'{}'", a.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_log_line() {
        let line = r#"{"commit":{"commit_id":"abc","parents":["def"],"change_id":"zzz","description":"hello\n\nbody\n","author":{"name":"A","email":"a@b","timestamp":"2026-11-01T15:50:39-05:00"},"committer":{"name":"A","email":"a@b","timestamp":"2026-11-01T15:50:39-05:00"}},"short_change":"zzzzzzzz","change_prefix_len":2,"short_commit":"abcabcab","bookmarks":["main"],"remote_bookmarks":["main@origin"],"tags":[],"working_copies":[],"is_wc":true,"immutable":false,"empty":false,"conflict":false,"divergent":false,"trunk":false,"mine":true}"#;
        let commits = parse_log(line).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].title(), "hello");
        assert_eq!(commits[0].body(), "body");
        assert_eq!(commits[0].parents, vec!["def"]);
        assert!(commits[0].is_wc);
    }

    #[test]
    fn quotes_shell_args() {
        assert_eq!(
            shell_join(&["rebase", "-s", "abc", "-m", "hi there"]),
            "rebase -s abc -m 'hi there'"
        );
    }
}
