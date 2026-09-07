use crate::data;
use crate::highlight;
use crate::theme;
use crossterm::cursor::{MoveTo, MoveToColumn};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::Stylize;
use crossterm::terminal::{Clear, ClearType};
use iocraft::components::MixedTextContent;
use iocraft::prelude::*;

pub const COMMANDS: &[&str] = &[
    "cat", "cd", "clear", "dev-sam", "echo", "help", "history", "ls", "pwd", "whoami",
];

/// The flag that asks `dev-sam` for the touch build. The host pre-types it on a
/// phone, where it is the only way the app is ever run.
pub const TOUCH_FLAG: &str = "--touch";

pub struct Shell {
    /// `[]` = `/home/sam`
    cwd_segments: Vec<String>,
    history: Vec<String>,
}

pub enum CommandRunOutcome {
    RenderText(String),
    Clear,
    LaunchApp(Launch),
}

/// How `dev-sam` was asked to run.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Launch {
    /// `--touch`: the build for a host that has no keyboard and scrolls the
    /// terminal itself. It draws the same view, but whole, into the scrollback
    /// rather than taking the screen over — see [`crate::view::touch_element`].
    pub touch: bool,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            cwd_segments: Vec::new(),
            history: Vec::new(),
        }
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn execute(&mut self, line: &str) -> CommandRunOutcome {
        let line = line.trim();
        if line.is_empty() {
            return CommandRunOutcome::RenderText(String::new());
        }
        self.history.push(line.to_string());
        let mut words = line.split_whitespace();
        let command = words.next().unwrap_or_default();
        let args: Vec<&str> = words.collect();
        match command {
            "clear" => CommandRunOutcome::Clear,
            "dev-sam" => CommandRunOutcome::LaunchApp(Launch {
                touch: args.contains(&TOUCH_FLAG),
            }),
            "help" => CommandRunOutcome::RenderText(self.help()),
            "ls" => CommandRunOutcome::RenderText(self.ls(&args)),
            "cat" => CommandRunOutcome::RenderText(self.cat(&args)),
            "cd" => CommandRunOutcome::RenderText(self.cd(&args)),
            "pwd" => CommandRunOutcome::RenderText(self.pwd()),
            "echo" => CommandRunOutcome::RenderText(format!("{}\n", args.join(" "))),
            "whoami" => CommandRunOutcome::RenderText("sam\n".to_string()),
            "history" => CommandRunOutcome::RenderText(self.print_history()),
            _ => CommandRunOutcome::RenderText(self.render_unknown_command(command)),
        }
    }

    pub fn tab_complete(&self, line: &str) -> String {
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
                "{}{}\n",
                format!("  {name:<12}").with(theme::ACCENT_TEXT).bold(),
                description.with(theme::TEXT),
            ));
        }
        out
    }

    fn ls(&self, args: &[&str]) -> String {
        let target = match args.first() {
            None => self.cwd_segments.clone(),
            Some(&arg) => match self.resolve_path(arg) {
                Ok(path) => path,
                Err(error) => return error,
            },
        };
        match fs_entries(&target) {
            Some(items) => {
                let mut out = String::new();
                for (name, directory) in items {
                    if directory {
                        out.push_str(
                            &format!("{name:<16}")
                                .with(theme::ACCENT_TEXT)
                                .bold()
                                .to_string(),
                        );
                    } else {
                        out.push_str(&format!("{name:<16}").with(theme::TEXT).to_string());
                    }
                }
                format!("{}\n", out.trim_end())
            }
            None => format!(
                "{}\n",
                format!("ls: no such directory: {}", args[0]).with(theme::FUNCTION)
            ),
        }
    }

    fn cat(&self, args: &[&str]) -> String {
        let Some(arg) = args.first() else {
            return format!("{}\n", "usage: cat <file>".with(theme::FUNCTION));
        };
        let path = match self.resolve_path(arg) {
            Ok(path) => path,
            Err(error) => return error,
        };
        match read_file(&path) {
            Some(content) => format!("{content}\n"),
            None => format!(
                "{}\n",
                format!("cat: no such file: {arg}").with(theme::FUNCTION)
            ),
        }
    }

    fn cd(&mut self, args: &[&str]) -> String {
        let path = match args.first() {
            None => Vec::new(),
            Some(&arg) => match self.resolve_path(arg) {
                Ok(path) => path,
                Err(error) => return error,
            },
        };
        if fs_entries(&path).is_none() {
            return format!(
                "{}\n",
                format!("cd: not a directory: {}", args[0]).with(theme::FUNCTION)
            );
        }
        self.cwd_segments = path;
        String::new()
    }

    fn pwd(&self) -> String {
        if self.cwd_segments.is_empty() {
            return "/home/sam\n".to_string();
        }
        format!("/home/sam/{}\n", self.cwd_segments.join("/"))
    }

    fn print_history(&self) -> String {
        let mut out = String::new();
        for (index, entry) in self.history.iter().enumerate() {
            out.push_str(&format!(
                "{} {}\n",
                format!("{:>4}", index + 1).with(theme::MUTED),
                entry
            ));
        }
        out
    }

    fn render_unknown_command(&self, command: &str) -> String {
        let mut out = format!(
            "{}\n",
            format!("dev-sam-sh: command not found: {command}").with(theme::FUNCTION)
        );

        fn suggest(command: &str) -> Option<&'static str> {
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

            let mut best: Option<(usize, &'static str)> = None;
            for name in COMMANDS {
                let distance = levenshtein(command, name);
                if distance <= 2 && best.is_none_or(|(best_distance, _)| distance < best_distance) {
                    best = Some((distance, name));
                }
            }
            best.map(|(_, name)| name)
        }

        if let Some(suggestion) = suggest(command) {
            out.push_str(
                &format!("did you mean `{suggestion}`? try help\n")
                    .with(theme::MUTED)
                    .to_string(),
            );
        }
        out
    }

    fn complete_path(&self, word: &str) -> Vec<String> {
        let (base, segment) = match word.rfind('/') {
            Some(position) => (&word[..position + 1], &word[position + 1..]),
            None => ("", word),
        };
        let mut base_path = self.cwd_segments.clone();
        for part in base
            .split('/')
            .filter(|part| !part.is_empty() && *part != ".")
        {
            if part == ".." {
                base_path.pop();
            } else if part == "projects" && fs_entries(&base_path).is_some() {
                base_path.push("projects".to_string());
            } else {
                return Vec::new();
            }
        }
        fs_entries(&base_path)
            .into_iter()
            .flatten()
            .filter(|(name, _)| name.starts_with(segment))
            .map(|(name, _)| format!("{base}{name}"))
            .collect()
    }

    fn resolve_path(&self, arg: &str) -> Result<Vec<String>, String> {
        let mut path = self.cwd_segments.clone();
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

fn render_prompt() -> String {
    format!(
        "{}{}{}{}",
        "sam@developersam".with(theme::STRING).bold(),
        ":".with(theme::BORDER),
        "~".with(theme::KEYWORD),
        "$ ".with(theme::BORDER),
    )
}

enum HistoryStep {
    Older,
    Newer,
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

    pub fn opening_screen(
        &mut self,
        // `touch` drops the keyboard hint, which a phone cannot act on, and
        // pre-types the flag that asks for the build a phone can read.
        touch: bool,
    ) -> String {
        let mut out = "dev-sam-sh 1.0 — developer sam's terminal"
            .with(theme::MUTED)
            .to_string();
        out.push_str("\r\n");
        if !touch {
            out.push_str(
                &"type help for commands, or run dev-sam"
                    .with(theme::MUTED)
                    .to_string(),
            );
            out.push_str("\r\n");
        }
        out.push_str("\r\n");
        // Pre-typed so a visitor only has to press Enter — and on a phone,
        // where nothing can press it, so the host has something to submit.
        self.set_line(if touch {
            format!("dev-sam {TOUCH_FLAG}")
        } else {
            "dev-sam".to_string()
        });
        out.push_str(&self.render_prompt_row());
        out
    }

    pub fn after_dev_sam_app_exit(&mut self) -> String {
        self.set_line(String::new());
        format!(
            "{}\r\n{}",
            "dev-sam exited — type dev-sam to run it again, or help".with(theme::MUTED),
            render_prompt(),
        )
    }

    /// Feeds one key. Returns the ANSI to write and how the app should boot, if
    /// this key is what ran it.
    pub fn handle_key(&mut self, key: KeyEvent, shell: &mut Shell) -> (String, Option<Launch>) {
        // A key reports twice where the keyboard enhancement flags are
        // supported; act on the press, as the app does.
        if key.kind == KeyEventKind::Release {
            return (String::new(), None);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return (self.handle_control_key(key.code), None);
        }
        match key.code {
            KeyCode::Enter => return self.on_command_submit(shell),
            KeyCode::Tab => return (self.complete(shell), None),
            KeyCode::Up => self.step_history(shell, HistoryStep::Older),
            KeyCode::Down => self.step_history(shell, HistoryStep::Newer),
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
            _ => return (String::new(), None),
        }
        (self.render_prompt_row(), None)
    }

    fn handle_control_key(&mut self, code: KeyCode) -> String {
        match code {
            KeyCode::Char('c') => {
                self.set_line(String::new());
                format!("^C\r\n{}", render_prompt())
            }
            // Wipe the screen but keep whatever was being typed, as a shell does.
            KeyCode::Char('l') => {
                format!(
                    "{}{}{}",
                    Clear(ClearType::All),
                    MoveTo(0, 0),
                    self.render_prompt_row()
                )
            }
            KeyCode::Char('a') => {
                self.cursor = 0;
                self.render_prompt_row()
            }
            KeyCode::Char('e') => {
                self.cursor = self.line.len();
                self.render_prompt_row()
            }
            KeyCode::Char('u') => {
                self.line.drain(..self.cursor);
                self.cursor = 0;
                self.render_prompt_row()
            }
            _ => String::new(),
        }
    }

    fn on_command_submit(&mut self, shell: &mut Shell) -> (String, Option<Launch>) {
        let line = std::mem::take(&mut self.line);
        self.cursor = 0;
        let outcome = shell.execute(&line);
        self.history_index = shell.history().len();
        match outcome {
            CommandRunOutcome::LaunchApp(launch) => ("\r\n".to_string(), Some(launch)),
            CommandRunOutcome::Clear => (
                format!(
                    "{}{}{}",
                    Clear(ClearType::All),
                    MoveTo(0, 0),
                    render_prompt()
                ),
                None,
            ),
            CommandRunOutcome::RenderText(text) => {
                // The shell writes bare newlines; a terminal in raw mode needs
                // the carriage return too.
                let body = text.trim_end_matches('\n').replace('\n', "\r\n");
                (format!("\r\n{body}\r\n{}", render_prompt()), None)
            }
        }
    }

    fn complete(&mut self, shell: &Shell) -> String {
        let candidates = shell.tab_complete(&self.line);
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
                self.render_prompt_row()
            }
            many => format!("\r\n{}\r\n{}", many.join("   "), self.render_prompt_row()),
        }
    }

    fn step_history(&mut self, shell: &Shell, step: HistoryStep) {
        let history = shell.history();
        let next = match step {
            HistoryStep::Older => self.history_index.saturating_sub(1),
            HistoryStep::Newer => (self.history_index + 1).min(history.len()),
        };
        self.history_index = next;
        self.set_line(history.get(next).cloned().unwrap_or_default());
    }

    fn set_line(&mut self, line: String) {
        self.cursor = line.len();
        self.line = line;
    }

    fn render_prompt_row(&self) -> String {
        let column = PROMPT_WIDTH + self.line[..self.cursor].chars().count();
        format!(
            "\r{}{}{}{}",
            Clear(ClearType::UntilNewLine),
            render_prompt(),
            self.line,
            MoveToColumn(column as u16)
        )
    }
}

/// The virtual file system rooted at `/home/sam`.
fn fs_entries(path: &[String]) -> Option<Vec<(String, bool)>> {
    if path.is_empty() {
        return Some(
            [
                ("projects/", true),
                ("about.txt", false),
                ("contact.txt", false),
                ("readme.md", false),
                ("resume.pdf", false),
                ("timeline.txt", false),
            ]
            .map(|(name, directory)| (name.to_string(), directory))
            .to_vec(),
        );
    }
    if path.len() == 1 && path[0] == "projects" {
        return Some(
            data::PROJECTS
                .iter()
                .map(|project| (project.id.decrypt(), false))
                .collect(),
        );
    }
    None
}

fn render_link(text: &str, url: &str) -> String {
    format!("\x1b]8;;{url}\x07{text}\x1b]8;;\x07")
}

/// Renders the highlighter's spans as one ANSI line.
fn spans_line(contents: &[MixedTextContent]) -> String {
    let mut out = String::new();
    for piece in contents {
        let mut styled = piece.text.as_str().stylize();
        if let Some(color) = piece.color {
            styled = styled.with(color);
        }
        if piece.weight == Weight::Bold {
            styled = styled.bold();
        }
        if piece.italic {
            styled = styled.italic();
        }
        if piece.decoration == TextDecoration::Underline {
            styled = styled.underlined();
        }
        out.push_str(&styled.to_string());
    }
    out
}

fn read_file(path: &[String]) -> Option<String> {
    let path: Vec<&str> = path.iter().map(|segment| segment.as_str()).collect();
    match path.as_slice() {
        ["readme.md"] => Some(
            [
                "Served by Rust, compiled to WASM"
                    .with(theme::SUBTLE)
                    .to_string(),
                String::new(),
                "run dev-sam to launch the app, or explore:"
                    .with(theme::TEXT)
                    .to_string(),
                "  cat about.txt".with(theme::ACCENT_TEXT).to_string(),
                "  ls projects".with(theme::ACCENT_TEXT).to_string(),
                "  cat timeline.txt".with(theme::ACCENT_TEXT).to_string(),
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
                    Some(url) => render_link(&rendered, url),
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
                let url = entry.url.decrypt();
                out.push_str(&format!(
                    "{} {}\n",
                    format!("@{}:", entry.name).with(theme::ACCENT_TEXT).bold(),
                    render_link(&url, &url),
                ));
            }
            Some(out)
        }
        ["contact.txt"] => {
            let mut out = String::new();
            for entry in data::SOCIAL_LINKS {
                let url = entry.url.decrypt();
                out.push_str(&format!(
                    "{} {}\n",
                    format!("{:<10}", entry.name)
                        .with(theme::ACCENT_TEXT)
                        .bold(),
                    render_link(&url, &url),
                ));
            }
            Some(out)
        }
        ["resume.pdf"] => Some(format!(
            "resume.pdf: PDF document\n{} {}\n",
            "open:".with(theme::MUTED),
            render_link(
                "https://developersam.com/resume.pdf",
                "https://developersam.com/resume.pdf"
            ),
        )),
        ["timeline.txt"] => {
            let mut out = String::new();
            for (index, event) in data::TIMELINE.iter().enumerate() {
                out.push_str(&format!(
                    "{}{}{}\n",
                    format!("{:>3}  {:<15}", index + 1, event.time).with(theme::MUTED),
                    event.title.decrypt().with(theme::TEXT),
                    format!(" [{}]", event.category.label()).with(event.category.color()),
                ));
            }
            Some(out)
        }
        ["projects", name] => {
            let project = data::PROJECTS
                .iter()
                .find(|project| project.id.decrypt() == *name)?;
            let mut out = format!(
                "{}\n{}\n\n",
                project.id.decrypt().with(theme::ACCENT_TEXT).bold(),
                project.tagline.decrypt().with(theme::SUBTLE),
            );
            for entry in project.links {
                let url = entry.url.decrypt();
                out.push_str(&format!(
                    "  {} {}\n",
                    format!("{:<12}", entry.name)
                        .with(theme::ACCENT_TEXT)
                        .bold(),
                    render_link(&url, &url),
                ));
            }
            Some(out)
        }
        _ => None,
    }
}
