//! `sam projects` — a dashboard of the git repos under ~/Desktop.
//!
//! For every repository it fuses two views:
//!
//! - **Unreleased work**: uncommitted changes, unpushed commits, or a branch
//!   with no upstream — work that only exists locally. Some repos are managed by
//!   [jj](https://jj-vcs.github.io/) (colocated, with a `.jj` directory); for
//!   those we query jj instead of git (see [`jj_status`]), since git's upstream
//!   view misreports jj's bookmark→remote relationships.
//! - **Language breakdown**: a per-language byte share, reported the way GitHub
//!   does (via [Linguist]). The unit is **bytes**, not lines — prose languages
//!   have long lines, so the two tell different stories (e.g. MDX outweighs
//!   TypeScript by bytes but not by lines). Only *programming* and *markup*
//!   languages count, which is why GitHub's bar shows MDX/TypeScript/Rust/CSS/TeX
//!   but omits Markdown (prose) and JSON/YAML/TOML (data). Vendored and generated
//!   paths (e.g. `node_modules`, `dist`) are ignored, and only git-tracked files
//!   are considered, so ignored files never count.
//!
//! [Linguist]: https://github.com/github-linguist/linguist

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::charts::{bold, hbar, paint, paint_rgb, Color};
use crate::cli::ProjectsArgs;

/// How deep below the scan root to look for repositories.
const MAX_DEPTH: usize = 4;
/// Directories never worth descending into when hunting for repos.
const SKIP_DIRS: &[&str] = &["node_modules", "target", "dist", "build", ".next"];
/// Width of the per-language byte-share bar, in cells.
const BAR_WIDTH: usize = 20;
/// Path segments that mark vendored or generated trees Linguist would skip.
/// Most are already gitignored (and thus absent from `git ls-files`), but a few
/// projects commit them, so we belt-and-braces filter by path too.
const VENDOR_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    "third_party",
    "dist",
    "build",
    ".next",
    "target",
    "out",
    ".yarn",
    "generated",
];

/// The interesting state of a single repository: its git/jj status and the
/// byte breakdown of its tracked code.
struct RepoStatus {
    name: String,
    branch: String,
    /// Number of changed (staged or unstaged) paths.
    dirty: usize,
    /// Commits on the upstream branch not on HEAD.
    behind: u32,
    /// Whether HEAD tracks an upstream branch at all.
    has_upstream: bool,
    /// Whether the repo is colocated with jj (has a `.jj` directory).
    is_jj: bool,
    /// One line per unreleased (unpushed) commit, newest first. With a
    /// one-commit-per-work convention each entry is a distinct piece of work.
    /// The source of truth for the unpushed count; also the fallback display.
    commits: Vec<String>,
    /// For jj repos, the unpushed commits rendered as a graph (`jj log`), so the
    /// stack structure shows the way it does in `jj log`. Empty for git repos.
    tree: String,
    /// The tip commit, shown for context when there are no unpushed commits to
    /// list (e.g. only uncommitted changes, or a clean repo).
    tip: String,
    /// Byte totals per language, sorted by bytes descending.
    languages: Vec<(&'static str, u64)>,
}

impl RepoStatus {
    /// Work that exists only locally: uncommitted, unpushed, or untracked branch.
    fn is_unreleased(&self) -> bool {
        self.dirty > 0 || !self.commits.is_empty() || !self.has_upstream
    }
}

pub fn run(args: ProjectsArgs) -> Result<()> {
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
            &format!("Projects under {}", root.display()),
            Color::Blue
        ))
    );
    println!();

    if repos.is_empty() {
        println!("{}", paint("No git repositories found.", Color::Dim));
        return Ok(());
    }

    let mut total_langs: HashMap<&'static str, u64> = HashMap::new();
    let mut unreleased = 0;
    let mut shown = 0;
    for repo in &repos {
        let Ok(status) = repo_status(repo) else {
            continue;
        };
        shown += 1;
        if status.is_unreleased() {
            unreleased += 1;
        }
        print_project(&status);
        for (lang, bytes) in &status.languages {
            *total_langs.entry(lang).or_default() += bytes;
        }
    }

    if shown == 0 {
        println!("{}", paint("No readable repositories found.", Color::Dim));
        return Ok(());
    }

    // When more than one project contributes code, show the combined language
    // picture across all of them.
    let total = sorted(total_langs);
    if shown > 1 && !total.is_empty() {
        print_breakdown("All projects", Color::Green, &total);
    }

    if unreleased == 0 {
        println!(
            "{}",
            paint(
                &format!("All {shown} repo(s) are committed and pushed. ✨"),
                Color::Green,
            )
        );
    } else {
        println!(
            "{}",
            paint(
                &format!("{unreleased} of {shown} repo(s) have unreleased work."),
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
/// git otherwise, then attach its language breakdown.
fn repo_status(dir: &Path) -> Result<RepoStatus> {
    let mut status = if dir.join(".jj").exists() {
        jj_status(dir)?
    } else {
        git_status(dir)?
    };
    status.languages = repo_languages(dir);
    Ok(status)
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
    let (behind, has_upstream) = match git(
        dir,
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
    ) {
        Ok(out) => {
            let behind = out
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            (behind, true)
        }
        Err(_) => (0, false),
    };
    // The unpushed commits, newest first. Without an upstream there's no base to
    // diff against, so we leave the list empty and let "no upstream" stand in.
    let commits = if has_upstream {
        git(dir, &["log", "--format=%cr · %s", "@{upstream}..HEAD"])
            .map(|out| {
                out.lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let tip =
        git(dir, &["log", "-1", "--format=%cr · %s"]).unwrap_or_else(|_| "no commits yet".into());

    Ok(RepoStatus {
        name,
        branch,
        dirty,
        behind,
        has_upstream,
        is_jj: false,
        commits,
        tree: String::new(),
        tip,
        languages: Vec::new(),
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
    .or_else(|| {
        jj(
            dir,
            &["log", "--no-graph", "-r", "@", "-T", "change_id.short()"],
        )
        .ok()
    })
    .unwrap_or_else(|| "?".into());

    // Working-copy changes are jj's analog of uncommitted work.
    let dirty = jj(dir, &["diff", "--name-only", "-r", "@"])
        .map(|out| out.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);

    // The unpushed commits: every local-only commit not on any remote, excluding
    // the working copy itself (counted as `dirty`) and empties. We span `all()`
    // rather than just `::@` so that detached stacks — e.g. work orphaned from @
    // by a rebase — are still counted. With a one-commit-per-work convention each
    // commit is a distinct piece of work.
    let revset = "remote_bookmarks()..all() & ~empty() & ~@";
    let template = "committer.timestamp().ago() ++ \" · \" ++ description.first_line() ++ \"\\n\"";

    // A flat newest-first list, used for the count and as a plain-text fallback.
    let commits = jj(dir, &["log", "--no-graph", "-r", revset, "-T", template])
        .map(|out| {
            out.lines()
                .filter(|l| !l.trim().is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    // The same commits rendered as a graph, so the stack structure reads the way
    // it does in `jj log`. Match our own coloring so piped output stays clean.
    let color = if crate::charts::color_enabled() {
        "always"
    } else {
        "never"
    };
    let tree = jj(
        dir,
        &["log", "--color", color, "-r", revset, "-T", template],
    )
    .unwrap_or_default();

    let tip = jj(
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
        // jj tracks remotes itself, so git's upstream/behind notions don't apply;
        // mark upstream present to suppress the misleading "no upstream" flag.
        behind: 0,
        has_upstream: true,
        is_jj: true,
        commits,
        tree,
        tip,
        languages: Vec::new(),
    })
}

/// Byte totals per language for the git-tracked files in `dir`, sorted by bytes
/// descending. Files we can't classify (or stat) contribute nothing.
fn repo_languages(dir: &Path) -> Vec<(&'static str, u64)> {
    let mut bytes: HashMap<&'static str, u64> = HashMap::new();
    for rel in tracked_files(dir) {
        if is_vendored(&rel) {
            continue;
        }
        let Some(lang) = classify(&rel) else {
            continue;
        };
        let Ok(meta) = std::fs::metadata(dir.join(&rel)) else {
            continue;
        };
        *bytes.entry(lang).or_default() += meta.len();
    }
    sorted(bytes)
}

/// Collapse a language→bytes map into a vector sorted by bytes descending, then
/// language name for a stable tie-break.
fn sorted(map: HashMap<&'static str, u64>) -> Vec<(&'static str, u64)> {
    let mut langs: Vec<(&'static str, u64)> = map.into_iter().filter(|(_, b)| *b > 0).collect();
    langs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    langs
}

/// The git-tracked files in `dir`, as paths relative to it. Returns empty on any
/// failure (not a repo, git missing, etc.) so a bad repo just contributes nothing.
fn tracked_files(dir: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["ls-files", "-z"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    // `-z` is NUL-separated so paths with spaces or newlines stay intact.
    output
        .stdout
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| PathBuf::from(String::from_utf8_lossy(s).into_owned()))
        .collect()
}

/// Whether a repo-relative path lives under a vendored or generated directory.
fn is_vendored(rel: &Path) -> bool {
    rel.components()
        .any(|c| VENDOR_DIRS.contains(&c.as_os_str().to_string_lossy().as_ref()))
}

/// Map a file to its Linguist language name, but only for languages of type
/// *programming* or *markup* — the ones GitHub includes in its language bar.
/// Prose (Markdown) and data (JSON/YAML/TOML/SQL) deliberately return `None`.
fn classify(path: &Path) -> Option<&'static str> {
    // A handful of languages are recognized by filename rather than extension.
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name == "Dockerfile" || name.starts_with("Dockerfile.") {
            return Some("Dockerfile");
        }
        if name == "Makefile" || name == "GNUmakefile" {
            return Some("Makefile");
        }
    }

    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let lang = match ext.as_str() {
        "rs" => "Rust",
        "ts" | "tsx" | "mts" | "cts" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "mdx" => "MDX",
        "tex" | "sty" | "cls" => "TeX",
        "css" => "CSS",
        "scss" | "sass" => "SCSS",
        "less" => "Less",
        "html" | "htm" => "HTML",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "py" => "Python",
        "go" => "Go",
        "rb" => "Ruby",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "swift" => "Swift",
        "c" | "h" => "C",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "C++",
        "cs" => "C#",
        "php" => "PHP",
        "sh" | "bash" | "zsh" => "Shell",
        "lua" => "Lua",
        "nix" => "Nix",
        "dart" => "Dart",
        "scala" => "Scala",
        "ml" | "mli" => "OCaml",
        "hs" => "Haskell",
        "ex" | "exs" => "Elixir",
        _ => return None,
    };
    Some(lang)
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

/// Print one repository: a heading, its git/jj status, any unpushed commits, and
/// its language breakdown.
fn print_project(s: &RepoStatus) {
    println!(
        "{}  {}{}  {}",
        bold(&paint(&s.name, Color::Cyan)),
        paint(&format!("({})", s.branch), Color::Dim),
        if s.is_jj {
            paint(" [jj]", Color::Magenta)
        } else {
            String::new()
        },
        paint(&format!("({})", human_bytes(repo_bytes(s))), Color::Dim),
    );

    // Status line: the unreleased-work bits, or a clean confirmation.
    if s.is_unreleased() {
        let mut bits = Vec::new();
        if s.dirty > 0 {
            bits.push(paint(&format!("{} uncommitted", s.dirty), Color::Red));
        }
        if !s.commits.is_empty() {
            bits.push(paint(
                &format!("{} unpushed", s.commits.len()),
                Color::Yellow,
            ));
        }
        if s.behind > 0 {
            bits.push(paint(&format!("{} behind", s.behind), Color::Blue));
        }
        if !s.has_upstream {
            bits.push(paint("no upstream", Color::Yellow));
        }
        let separator = paint(" · ", Color::Dim);
        println!("  {}", bits.join(&separator));
    } else {
        println!("  {}", paint("clean ✨", Color::Green));
    }

    // Unpushed commits, if any; otherwise the tip for context.
    if s.commits.is_empty() {
        println!("  {}", paint(&s.tip, Color::Dim));
    } else if !s.tree.is_empty() {
        // jj: show the unpushed commits as a graph, indented under the repo, so
        // the stack structure reads the way it does in `jj log`.
        for line in s.tree.lines() {
            if line.is_empty() {
                println!();
            } else {
                println!("  {line}");
            }
        }
    } else {
        // git fallback: one piece of work per line.
        let bullet = paint("·", Color::Dim);
        for commit in &s.commits {
            println!("  {bullet} {}", paint(commit, Color::Dim));
        }
    }

    // Language breakdown.
    print_languages(&s.languages);
    println!();
}

/// Total counted bytes across a repo's languages.
fn repo_bytes(s: &RepoStatus) -> u64 {
    s.languages.iter().map(|(_, b)| b).sum()
}

/// Print the per-language rows (share, bar, byte count), indented under a repo.
fn print_languages(langs: &[(&'static str, u64)]) {
    let total: u64 = langs.iter().map(|(_, b)| b).sum();
    if total == 0 {
        return;
    }
    let name_width = langs.iter().map(|(lang, _)| lang.len()).max().unwrap_or(0);
    for (lang, bytes) in langs {
        let share = *bytes as f64 / total as f64 * 100.0;
        let bar = hbar(*bytes as f64, total as f64, BAR_WIDTH);
        println!(
            "  {:<name_width$}  {:>5.1}%  {}  {}",
            lang,
            share,
            paint_rgb(&bar, lang_color(lang)),
            paint(&human_bytes(*bytes), Color::Dim),
        );
    }
}

/// GitHub's display color for a language, as RGB. These mirror the hex colors in
/// [Linguist]'s `languages.yml`, so the bar reads the way the language strip does
/// on a repo's GitHub page. Languages without a known color fall back to a
/// neutral gray (matching how GitHub renders uncolored languages).
///
/// [Linguist]: https://github.com/github-linguist/linguist/blob/main/lib/linguist/languages.yml
fn lang_color(lang: &str) -> (u8, u8, u8) {
    match lang {
        "Rust" => (0xde, 0xa5, 0x84),
        "TypeScript" => (0x31, 0x78, 0xc6),
        "JavaScript" => (0xf1, 0xe0, 0x5a),
        "MDX" => (0xfc, 0xb3, 0x2c),
        "TeX" => (0x3d, 0x61, 0x17),
        "CSS" => (0x66, 0x33, 0x99),
        "SCSS" => (0xc6, 0x53, 0x8c),
        "Less" => (0x1d, 0x36, 0x5d),
        "HTML" => (0xe3, 0x4c, 0x26),
        "Vue" => (0x41, 0xb8, 0x83),
        "Svelte" => (0xff, 0x3e, 0x00),
        "Python" => (0x35, 0x72, 0xa5),
        "Go" => (0x00, 0xad, 0xd8),
        "Ruby" => (0x70, 0x15, 0x16),
        "Java" => (0xb0, 0x72, 0x19),
        "Kotlin" => (0xa9, 0x7b, 0xff),
        "Swift" => (0xf0, 0x51, 0x38),
        "C" => (0x55, 0x55, 0x55),
        "C++" => (0xf3, 0x4b, 0x7d),
        "C#" => (0x17, 0x86, 0x00),
        "PHP" => (0x4f, 0x5d, 0x95),
        "Shell" => (0x89, 0xe0, 0x51),
        "Lua" => (0x00, 0x00, 0x80),
        "Nix" => (0x7e, 0x7e, 0xff),
        "Dart" => (0x00, 0xb4, 0xab),
        "Scala" => (0xc2, 0x2d, 0x40),
        "OCaml" => (0xef, 0x7a, 0x08),
        "Haskell" => (0x5e, 0x50, 0x86),
        "Elixir" => (0x6e, 0x4a, 0x7e),
        "Dockerfile" => (0x38, 0x4d, 0x54),
        "Makefile" => (0x42, 0x78, 0x19),
        _ => (0x80, 0x80, 0x80),
    }
}

/// Print a standalone titled language breakdown (used for the all-projects total).
fn print_breakdown(title: &str, title_color: Color, langs: &[(&'static str, u64)]) {
    let total: u64 = langs.iter().map(|(_, b)| b).sum();
    println!(
        "{}  {}",
        bold(&paint(title, title_color)),
        paint(&format!("({})", human_bytes(total)), Color::Dim),
    );
    print_languages(langs);
    println!();
}

/// Format a byte count as a compact, human-readable string (`B`/`KiB`/`MiB`/…).
fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = n as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{n} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_counts_programming_and_markup_only() {
        assert_eq!(classify(Path::new("a/b.rs")), Some("Rust"));
        assert_eq!(classify(Path::new("a/b.tsx")), Some("TypeScript"));
        assert_eq!(classify(Path::new("docs/post.mdx")), Some("MDX"));
        assert_eq!(classify(Path::new("Dockerfile")), Some("Dockerfile"));
        assert_eq!(
            classify(Path::new("svc/Dockerfile.prod")),
            Some("Dockerfile")
        );
        // Prose and data are excluded, the way Linguist's language bar excludes them.
        assert_eq!(classify(Path::new("README.md")), None);
        assert_eq!(classify(Path::new("package.json")), None);
        assert_eq!(classify(Path::new("config.yaml")), None);
        assert_eq!(classify(Path::new("Cargo.toml")), None);
        assert_eq!(classify(Path::new("noext")), None);
    }

    #[test]
    fn is_vendored_matches_any_segment() {
        assert!(is_vendored(Path::new("web/node_modules/x/index.js")));
        assert!(is_vendored(Path::new("dist/bundle.js")));
        assert!(!is_vendored(Path::new("src/main.rs")));
    }

    #[test]
    fn sorted_orders_by_bytes_then_name() {
        let map = HashMap::from([("Rust", 100), ("MDX", 100), ("TypeScript", 300)]);
        assert_eq!(
            sorted(map),
            vec![("TypeScript", 300), ("MDX", 100), ("Rust", 100)],
        );
    }

    #[test]
    fn lang_color_matches_github_and_falls_back() {
        // A known language echoes its Linguist hex (Rust = #dea584).
        assert_eq!(lang_color("Rust"), (0xde, 0xa5, 0x84));
        // Unknown languages fall back to neutral gray.
        assert_eq!(lang_color("Brainfuck"), (0x80, 0x80, 0x80));
    }

    #[test]
    fn human_bytes_scales_units() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(1024 * 1024), "1.0 MiB");
    }
}
