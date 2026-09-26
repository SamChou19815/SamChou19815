mod editor;
mod export;
mod fs;
mod maze;

pub(crate) use editor::{EditOutcome, LineEditor, PromptRow};

use crate::crypt::{encrypted_str, EncryptedString};
use crate::style::{bold_colored, colored, Line, Span};
use crate::theme;

use fs::{fs_entries, read_file, ABOUT_TXT, EVERYTHING_TXT, HOME_DIR};
use maze::ARCHIVE_DIR;

pub(in crate::shell) const CMD_CAT: EncryptedString = encrypted_str!("cat");
const CMD_CD: EncryptedString = encrypted_str!("cd");
const CMD_CLEAR: EncryptedString = encrypted_str!("clear");
const CMD_DEV_SAM: EncryptedString = encrypted_str!("dev-sam");
const CMD_ECHO: EncryptedString = encrypted_str!("echo");
const CMD_HELP: EncryptedString = encrypted_str!("help");
const CMD_HISTORY: EncryptedString = encrypted_str!("history");
pub(in crate::shell) const CMD_LS: EncryptedString = encrypted_str!("ls");
const CMD_PWD: EncryptedString = encrypted_str!("pwd");
const CMD_WHOAMI: EncryptedString = encrypted_str!("whoami");

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

#[derive(Clone)]
pub(crate) struct Shell {
    /// `[]` = `/home/sam`
    cwd_segments: Vec<String>,
    history: Vec<String>,
    /// Set once `everything.txt` is read. From then on nothing leaves `~/archive`.
    trap: Option<Trap>,
}

/// How far into `everything.txt` the reader is. See [`export`].
#[derive(Clone)]
struct Trap {
    /// The part to read next.
    part: usize,
}

/// Where each part of `everything.txt` lives, index 0 being part 1. See [`maze::part_paths`].
fn part_paths() -> &'static [String] {
    static PATHS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    PATHS.get_or_init(|| maze::part_paths(export::PARTS))
}

/// Where part `part` (1-based) lives, relative to the archive root.
fn part_segments(part: usize) -> Vec<String> {
    part_paths()
        .get(part.saturating_sub(1))
        .map(|path| maze::segments_of(path))
        .unwrap_or_default()
}

pub(in crate::shell) enum CommandRunOutcome {
    RenderText(Vec<Line>),
    Clear,
    LaunchApp,
}

impl Shell {
    pub(crate) fn new() -> Self {
        Shell {
            cwd_segments: Vec::new(),
            history: Vec::new(),
            trap: None,
        }
    }

    pub(in crate::shell) fn history(&self) -> &[String] {
        &self.history
    }

    /// `sam@developersam:~/projects$ `
    pub(in crate::shell) fn prompt_spans(&self) -> Vec<Span> {
        let mut cwd = String::from("~");
        for segment in &self.cwd_segments {
            cwd.push('/');
            cwd.push_str(segment);
        }
        vec![
            bold_colored(
                encrypted_str!("sam@developersam").decrypt(),
                theme::PROMPT_USER,
            ),
            colored(":", theme::MUTED),
            colored(cwd, theme::ACCENT_TEXT),
            colored("$ ", theme::MUTED),
        ]
    }

    pub(in crate::shell) fn execute(&mut self, line: &str) -> CommandRunOutcome {
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
            if self.trap.is_some() {
                return CommandRunOutcome::RenderText(self.refuse_exit(command));
            }
            CommandRunOutcome::LaunchApp
        } else if command == CMD_HELP.decrypt() {
            CommandRunOutcome::RenderText(self.help())
        } else if command == CMD_LS.decrypt() {
            CommandRunOutcome::RenderText(self.ls(command, &args))
        } else if command == CMD_CAT.decrypt() {
            CommandRunOutcome::RenderText(self.cat(command, &args))
        } else if command == CMD_CD.decrypt() {
            CommandRunOutcome::RenderText(self.cd(command, &args))
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

    fn tab_complete(&self, line: &str) -> Vec<String> {
        let trimmed = line.trim_start();
        let (before, word) = trimmed
            .rsplit_once(' ')
            .map_or(("", trimmed), |(before, word)| (before, word.trim_start()));
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

    fn ls(&self, command: &str, args: &[&str]) -> Vec<Line> {
        let target = match args.first() {
            None => self.cwd_segments.clone(),
            Some(&arg) => match self.resolve_path(arg) {
                Ok(path) => path,
                Err(error) => return vec![error],
            },
        };
        if self.escapes(&target) {
            return self.refuse_exit(command);
        }
        match fs_entries(&target) {
            Some(items) => {
                let width = items
                    .iter()
                    .map(|(name, _)| name.chars().count())
                    .max()
                    .unwrap_or(0)
                    + 2;
                let mut contents = Vec::new();
                for (name, directory) in items {
                    // Not `{name:<width$}`: a runtime width can panic.
                    let padding = " ".repeat(width.saturating_sub(name.chars().count()));
                    let padded = format!("{name}{padding}");
                    if directory {
                        contents.push(bold_colored(padded, theme::ACCENT_TEXT));
                    } else {
                        contents.push(colored(padded, theme::TEXT));
                    }
                }
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
                    args.first().copied().unwrap_or_default()
                ),
                theme::FUNCTION,
            ))],
        }
    }

    fn cat(&mut self, command: &str, args: &[&str]) -> Vec<Line> {
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
        if path == [EVERYTHING_TXT.decrypt()] {
            return self.open_export();
        }
        if self.escapes(&path) {
            return self.refuse_exit(command);
        }
        if let [directory, rest @ ..] = path.as_slice() {
            if *directory == ARCHIVE_DIR.decrypt() {
                if let Some(index) = part_paths()
                    .iter()
                    .position(|part| maze::segments_of(part) == rest)
                {
                    return self.read_part(index + 1);
                }
            }
        }
        match read_file(&path) {
            Some(content) => content,
            None => vec![one(colored(
                format!("{}: {arg}", encrypted_str!("cat: no such file").decrypt()),
                theme::FUNCTION,
            ))],
        }
    }

    fn cd(&mut self, command: &str, args: &[&str]) -> Vec<Line> {
        let path = match args.first() {
            None => Vec::new(),
            Some(&arg) => match self.resolve_path(arg) {
                Ok(path) => path,
                Err(error) => return vec![error],
            },
        };
        if self.escapes(&path) {
            return self.refuse_exit(command);
        }
        if fs_entries(&path).is_none() {
            return vec![one(colored(
                format!(
                    "{}: {}",
                    encrypted_str!("cd: not a directory").decrypt(),
                    args.first().copied().unwrap_or_default()
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
                    let diagonals = previous.iter().zip(previous.iter().skip(1));
                    for (cb, (diagonal, above)) in b.iter().zip(diagonals) {
                        let left = current.last().copied().unwrap_or_default();
                        let cost = usize::from(ca != *cb);
                        current.push((diagonal + cost).min(left + 1).min(above + 1));
                    }
                    previous = current;
                }
                previous.last().copied().unwrap_or_default()
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
        let (base, segment) = word
            .rfind('/')
            .and_then(|position| word.split_at_checked(position + 1))
            .unwrap_or(("", word));
        self.resolve_segments(base)
            .and_then(|path| fs_entries(&path))
            .into_iter()
            .flatten()
            .filter(|(name, _)| name.starts_with(segment))
            .map(|(name, _)| format!("{base}{name}"))
            .collect()
    }

    fn resolve_path(&self, arg: &str) -> Result<Vec<String>, Line> {
        self.resolve_segments(arg).ok_or_else(|| {
            one(colored(
                format!("{}: {arg}", encrypted_str!("no such file or directory")),
                theme::FUNCTION,
            ))
        })
    }

    /// Outside `~/archive`, which a trapped shell never lets a command reach.
    fn escapes(&self, path: &[String]) -> bool {
        self.trap.is_some()
            && path
                .first()
                .is_none_or(|first| *first != ARCHIVE_DIR.decrypt())
    }

    /// The way forward is always the next part.
    fn refuse_exit(&self, command: &str) -> Vec<Line> {
        let mut out = vec![one(colored(
            format!(
                "{command}: {}",
                encrypted_str!(
                    "the terminal is in reader mode while everything.txt is open; only the \
                     archive it is stored in is available until the last part."
                )
            ),
            theme::FUNCTION,
        ))];
        out.extend(self.export_reminder());
        out
    }

    /// Where the reader is in `everything.txt` and where to go next, while trapped.
    pub(in crate::shell) fn export_reminder(&self) -> Vec<Line> {
        let Some(trap) = &self.trap else {
            return Vec::new();
        };
        vec![
            line_of(format!(
                "{} {} {} {}.",
                encrypted_str!("you are up to part"),
                trap.part,
                encrypted_str!("of"),
                part_paths().len()
            )),
            line_of(maze::directions(trap.part, &part_segments(trap.part), None)),
        ]
    }

    /// The bait. Reading it springs the trap, see [`Shell::escapes`].
    fn open_export(&mut self) -> Vec<Line> {
        self.trap.get_or_insert(Trap { part: 1 });
        export::contents(maze::directions(1, &part_segments(1), None))
    }

    /// Part `part`, wherever the reader is and whatever they read before: a part is always the
    /// same file, so nothing a reader does turns up a contradiction. Reading one springs the trap
    /// too, and after the last part it lets go: there really is an end.
    fn read_part(&mut self, part: usize) -> Vec<Line> {
        let total = part_paths().len();
        if part >= total {
            self.trap = None;
            return export::part(part, None);
        }
        self.trap = Some(Trap { part: part + 1 });
        let here = part_segments(part);
        let from = here.split_last().map(|(_, dir)| dir);
        export::part(
            part,
            Some(maze::directions(part + 1, &part_segments(part + 1), from)),
        )
    }

    pub(crate) fn is_trapped(&self) -> bool {
        self.trap.is_some()
    }

    /// Relative to the cwd, or absolute: `~/...` or `/home/sam/...`. None outside the home.
    fn resolve_segments(&self, arg: &str) -> Option<Vec<String>> {
        let home = HOME_DIR.decrypt();
        let (mut path, arg) = if arg == "~" || arg.starts_with("~/") {
            (Vec::new(), arg.trim_start_matches('~'))
        } else if let Some(rest) = arg.strip_prefix(&home) {
            if !rest.is_empty() && !rest.starts_with('/') {
                return None;
            }
            (Vec::new(), rest)
        } else if arg.starts_with('/') {
            return None;
        } else {
            (self.cwd_segments.clone(), arg)
        };
        for part in arg.split('/').filter(|part| !part.is_empty()) {
            match part {
                "." => {}
                ".." => {
                    path.pop();
                }
                _ => path.push(part.to_string()),
            }
        }
        Some(path)
    }
}

pub(in crate::shell) fn line_of(text: impl Into<String>) -> Line {
    vec![Span::new(text)]
}

pub(in crate::shell) fn one(span: Span) -> Line {
    vec![span]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(lines: &[Line]) -> String {
        lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn run(shell: &mut Shell, line: &str) -> String {
        match shell.execute(line) {
            CommandRunOutcome::RenderText(lines) => text(&lines),
            _ => String::new(),
        }
    }

    #[test]
    fn export_runs_to_a_real_end_and_every_part_is_a_fixed_file() {
        let mut shell = Shell::new();
        let contents = run(&mut shell, "cat everything.txt");
        let paths = part_paths();
        let total = paths.len();
        assert_eq!(total, export::PARTS);
        assert!(contents.contains(&format!("({total} parts)")));
        for (index, path) in paths.iter().enumerate() {
            assert!(!paths[..index].contains(path), "{path} repeats");
            assert!(!path.ends_with("cake.txt"), "{path}");
            assert!(
                read_file(&[vec![ARCHIVE_DIR.decrypt()], maze::segments_of(path)].concat())
                    .is_some()
            );
        }
        // A part is the same file for anyone: a fresh shell reading part 5 gets part 5.
        let fifth = run(&mut Shell::new(), &format!("cat {}", paths[4]));
        assert!(fifth.contains(&format!("part 5 of {total}")), "{fifth}");
        let mut last = String::new();
        for (index, path) in paths.iter().enumerate() {
            last = run(&mut shell, &format!("cat {path}"));
            assert!(
                last.contains(&format!("part {} of {total}", index + 1)),
                "{last}"
            );
        }
        assert_eq!(total, 20);
        assert!(last.contains("this is a maze."), "{last}");
        assert!(shell.trap.is_none(), "the last part lets go");
        assert!(matches!(
            shell.execute("dev-sam"),
            CommandRunOutcome::LaunchApp
        ));
    }
}
