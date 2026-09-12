//! The `dev-sam-sh` prompt: a small shell with a virtual file system, a line
//! editor, and the `dev-sam` command that hands the page over to the app.
//!
//! It prints styled lines ([`Line`]) for the front-end to draw. A line that
//! ends in a URL carries it on the run of text it is written on, so the links
//! `cat` prints are clicked where they are read.

use crate::crypt::EncryptedString;
use crate::data;
use crate::encrypted_str;
use crate::highlight;
use crate::style::{bold_colored, colored, Line, Span, TextStyle};
use crate::theme;
use crate::{Key, Mods};

// The command vocabulary, encrypted like the rest of the site's content so the
// binary spells out none of it — no `strings` pass reveals the shell's verbs.
// A match arm cannot be an [`EncryptedString`], so dispatch and completion
// compare the typed word against these decrypted.
const CMD_CAT: EncryptedString = encrypted_str!("cat");
const CMD_CD: EncryptedString = encrypted_str!("cd");
const CMD_CLEAR: EncryptedString = encrypted_str!("clear");
const CMD_DEV_SAM: EncryptedString = encrypted_str!("dev-sam");
const CMD_ECHO: EncryptedString = encrypted_str!("echo");
const CMD_HELP: EncryptedString = encrypted_str!("help");
const CMD_HISTORY: EncryptedString = encrypted_str!("history");
const CMD_LS: EncryptedString = encrypted_str!("ls");
const CMD_PWD: EncryptedString = encrypted_str!("pwd");
const CMD_WHOAMI: EncryptedString = encrypted_str!("whoami");

/// Every command name, in the order `help` prints and completion offers.
const COMMANDS: [EncryptedString; 10] = [
    CMD_CAT,
    CMD_CD,
    CMD_CLEAR,
    CMD_DEV_SAM,
    CMD_ECHO,
    CMD_HELP,
    CMD_HISTORY,
    CMD_LS,
    CMD_PWD,
    CMD_WHOAMI,
];

/// The flag that asks `dev-sam` for the touch build. The host pre-types it on a
/// phone, where it is the only way the app is ever run.
const TOUCH_FLAG: EncryptedString = encrypted_str!("--touch");

#[derive(Clone)]
pub struct Shell {
    /// `[]` = `/home/sam`
    cwd_segments: Vec<String>,
    history: Vec<String>,
}

pub enum CommandRunOutcome {
    RenderText(Vec<Line>),
    Clear,
    LaunchApp(Launch),
}

/// How `dev-sam` was asked to run.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Launch {
    /// `--touch`: the build for a host that has no keyboard and scrolls the
    /// page itself. It draws the same view, whole, for the page to scroll —
    /// see [`crate::view`].
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
            return CommandRunOutcome::RenderText(Vec::new());
        }
        self.history.push(line.to_string());
        let mut words = line.split_whitespace();
        let command = words.next().unwrap_or_default();
        let args: Vec<&str> = words.collect();
        if command == CMD_CLEAR.decrypt() {
            CommandRunOutcome::Clear
        } else if command == CMD_DEV_SAM.decrypt() {
            CommandRunOutcome::LaunchApp(Launch {
                touch: args.iter().any(|arg| *arg == TOUCH_FLAG.decrypt()),
            })
        } else if command == CMD_HELP.decrypt() {
            CommandRunOutcome::RenderText(self.help())
        } else if command == CMD_LS.decrypt() {
            CommandRunOutcome::RenderText(self.ls(&args))
        } else if command == CMD_CAT.decrypt() {
            CommandRunOutcome::RenderText(self.cat(&args))
        } else if command == CMD_CD.decrypt() {
            CommandRunOutcome::RenderText(self.cd(&args))
        } else if command == CMD_PWD.decrypt() {
            CommandRunOutcome::RenderText(self.pwd())
        } else if command == CMD_ECHO.decrypt() {
            CommandRunOutcome::RenderText(vec![line_of(args.join(" "))])
        } else if command == CMD_WHOAMI.decrypt() {
            CommandRunOutcome::RenderText(vec![line_of(encrypted_str!("sam").decrypt())])
        } else if command == CMD_HISTORY.decrypt() {
            CommandRunOutcome::RenderText(self.print_history())
        } else {
            CommandRunOutcome::RenderText(self.render_unknown_command(command))
        }
    }

    pub fn tab_complete(&self, line: &str) -> Vec<String> {
        let trimmed = line.trim_start();
        let (before, word) = match trimmed.rfind(' ') {
            Some(position) => (&trimmed[..position], trimmed[position + 1..].trim_start()),
            None => ("", trimmed),
        };
        if word.is_empty() {
            return Vec::new();
        }
        let first = before.split_whitespace().next().unwrap_or("");
        if before.trim().is_empty() {
            COMMANDS
                .iter()
                .map(EncryptedString::decrypt)
                .filter(|candidate| candidate.starts_with(word))
                .collect()
        } else if [CMD_LS, CMD_CD, CMD_CAT]
            .iter()
            .any(|command| command.decrypt() == first)
        {
            self.complete_path(word)
        } else {
            Vec::new()
        }
    }

    fn help(&self) -> Vec<Line> {
        let mut out = Vec::new();
        // The names and blurbs are decrypted here, never spelled in the binary;
        // the one line that names a file reads it from the file system too.
        let cat_hint = format!(
            "{} {ABOUT_TXT})",
            encrypted_str!("print a file (try").decrypt()
        );
        for (name, description) in [
            (
                format!("{CMD_DEV_SAM}"),
                encrypted_str!("launch the developer sam app (q exits back here)").decrypt(),
            ),
            (
                format!("{CMD_LS} [dir]"),
                encrypted_str!("list the file system").decrypt(),
            ),
            (format!("{CMD_CAT} <file>"), cat_hint),
            (
                format!("{CMD_CD} <dir>"),
                encrypted_str!("change directory").decrypt(),
            ),
            (
                format!("{CMD_PWD}"),
                encrypted_str!("print working directory").decrypt(),
            ),
            (
                format!("{CMD_ECHO} <text>"),
                encrypted_str!("print text").decrypt(),
            ),
            (
                format!("{CMD_WHOAMI}"),
                encrypted_str!("print the user").decrypt(),
            ),
            (
                format!("{CMD_HISTORY}"),
                encrypted_str!("command history (also ↑/↓)").decrypt(),
            ),
            (
                format!("{CMD_CLEAR}"),
                encrypted_str!("clear the screen (Ctrl+L)").decrypt(),
            ),
            (
                format!("{CMD_HELP}"),
                encrypted_str!("this message").decrypt(),
            ),
        ] {
            out.push(vec![
                bold_colored(format!("  {name:<12}"), theme::ACCENT_TEXT),
                colored(description, theme::TEXT),
            ]);
        }
        out
    }

    fn ls(&self, args: &[&str]) -> Vec<Line> {
        let target = match args.first() {
            None => self.cwd_segments.clone(),
            Some(&arg) => match self.resolve_path(arg) {
                Ok(path) => path,
                Err(error) => return vec![error],
            },
        };
        match fs_entries(&target) {
            Some(items) => {
                let mut contents = Vec::new();
                for (name, directory) in items {
                    if directory {
                        contents.push(bold_colored(format!("{name:<16}"), theme::ACCENT_TEXT));
                    } else {
                        contents.push(colored(format!("{name:<16}"), theme::TEXT));
                    }
                }
                // The listing ends where its last padded column ends.
                if let Some(last) = contents.last_mut() {
                    let trimmed = last.text.trim_end().to_string();
                    last.text = trimmed;
                }
                vec![contents]
            }
            None => vec![one(colored(
                format!(
                    "{}: {}",
                    encrypted_str!("ls: no such directory").decrypt(),
                    args[0]
                ),
                theme::FUNCTION,
            ))],
        }
    }

    fn cat(&self, args: &[&str]) -> Vec<Line> {
        let Some(arg) = args.first() else {
            return vec![one(colored(
                encrypted_str!("usage: cat <file>").decrypt(),
                theme::FUNCTION,
            ))];
        };
        let path = match self.resolve_path(arg) {
            Ok(path) => path,
            Err(error) => return vec![error],
        };
        match read_file(&path) {
            Some(content) => content,
            None => vec![one(colored(
                format!("{}: {arg}", encrypted_str!("cat: no such file").decrypt()),
                theme::FUNCTION,
            ))],
        }
    }

    fn cd(&mut self, args: &[&str]) -> Vec<Line> {
        let path = match args.first() {
            None => Vec::new(),
            Some(&arg) => match self.resolve_path(arg) {
                Ok(path) => path,
                Err(error) => return vec![error],
            },
        };
        if fs_entries(&path).is_none() {
            return vec![one(colored(
                format!(
                    "{}: {}",
                    encrypted_str!("cd: not a directory").decrypt(),
                    args[0]
                ),
                theme::FUNCTION,
            ))];
        }
        self.cwd_segments = path;
        Vec::new()
    }

    fn pwd(&self) -> Vec<Line> {
        if self.cwd_segments.is_empty() {
            return vec![line_of(HOME_DIR.decrypt())];
        }
        vec![line_of(format!(
            "{HOME_DIR}/{}",
            self.cwd_segments.join("/")
        ))]
    }

    fn print_history(&self) -> Vec<Line> {
        let mut out = Vec::new();
        for (index, entry) in self.history.iter().enumerate() {
            out.push(vec![
                colored(format!("{:>4}", index + 1), theme::MUTED),
                Span::new(format!(" {entry}")),
            ]);
        }
        out
    }

    fn render_unknown_command(&self, command: &str) -> Vec<Line> {
        let mut out = vec![one(colored(
            format!(
                "{}: {command}",
                encrypted_str!("dev-sam-sh: command not found").decrypt()
            ),
            theme::FUNCTION,
        ))];

        fn suggest(command: &str) -> Option<String> {
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

            let mut best: Option<(usize, String)> = None;
            for name in COMMANDS {
                let name = name.decrypt();
                let distance = levenshtein(command, &name);
                if distance <= 2
                    && best
                        .as_ref()
                        .is_none_or(|(best_distance, _)| distance < *best_distance)
                {
                    best = Some((distance, name));
                }
            }
            best.map(|(_, name)| name)
        }

        if let Some(suggestion) = suggest(command) {
            out.push(one(colored(
                format!(
                    "{} `{suggestion}`? {}",
                    encrypted_str!("did you mean").decrypt(),
                    encrypted_str!("try help").decrypt()
                ),
                theme::MUTED,
            )));
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
            } else if part == PROJECTS_DIR.decrypt() && fs_entries(&base_path).is_some() {
                base_path.push(part.to_string());
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

    fn resolve_path(&self, arg: &str) -> Result<Vec<String>, Line> {
        Ok(self.resolve_segments(arg))
    }

    fn resolve_segments(&self, arg: &str) -> Vec<String> {
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
        path
    }
}

fn line_of(text: impl Into<String>) -> Line {
    vec![Span::new(text)]
}

/// A one-span line.
fn one(span: Span) -> Line {
    vec![span]
}

/// One prompt row: the prompt itself, then the edited line split around the
/// cursor so the front-end can paint the block cursor over the character it
/// sits on (or an empty cell at the end of the line).
pub struct PromptRow {
    pub before_cursor: Vec<Span>,
    /// The character under the cursor, if any.
    pub at_cursor: Option<char>,
    pub after_cursor: Vec<Span>,
}

impl PromptRow {
    pub fn line_len(&self) -> usize {
        self.before_cursor
            .iter()
            .chain(self.after_cursor.iter())
            .map(|span| span.text.chars().count())
            .sum()
    }
}

pub enum EditOutcome {
    /// The key changed nothing on screen.
    None,
    /// The prompt row's contents moved or changed.
    Redraw,
    /// Lines to append, then a fresh prompt row.
    Output(Vec<Line>),
    /// Wipe the screen; the prompt (and whatever was being typed, for Ctrl+L)
    /// starts over at the top.
    ClearScreen,
    /// The submitted line ran `dev-sam`.
    Launch(Launch),
    /// Tab completion printed its candidates, then the prompt.
    Completion(Vec<String>),
    /// Ctrl+C: the current prompt row freezes with a `^C` echoed at the
    /// cursor — over the characters ahead of it, as a terminal prints — and a
    /// fresh empty prompt follows.
    Interrupt,
}

/// The editable line at the prompt. Keeps the buffer and the cursor; history
/// lives on the [`Shell`], which is the thing that records it, so the `history`
/// command and the arrow keys can never drift apart.
#[derive(Clone)]
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

    /// The screen the session opens on, plus the line pre-typed at the prompt.
    /// `touch` drops the keyboard hint, which a phone cannot act on, and
    /// pre-types the flag that asks for the build a phone can read.
    pub fn opening_screen(&mut self, touch: bool) -> (Vec<Line>, String) {
        let mut out = vec![one(colored(
            encrypted_str!("dev-sam-sh 1.0 — developer sam's terminal").decrypt(),
            theme::MUTED,
        ))];
        if !touch {
            out.push(one(colored(
                encrypted_str!("type help for commands, or run dev-sam").decrypt(),
                theme::MUTED,
            )));
        }
        out.push(Line::new());
        let line = if touch {
            format!("{CMD_DEV_SAM} {TOUCH_FLAG}")
        } else {
            CMD_DEV_SAM.decrypt()
        };
        self.set_line(line.clone());
        (out, line)
    }

    /// The message printed when the app has exited, and the cleared line.
    pub fn after_dev_sam_app_exit(&mut self) -> Vec<Line> {
        self.set_line(String::new());
        vec![one(colored(
            encrypted_str!("dev-sam exited — type dev-sam to run it again, or help").decrypt(),
            theme::MUTED,
        ))]
    }

    /// Feeds one key. Returns what the front-end should paint.
    pub fn handle_key(&mut self, key: Key, mods: Mods, shell: &mut Shell) -> EditOutcome {
        if mods.ctrl {
            return self.handle_control_key(key);
        }
        match key {
            Key::Enter => self.on_command_submit(shell),
            Key::Tab => self.complete(shell),
            Key::Up => {
                self.step_history(shell, false);
                EditOutcome::Redraw
            }
            Key::Down => {
                self.step_history(shell, true);
                EditOutcome::Redraw
            }
            Key::Left => {
                if let Some((offset, _)) = self.line[..self.cursor].char_indices().next_back() {
                    self.cursor = offset;
                }
                EditOutcome::Redraw
            }
            Key::Right => {
                if let Some(character) = self.line[self.cursor..].chars().next() {
                    self.cursor += character.len_utf8();
                }
                EditOutcome::Redraw
            }
            Key::Home => {
                self.cursor = 0;
                EditOutcome::Redraw
            }
            Key::End => {
                self.cursor = self.line.len();
                EditOutcome::Redraw
            }
            Key::Backspace => {
                if let Some((offset, _)) = self.line[..self.cursor].char_indices().next_back() {
                    self.line.remove(offset);
                    self.cursor = offset;
                }
                EditOutcome::Redraw
            }
            Key::Delete => {
                if self.cursor < self.line.len() {
                    self.line.remove(self.cursor);
                }
                EditOutcome::Redraw
            }
            Key::Char(character) => {
                self.line.insert(self.cursor, character);
                self.cursor += character.len_utf8();
                EditOutcome::Redraw
            }
            _ => EditOutcome::None,
        }
    }

    fn handle_control_key(&mut self, key: Key) -> EditOutcome {
        match key {
            Key::Char('c') => {
                self.set_line(String::new());
                EditOutcome::Interrupt
            }
            // Wipe the screen but keep whatever was being typed, as a shell does.
            Key::Char('l') => EditOutcome::ClearScreen,
            Key::Char('a') => {
                self.cursor = 0;
                EditOutcome::Redraw
            }
            Key::Char('e') => {
                self.cursor = self.line.len();
                EditOutcome::Redraw
            }
            Key::Char('u') => {
                self.line.drain(..self.cursor);
                self.cursor = 0;
                EditOutcome::Redraw
            }
            _ => EditOutcome::None,
        }
    }

    fn on_command_submit(&mut self, shell: &mut Shell) -> EditOutcome {
        let line = std::mem::take(&mut self.line);
        self.cursor = 0;
        let outcome = shell.execute(&line);
        self.history_index = shell.history().len();
        match outcome {
            CommandRunOutcome::LaunchApp(launch) => EditOutcome::Launch(launch),
            CommandRunOutcome::Clear => EditOutcome::ClearScreen,
            CommandRunOutcome::RenderText(lines) => EditOutcome::Output(lines),
        }
    }

    fn complete(&mut self, shell: &Shell) -> EditOutcome {
        let candidates = shell.tab_complete(&self.line);
        match candidates.as_slice() {
            [] => EditOutcome::None,
            // Completion works on the last word of the whole line, so it also
            // lands the cursor at the end.
            [only] => {
                let start = self.line.rfind(' ').map_or(0, |position| position + 1);
                let suffix = if only.ends_with('/') { "" } else { " " };
                self.set_line(format!("{}{only}{suffix}", &self.line[..start]));
                EditOutcome::Redraw
            }
            many => EditOutcome::Completion(many.to_vec()),
        }
    }

    fn step_history(&mut self, shell: &Shell, newer: bool) {
        let history = shell.history();
        let next = if newer {
            (self.history_index + 1).min(history.len())
        } else {
            self.history_index.saturating_sub(1)
        };
        self.history_index = next;
        self.set_line(history.get(next).cloned().unwrap_or_default());
    }

    fn set_line(&mut self, line: String) {
        self.cursor = line.len();
        self.line = line;
    }

    /// The prompt row as it is rendered right now.
    pub fn prompt_row(&self) -> PromptRow {
        let before = self.line[..self.cursor].to_string();
        let mut chars = self.line[self.cursor..].chars();
        let at_cursor = chars.next();
        let after: String = chars.collect();
        PromptRow {
            before_cursor: vec![Span::new(before)],
            at_cursor,
            after_cursor: vec![Span::new(after)],
        }
    }
}

/// The prompt's own styled text: `sam@developersam:~$ `.
pub fn prompt_spans() -> Vec<Span> {
    vec![
        bold_colored(
            encrypted_str!("sam@developersam").decrypt(),
            theme::PROMPT_USER,
        ),
        colored(":", theme::MUTED),
        colored("~", theme::ACCENT_TEXT),
        colored("$ ", theme::MUTED),
    ]
}

// The virtual file system's names. Encrypted like the site's content in
// [`data`], so the listing is no more legible in the binary than what `cat`
// prints; each is written once so `ls`, `cat` and completion cannot drift.
const HOME_DIR: EncryptedString = encrypted_str!("/home/sam");
const PROJECTS_DIR: EncryptedString = encrypted_str!("projects");
const ABOUT_TXT: EncryptedString = encrypted_str!("about.txt");
const CONTACT_TXT: EncryptedString = encrypted_str!("contact.txt");
const README_MD: EncryptedString = encrypted_str!("readme.md");
const RESUME_PDF: EncryptedString = encrypted_str!("resume.pdf");
const TIMELINE_TXT: EncryptedString = encrypted_str!("timeline.txt");

/// The virtual file system rooted at `/home/sam`.
fn fs_entries(path: &[String]) -> Option<Vec<(String, bool)>> {
    if path.is_empty() {
        return Some(vec![
            (format!("{PROJECTS_DIR}/"), true),
            (ABOUT_TXT.decrypt(), false),
            (CONTACT_TXT.decrypt(), false),
            (README_MD.decrypt(), false),
            (RESUME_PDF.decrypt(), false),
            (TIMELINE_TXT.decrypt(), false),
        ]);
    }
    if path.len() == 1 && path[0] == PROJECTS_DIR.decrypt() {
        return Some(
            data::PROJECTS
                .iter()
                .map(|project| (project.id.decrypt(), false))
                .collect(),
        );
    }
    None
}

/// A line whose tail is the URL itself, clickable where it is printed.
fn link_line(mut spans: Line, url: &str) -> Line {
    spans.push(Span::new(url).linked(url));
    spans
}

/// The file at `path`, as `cat` prints it. The names are matched decrypted,
/// since a pattern cannot be an [`EncryptedString`].
fn read_file(path: &[String]) -> Option<Vec<Line>> {
    match path {
        [file] if *file == README_MD.decrypt() => Some(vec![
            one(colored(
                encrypted_str!("Served by Rust, compiled to WASM").decrypt(),
                theme::SUBTLE,
            )),
            Line::new(),
            one(colored(
                encrypted_str!("run dev-sam to launch the app, or explore:").decrypt(),
                theme::TEXT,
            )),
            one(colored(
                format!("  {CMD_CAT} {ABOUT_TXT}"),
                theme::ACCENT_TEXT,
            )),
            one(colored(
                format!("  {CMD_LS} {PROJECTS_DIR}"),
                theme::ACCENT_TEXT,
            )),
            one(colored(
                format!("  {CMD_CAT} {TIMELINE_TXT}"),
                theme::ACCENT_TEXT,
            )),
        ]),
        [file] if *file == ABOUT_TXT.decrypt() => {
            let mut out: Vec<Line> = highlight::doc_comment_lines();
            out.push(Line::new());
            out.extend(highlight::program_lines());
            out.push(Line::new());
            for entry in data::ABOUT_DOC_LINKS {
                let url = entry.url.decrypt();
                out.push(link_line(
                    vec![bold_colored(
                        format!("@{}:", entry.name),
                        theme::ACCENT_TEXT,
                    )],
                    &url,
                ));
            }
            Some(out)
        }
        [file] if *file == CONTACT_TXT.decrypt() => {
            let mut out = Vec::new();
            for entry in data::SOCIAL_LINKS {
                let url = entry.url.decrypt();
                out.push(link_line(
                    vec![bold_colored(
                        format!("{:<10}", entry.name),
                        theme::ACCENT_TEXT,
                    )],
                    &url,
                ));
            }
            Some(out)
        }
        [file] if *file == RESUME_PDF.decrypt() => Some(vec![link_line(
            vec![
                line_span(format!("{RESUME_PDF}: {}", encrypted_str!("PDF document"))),
                colored(encrypted_str!("open:").decrypt(), theme::MUTED),
            ],
            &encrypted_str!("https://developersam.com/resume.pdf").decrypt(),
        )]),
        [file] if *file == TIMELINE_TXT.decrypt() => {
            let mut out = Vec::new();
            for (index, event) in data::TIMELINE.iter().enumerate() {
                out.push(vec![
                    colored(
                        format!("{:>3}  {:<15}", index + 1, event.time),
                        theme::MUTED,
                    ),
                    colored(event.title.decrypt(), theme::TEXT),
                    Span::styled(
                        format!(" [{}]", event.category.label()),
                        TextStyle::new().color(event.category.color()),
                    ),
                ]);
            }
            Some(out)
        }
        [directory, name] if *directory == PROJECTS_DIR.decrypt() => {
            let project = data::PROJECTS
                .iter()
                .find(|project| project.id.decrypt() == *name)?;
            let mut out = vec![
                one(bold_colored(project.id.decrypt(), theme::ACCENT_TEXT)),
                one(colored(project.tagline.decrypt(), theme::SUBTLE)),
                Line::new(),
            ];
            for entry in project.links {
                let url = entry.url.decrypt();
                out.push(link_line(
                    vec![bold_colored(
                        format!("  {:<12}", entry.name),
                        theme::ACCENT_TEXT,
                    )],
                    &url,
                ));
            }
            Some(out)
        }
        _ => None,
    }
}

fn line_span(text: impl Into<String>) -> Span {
    Span::new(text)
}
