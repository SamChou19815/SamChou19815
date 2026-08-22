//! The `dev-sam-sh` shell: what the web terminal boots into. The app is just
//! one command (`dev-sam`) away, and everything else explores the same static
//! site data the TUI renders — a small virtual filesystem served from
//! [`crate::data`], with real `ls`/`cat`/`cd` and Tab completion.
//!
//! Output is ANSI text (styled through [`crate::theme`] and the Prism
//! highlighter), printed verbatim by xterm.js. Links use OSC 8 so they are
//! clickable without a link-region table.

use crate::data;
use crate::highlight;
use crate::theme;
use crossterm::style::Color;
use iocraft::components::MixedTextContent;
use iocraft::prelude::*;

pub const COMMANDS: &[&str] = &[
    "cat", "cd", "clear", "dev-sam", "echo", "help", "history", "ls", "pwd", "whoami",
];

pub struct Shell {
    /// Working directory as segments (`[]` = `/home/sam`).
    path: Vec<String>,
    history: Vec<String>,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            path: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Runs one command line and returns styled output (no prompt; the
    /// terminal prints the prompt itself).
    pub fn execute(&mut self, line: &str) -> String {
        let line = line.trim();
        if line.is_empty() {
            return String::new();
        }
        self.history.push(line.to_string());
        let mut words = line.split_whitespace();
        let command = words.next().unwrap_or_default();
        let args: Vec<&str> = words.collect();
        match command {
            "help" => self.help(),
            "ls" => self.ls(&args),
            "cat" => self.cat(&args),
            "cd" => self.cd(&args),
            "pwd" => self.pwd(),
            "echo" => format!("{}\n", args.join(" ")),
            "whoami" => "sam\n".to_string(),
            "history" => self.print_history(),
            "clear" | "dev-sam" => String::new(), // handled by the terminal itself
            _ => self.unknown(command),
        }
    }

    /// Tab-completion candidates for the last word of the line, one per line.
    pub fn complete(&self, line: &str) -> String {
        let trimmed = line.trim_start();
        let (before, word) = match trimmed.rfind(' ') {
            Some(position) => (&trimmed[..position], trimmed[position + 1..].trim_start()),
            None => ("", trimmed),
        };
        if word.is_empty() {
            return String::new();
        }
        let first = before.split_whitespace().next().unwrap_or("");
        let candidates: Vec<String> = if before.trim().is_empty() {
            COMMANDS
                .iter()
                .filter(|candidate| candidate.starts_with(word))
                .map(|candidate| (*candidate).to_string())
                .collect()
        } else if matches!(first, "ls" | "cd" | "cat") {
            self.complete_path(word)
        } else {
            Vec::new()
        };
        candidates.join("\n")
    }

    fn help(&self) -> String {
        let mut out = String::new();
        for (name, description) in [
            (
                "dev-sam",
                "launch the developer sam app (q exits back here)",
            ),
            ("ls [dir]", "list the file system"),
            ("cat <file>", "print a file (try cat about.txt)"),
            ("cd <dir>", "change directory"),
            ("pwd", "print working directory"),
            ("echo <text>", "print text"),
            ("whoami", "print the user"),
            ("history", "command history (also ↑/↓)"),
            ("clear", "clear the screen (Ctrl+L)"),
            ("help", "this message"),
        ] {
            out.push_str(&format!(
                "{}{}{}{}\n",
                paint(
                    Style::new().fg(theme::ACCENT_TEXT).bold(),
                    &format!("  {name:<12}")
                ),
                reset(),
                paint(Style::new().fg(theme::TEXT), description),
                reset(),
            ));
        }
        out
    }

    fn ls(&self, args: &[&str]) -> String {
        let target = match args.first() {
            None => self.path.clone(),
            Some(&arg) => match self.resolve(arg) {
                Ok(path) => path,
                Err(error) => return error,
            },
        };
        match entries(&target) {
            Some(items) => {
                let mut out = String::new();
                for (name, directory) in items {
                    if directory {
                        out.push_str(&paint(
                            Style::new().fg(theme::ACCENT_TEXT).bold(),
                            &format!("{name:<16}"),
                        ));
                    } else {
                        out.push_str(&paint(Style::new().fg(theme::TEXT), &format!("{name:<16}")));
                    }
                }
                format!("{}\n", out.trim_end())
            }
            None => format!(
                "{}\n",
                paint(
                    Style::new().fg(theme::FUNCTION),
                    &format!("ls: no such directory: {}", args[0])
                )
            ),
        }
    }

    fn cat(&self, args: &[&str]) -> String {
        let Some(arg) = args.first() else {
            return format!(
                "{}\n",
                paint(Style::new().fg(theme::FUNCTION), "usage: cat <file>")
            );
        };
        let path = match self.resolve(arg) {
            Ok(path) => path,
            Err(error) => return error,
        };
        match read_file(&path) {
            Some(content) => format!("{content}\n"),
            None => format!(
                "{}\n",
                paint(
                    Style::new().fg(theme::FUNCTION),
                    &format!("cat: no such file: {arg}")
                )
            ),
        }
    }

    fn cd(&mut self, args: &[&str]) -> String {
        let path = match args.first() {
            None => Vec::new(),
            Some(&arg) => match self.resolve(arg) {
                Ok(path) => path,
                Err(error) => return error,
            },
        };
        if entries(&path).is_none() {
            return format!(
                "{}\n",
                paint(
                    Style::new().fg(theme::FUNCTION),
                    &format!("cd: not a directory: {}", args[0])
                )
            );
        }
        self.path = path;
        String::new()
    }

    fn pwd(&self) -> String {
        if self.path.is_empty() {
            return "/home/sam\n".to_string();
        }
        format!("/home/sam/{}\n", self.path.join("/"))
    }

    fn print_history(&self) -> String {
        let mut out = String::new();
        for (index, entry) in self.history.iter().enumerate() {
            out.push_str(&format!(
                "{}{} {}\n",
                paint(Style::new().fg(theme::MUTED), &format!("{:>4}", index + 1)),
                reset(),
                entry
            ));
        }
        out
    }

    fn unknown(&self, command: &str) -> String {
        let mut out = format!(
            "{}\n",
            paint(
                Style::new().fg(theme::FUNCTION),
                &format!("dev-sam-sh: command not found: {command}")
            )
        );
        if let Some(suggestion) = suggest(command) {
            out.push_str(&paint(
                Style::new().fg(theme::MUTED),
                &format!("did you mean `{suggestion}`? try help\n"),
            ));
        }
        out
    }

    /// Completes the final segment of a path word (e.g. `projects/sa`).
    /// Directories complete with their trailing "/" so chained completion
    /// works (`cd proj<TAB> → cd projects/`).
    fn complete_path(&self, word: &str) -> Vec<String> {
        let (base, segment) = match word.rfind('/') {
            Some(position) => (&word[..position + 1], &word[position + 1..]),
            None => ("", word),
        };
        let mut base_path = self.path.clone();
        for part in base
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
        {
            if part == ".." {
                base_path.pop();
            } else if part == "projects" && entries(&base_path).is_some() {
                base_path.push("projects".to_string());
            } else {
                return Vec::new();
            }
        }
        entries(&base_path)
            .into_iter()
            .flatten()
            .filter(|(name, _)| name.starts_with(segment))
            .map(|(name, _)| format!("{base}{name}"))
            .collect()
    }

    /// Resolves an argument against the working directory; validity of the
    /// final segment is checked by the caller (`ls`/`cd` need directories,
    /// `cat` needs files).
    fn resolve(&self, arg: &str) -> Result<Vec<String>, String> {
        let mut path = self.path.clone();
        for part in arg.split('/').filter(|part| !part.is_empty()) {
            match part {
                "." => {}
                ".." => {
                    path.pop();
                }
                _ => path.push(part.to_string()),
            }
        }
        Ok(path)
    }
}

/// The virtual file system rooted at `/home/sam`.
fn entries(path: &[String]) -> Option<Vec<(&'static str, bool)>> {
    if path.is_empty() {
        return Some(vec![
            ("projects/", true),
            ("about.txt", false),
            ("contact.txt", false),
            ("readme.md", false),
            ("resume.pdf", false),
            ("timeline.txt", false),
        ]);
    }
    if path.len() == 1 && path[0] == "projects" {
        return Some(
            data::PROJECTS
                .iter()
                .map(|project| (project.id, false))
                .collect(),
        );
    }
    None
}

fn link(text: &str, url: &str) -> String {
    format!("\x1b]8;;{url}\x07{text}\x1b]8;;\x07")
}

/// Renders the highlighter's spans as one ANSI line.
fn spans_line(contents: &[MixedTextContent]) -> String {
    let mut out = String::new();
    for piece in contents {
        let style = Style {
            fg: piece.color,
            bg: None,
            bold: piece.weight == Weight::Bold,
            italic: piece.italic,
            underline: piece.decoration == TextDecoration::Underline,
        };
        out.push_str(&style_sgr(style));
        out.push_str(&piece.text);
        out.push_str("\x1b[m");
    }
    out
}

fn read_file(path: &[String]) -> Option<String> {
    let path: Vec<&str> = path.iter().map(|segment| segment.as_str()).collect();
    match path.as_slice() {
        ["readme.md"] => Some(
            [
                paint(
                    Style::new().fg(theme::ACCENT_TEXT).bold(),
                    "developer sam's home directory",
                ),
                paint(
                    Style::new().fg(theme::SUBTLE),
                    "served by rust, compiled to wasm",
                ),
                String::new(),
                paint(
                    Style::new().fg(theme::TEXT),
                    "run dev-sam to launch the app, or explore:",
                ),
                paint(Style::new().fg(theme::ACCENT_TEXT), "  cat about.txt"),
                paint(Style::new().fg(theme::ACCENT_TEXT), "  ls projects"),
                paint(Style::new().fg(theme::ACCENT_TEXT), "  cat timeline.txt"),
            ]
            .join("\n"),
        ),
        ["about.txt"] => {
            let mut out = String::new();
            for line in highlight::doc_comment_lines() {
                // The `@` tags carry a URL; make them the hyperlinks a terminal
                // can open, as they are anchors on the homepage's docblock.
                let rendered = spans_line(&line.contents);
                out.push_str(&match &line.link {
                    Some(url) => link(&rendered, url),
                    None => rendered,
                });
                out.push('\n');
            }
            out.push('\n');
            for line in highlight::program_lines() {
                out.push_str(&spans_line(&line));
                out.push('\n');
            }
            out.push('\n');
            for entry in data::ABOUT_DOC_LINKS {
                out.push_str(&format!(
                    "{} {}{}\n",
                    paint(
                        Style::new().fg(theme::ACCENT_TEXT).bold(),
                        &format!("@{}:", entry.name)
                    ),
                    link(entry.url, entry.url),
                    reset(),
                ));
            }
            Some(out)
        }
        ["contact.txt"] => {
            let mut out = String::new();
            for entry in data::SOCIAL_LINKS {
                out.push_str(&format!(
                    "{} {}{}\n",
                    paint(
                        Style::new().fg(theme::ACCENT_TEXT).bold(),
                        &format!("{:<10}", entry.name)
                    ),
                    link(entry.url, entry.url),
                    reset(),
                ));
            }
            Some(out)
        }
        ["resume.pdf"] => Some(format!(
            "resume.pdf: PDF document\n{} {}\n",
            paint(Style::new().fg(theme::MUTED), "open:"),
            link(
                "https://developersam.com/resume.pdf",
                "https://developersam.com/resume.pdf"
            ),
        )),
        ["timeline.txt"] => {
            let mut out = String::new();
            for (index, event) in data::TIMELINE.iter().enumerate() {
                out.push_str(&format!(
                    "{}{}{}{}{}{}\n",
                    paint(
                        Style::new().fg(theme::MUTED),
                        &format!("{:>3}  {:<15}", index + 1, event.time)
                    ),
                    paint(Style::new().fg(theme::TEXT), event.title),
                    paint(
                        Style::new().fg(event.category.color()),
                        &format!(" [{}]", event.category.label())
                    ),
                    reset(),
                    "",
                    "",
                ));
            }
            Some(out)
        }
        ["projects", name] => {
            let project = data::PROJECTS.iter().find(|project| project.id == *name)?;
            let mut out = format!(
                "{}\n{}\n\n",
                paint(Style::new().fg(theme::ACCENT_TEXT).bold(), project.id),
                paint(Style::new().fg(theme::SUBTLE), project.tagline),
            );
            for entry in project.links {
                out.push_str(&format!(
                    "  {} {}{}\n",
                    paint(
                        Style::new().fg(theme::ACCENT_TEXT).bold(),
                        &format!("{:<12}", entry.name)
                    ),
                    link(entry.url, entry.url),
                    reset(),
                ));
            }
            Some(out)
        }
        _ => None,
    }
}

/// A minimal styled-text description for the shell's direct ANSI output.
#[derive(Default, Clone, Copy)]
struct Style {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    italic: bool,
    underline: bool,
}

impl Style {
    fn new() -> Self {
        Self::default()
    }

    fn fg(self, color: Color) -> Self {
        Self {
            fg: Some(color),
            ..self
        }
    }

    fn bold(self) -> Self {
        Self { bold: true, ..self }
    }
}

/// One full SGR escape for a [`Style`].
fn style_sgr(style: Style) -> String {
    let mut parts = vec![String::from("0")];
    if let Some(color) = style.fg {
        parts.push(color_code(color, false));
    }
    if let Some(color) = style.bg {
        parts.push(color_code(color, true));
    }
    if style.bold {
        parts.push(String::from("1"));
    }
    if style.italic {
        parts.push(String::from("3"));
    }
    if style.underline {
        parts.push(String::from("4"));
    }
    format!("\x1b[{}m", parts.join(";"))
}

fn color_code(color: Color, background: bool) -> String {
    let (r, g, b) = match color {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => (0, 0, 0),
    };
    if background {
        format!("48;2;{r};{g};{b}")
    } else {
        format!("38;2;{r};{g};{b}")
    }
}

fn paint(style: Style, text: &str) -> String {
    format!("{}{}\x1b[m", style_sgr(style), text)
}

fn reset() -> &'static str {
    "\x1b[m"
}

fn suggest(command: &str) -> Option<&'static str> {
    let mut best: Option<(usize, &'static str)> = None;
    for name in COMMANDS {
        let distance = levenshtein(command, name);
        if distance <= 2 && best.is_none_or(|(best_distance, _)| distance < best_distance) {
            best = Some((distance, name));
        }
    }
    best.map(|(_, name)| name)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            current.push(
                (previous[j] + cost)
                    .min(current[j] + 1)
                    .min(previous[j + 1] + 1),
            );
        }
        previous = current;
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts against visible text: strips SGR sequences and OSC 8 links.
    fn plain(output: &str) -> String {
        let mut out = String::new();
        let mut rest = output;
        while let Some(position) = rest.find('\x1b') {
            out.push_str(&rest[..position]);
            rest = &rest[position..];
            if rest.starts_with("\x1b]8;;") {
                // Skip to the link text after the second BEL.
                if let Some(start) = rest[5..].find('\x07').map(|index| index + 6) {
                    let after = &rest[start..];
                    if let Some(end) = after.find("\x1b]8;;\x07") {
                        out.push_str(&after[..end]);
                        rest = &after[end + "\x1b]8;;\x07".len()..];
                        continue;
                    }
                }
                rest = "";
            } else if rest.len() > 1 {
                let tail = &rest[1..];
                let end = tail
                    .find(|character: char| character.is_ascii_alphabetic())
                    .map(|index| index + 1)
                    .unwrap_or(tail.len());
                rest = &rest[1 + end..];
            } else {
                rest = "";
            }
        }
        out + rest
    }

    #[test]
    fn lists_home_and_projects() {
        let mut shell = Shell::new();
        let home = shell.execute("ls");
        assert!(home.contains("about.txt"));
        assert!(home.contains("projects/"));
        let projects = shell.execute("ls projects");
        assert!(projects.contains("samlang"));
        assert!(projects.contains("mini-react"));
    }

    #[test]
    fn cats_files() {
        let mut shell = Shell::new();
        let about = plain(&shell.execute("cat about.txt"));
        assert!(about.contains("import {List} from std.list;"));
        assert!(about.contains("Developer.init(github, projects)"));
        let project = plain(&shell.execute("cat projects/samlang"));
        assert!(project.contains("full type-inference"));
        let timeline = plain(&shell.execute("cat timeline.txt"));
        assert!(timeline.contains("Facebook SWE"));
        assert!(timeline.contains("Bought developersam.com"));
        let missing = shell.execute("cat nope.txt");
        assert!(missing.contains("no such file"));
    }

    #[test]
    fn navigates_directories() {
        let mut shell = Shell::new();
        assert_eq!(shell.execute("cd projects"), "");
        assert_eq!(shell.execute("pwd"), "/home/sam/projects\n");
        let listing = shell.execute("ls");
        assert!(listing.contains("samwise"));
        assert_eq!(shell.execute("cd .."), "");
        assert_eq!(shell.execute("pwd"), "/home/sam\n");
        let bad = shell.execute("cd about.txt");
        assert!(bad.contains("not a directory"));
    }

    #[test]
    fn unknown_commands_get_suggestions() {
        let mut shell = Shell::new();
        let output = shell.execute("lss");
        assert!(output.contains("command not found: lss"));
        assert!(output.contains("did you mean `ls`"));
    }

    #[test]
    fn completes_commands_and_paths() {
        let shell = Shell::new();
        assert_eq!(shell.complete("ca"), "cat");
        assert_eq!(shell.complete("c"), "cat\ncd\nclear");
        assert_eq!(shell.complete("cat a"), "about.txt");
        assert_eq!(shell.complete("ls proj"), "projects/");
        assert_eq!(
            shell.complete("cd projects/sam"),
            "projects/samlang\nprojects/samwise\nprojects/sampl"
        );
        assert_eq!(shell.complete("cd projects/saml"), "projects/samlang");
    }

    #[test]
    fn history_records_commands() {
        let mut shell = Shell::new();
        shell.execute("ls");
        shell.execute("pwd");
        let history = plain(&shell.execute("history"));
        assert!(history.contains("1 ls"));
        assert!(history.contains("2 pwd"));
        assert!(history.contains("3 history"));
    }
}
