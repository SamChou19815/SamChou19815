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
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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

/// What running a command asks the terminal to do. `clear` and `dev-sam` are
/// the shell's two commands whose effect is the terminal's rather than a
/// string it prints.
pub enum Outcome {
    /// Styled text to print.
    Text(String),
    /// Wipe the screen and redraw the prompt at the top.
    Clear,
    /// Boot the full-screen app.
    LaunchApp,
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

    /// Every command run so far, oldest first — what `history` prints and what
    /// the line editor walks with the arrow keys.
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Runs one command line. Styled output carries no prompt; the line editor
    /// prints that itself.
    pub fn execute(&mut self, line: &str) -> Outcome {
        let line = line.trim();
        if line.is_empty() {
            return Outcome::Text(String::new());
        }
        self.history.push(line.to_string());
        let mut words = line.split_whitespace();
        let command = words.next().unwrap_or_default();
        let args: Vec<&str> = words.collect();
        match command {
            "clear" => Outcome::Clear,
            "dev-sam" => Outcome::LaunchApp,
            "help" => Outcome::Text(self.help()),
            "ls" => Outcome::Text(self.ls(&args)),
            "cat" => Outcome::Text(self.cat(&args)),
            "cd" => Outcome::Text(self.cd(&args)),
            "pwd" => Outcome::Text(self.pwd()),
            "echo" => Outcome::Text(format!("{}\n", args.join(" "))),
            "whoami" => Outcome::Text("sam\n".to_string()),
            "history" => Outcome::Text(self.print_history()),
            _ => Outcome::Text(self.unknown(command)),
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

// --- The line editor ---------------------------------------------------------

/// Cells the prompt occupies: `sam@developersam:~$ `. Fixed, so the cursor can
/// be placed by column without measuring escape sequences.
const PROMPT_WIDTH: usize = 20;

/// The shell prompt, in the site's colours — the same ones the xterm theme maps
/// its green, blue and bright black to.
fn prompt() -> String {
    format!(
        "{}{}{}{}",
        paint(Style::new().fg(theme::STRING).bold(), "sam@developersam"),
        paint(Style::new().fg(theme::BORDER), ":"),
        paint(Style::new().fg(theme::KEYWORD), "~"),
        paint(Style::new().fg(theme::BORDER), "$ "),
    )
}

/// The editable line at the prompt. Keeps the buffer and the cursor; history
/// lives on the [`Shell`], which is the thing that records it, so the `history`
/// command and the arrow keys can never drift apart.
pub struct LineEditor {
    line: String,
    /// Byte offset of the cursor within `line`.
    cursor: usize,
    /// Where the arrow keys are in the shell's history. Equal to its length
    /// while a fresh line is being typed.
    history_index: usize,
}

impl Default for LineEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl LineEditor {
    pub fn new() -> Self {
        LineEditor {
            line: String::new(),
            cursor: 0,
            history_index: 0,
        }
    }

    /// The greeting a fresh session opens with, `dev-sam` pre-typed so a
    /// visitor only has to press Enter. `touch` drops the keyboard hint, which
    /// a phone cannot act on.
    pub fn banner(&mut self, touch: bool) -> String {
        let mut out = paint(
            Style::new().fg(theme::MUTED),
            "dev-sam-sh 1.0 — developer sam's terminal",
        );
        out.push_str("\r\n");
        if !touch {
            out.push_str(&paint(
                Style::new().fg(theme::MUTED),
                "type help for commands, or run dev-sam",
            ));
            out.push_str("\r\n");
        }
        out.push_str("\r\n");
        self.set_line("dev-sam".to_string());
        out.push_str(&self.render());
        out
    }

    /// The prompt the visitor lands back on after the app exits.
    pub fn resume(&mut self) -> String {
        self.set_line(String::new());
        format!(
            "{}\r\n{}",
            paint(
                Style::new().fg(theme::MUTED),
                "dev-sam exited — type dev-sam to run it again, or help",
            ),
            prompt(),
        )
    }

    /// Feeds one key. Returns the ANSI to write and whether the app should boot.
    pub fn key(&mut self, key: KeyEvent, shell: &mut Shell) -> (String, bool) {
        // A key reports twice where the keyboard enhancement flags are
        // supported; act on the press, as the app does.
        if key.kind == KeyEventKind::Release {
            return (String::new(), false);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return (self.control(key.code), false);
        }
        match key.code {
            KeyCode::Enter => return self.submit(shell),
            KeyCode::Tab => return (self.complete(shell), false),
            KeyCode::Up => self.recall(shell, -1),
            KeyCode::Down => self.recall(shell, 1),
            KeyCode::Left => {
                if let Some((offset, _)) = self.line[..self.cursor].char_indices().next_back() {
                    self.cursor = offset;
                }
            }
            KeyCode::Right => {
                if let Some(character) = self.line[self.cursor..].chars().next() {
                    self.cursor += character.len_utf8();
                }
            }
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.line.len(),
            KeyCode::Backspace => {
                if let Some((offset, _)) = self.line[..self.cursor].char_indices().next_back() {
                    self.line.remove(offset);
                    self.cursor = offset;
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.line.len() {
                    self.line.remove(self.cursor);
                }
            }
            KeyCode::Char(character) => {
                self.line.insert(self.cursor, character);
                self.cursor += character.len_utf8();
            }
            _ => return (String::new(), false),
        }
        (self.render(), false)
    }

    fn control(&mut self, code: KeyCode) -> String {
        match code {
            KeyCode::Char('c') => {
                self.set_line(String::new());
                format!("^C\r\n{}", prompt())
            }
            // Wipe the screen but keep whatever was being typed, as a shell does.
            KeyCode::Char('l') => format!("\x1b[2J\x1b[H{}", self.render_from_start()),
            KeyCode::Char('a') => {
                self.cursor = 0;
                self.render()
            }
            KeyCode::Char('e') => {
                self.cursor = self.line.len();
                self.render()
            }
            KeyCode::Char('u') => {
                self.line.drain(..self.cursor);
                self.cursor = 0;
                self.render()
            }
            _ => String::new(),
        }
    }

    fn submit(&mut self, shell: &mut Shell) -> (String, bool) {
        let line = std::mem::take(&mut self.line);
        self.cursor = 0;
        let outcome = shell.execute(&line);
        self.history_index = shell.history().len();
        match outcome {
            Outcome::LaunchApp => ("\r\n".to_string(), true),
            Outcome::Clear => (format!("\x1b[2J\x1b[H{}", prompt()), false),
            Outcome::Text(text) => {
                // The shell writes bare newlines; a terminal in raw mode needs
                // the carriage return too.
                let body = text.trim_end_matches('\n').replace('\n', "\r\n");
                (format!("\r\n{body}\r\n{}", prompt()), false)
            }
        }
    }

    fn complete(&mut self, shell: &Shell) -> String {
        let candidates = shell.complete(&self.line);
        let candidates: Vec<&str> = if candidates.is_empty() {
            Vec::new()
        } else {
            candidates.lines().collect()
        };
        match candidates.as_slice() {
            [] => String::new(),
            // Completion works on the last word of the whole line, so it also
            // lands the cursor at the end.
            [only] => {
                let start = self.line.rfind(' ').map_or(0, |position| position + 1);
                let suffix = if only.ends_with('/') { "" } else { " " };
                self.set_line(format!("{}{only}{suffix}", &self.line[..start]));
                self.render()
            }
            many => format!("\r\n{}\r\n{}", many.join("   "), self.render()),
        }
    }

    /// Walks the shell's history. Past the newest entry is the fresh line the
    /// visitor was typing, which is why the index runs one past the end.
    fn recall(&mut self, shell: &Shell, direction: isize) {
        let history = shell.history();
        let at_oldest = direction < 0 && self.history_index == 0;
        let next = self.history_index.saturating_add_signed(direction);
        if at_oldest || next > history.len() {
            return;
        }
        self.history_index = next;
        self.set_line(history.get(next).cloned().unwrap_or_default());
    }

    fn set_line(&mut self, line: String) {
        self.cursor = line.len();
        self.line = line;
    }

    /// Repaints the prompt row and parks the cursor where it belongs.
    fn render(&self) -> String {
        format!("\r\x1b[K{}", self.render_from_start())
    }

    fn render_from_start(&self) -> String {
        let column = PROMPT_WIDTH + self.line[..self.cursor].chars().count() + 1;
        format!("{}{}\x1b[{column}G", prompt(), self.line)
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
