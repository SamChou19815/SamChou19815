use crate::crypt::encrypted_str;
use crate::keys::{Key, Mods};
use crate::style::{colored, Line, Span};
use crate::theme;

use super::{one, CommandRunOutcome, Shell, CMD_DEV_SAM};

/// The line split around the cursor so the UI can draw a block cursor.
pub(crate) struct PromptRow {
    /// Snapshotted: `cd` changes the prompt before the row is frozen into scrollback.
    pub(crate) prompt: Vec<Span>,
    pub(crate) before_cursor: Vec<Span>,
    pub(crate) at_cursor: Option<char>,
    pub(crate) after_cursor: Vec<Span>,
}

pub(crate) enum EditOutcome {
    /// Nothing to append to scrollback (the prompt row itself is always re-rendered).
    None,
    Output(Vec<Line>),
    ClearScreen,
    /// `dev-sam` was run.
    Launch,
    Completion(Vec<String>),
    /// Ctrl+C. Freeze the row with `^C` echoed at the cursor.
    Interrupt,
}

/// History lives on [`Shell`] so `history` and arrow keys share it.
#[derive(Clone)]
pub(crate) struct LineEditor {
    /// The line is `before + after`, split at the cursor, so editing never indexes into it.
    before: String,
    after: String,
    /// `== history.len()` when typing a fresh line.
    history_index: usize,
}

impl LineEditor {
    pub(crate) fn new() -> Self {
        LineEditor {
            before: String::new(),
            after: String::new(),
            history_index: 0,
        }
    }

    /// Pre-types `dev-sam` so Enter launches the app. While trapped in the export, says where
    /// the reader is up to instead, and pre-types nothing: Enter alone is no way forward.
    pub(crate) fn opening_screen(&mut self, shell: &Shell) -> Vec<Line> {
        let mut out = vec![
            one(colored(
                encrypted_str!("sam-sh 1.0 — developer sam's terminal").decrypt(),
                theme::MUTED,
            )),
            one(colored(
                encrypted_str!("type help for commands, or run dev-sam").decrypt(),
                theme::MUTED,
            )),
            // The bait, see `Shell::open_export`.
            one(colored(
                encrypted_str!(
                    "the whole site, every blog post included, is also in plain text: \
                     cat everything.txt"
                )
                .decrypt(),
                theme::MUTED,
            )),
            Line::new(),
        ];
        if shell.is_trapped() {
            out.extend(shell.export_reminder());
            out.push(Line::new());
        } else {
            self.set_line(CMD_DEV_SAM.decrypt());
        }
        out
    }

    pub(crate) fn after_dev_sam_app_exit() -> Vec<Line> {
        vec![one(colored(
            encrypted_str!("dev-sam exited — type dev-sam to run it again, or help").decrypt(),
            theme::MUTED,
        ))]
    }

    pub(crate) fn handle_key(&mut self, key: Key, mods: Mods, shell: &mut Shell) -> EditOutcome {
        if mods.ctrl {
            return self.handle_control_key(key);
        }
        match key {
            Key::Enter => self.on_command_submit(shell),
            Key::Tab => self.complete(shell),
            Key::Up => {
                self.step_history(shell, false);
                EditOutcome::None
            }
            Key::Down => {
                self.step_history(shell, true);
                EditOutcome::None
            }
            Key::Left => {
                if let Some(character) = self.before.pop() {
                    self.after = format!("{character}{}", self.after);
                }
                EditOutcome::None
            }
            Key::Right => {
                if let Some(character) = self.pop_after() {
                    self.before.push(character);
                }
                EditOutcome::None
            }
            Key::Home => {
                self.cursor_to_start();
                EditOutcome::None
            }
            Key::End => {
                self.cursor_to_end();
                EditOutcome::None
            }
            Key::Backspace => {
                self.before.pop();
                EditOutcome::None
            }
            Key::Delete => {
                self.pop_after();
                EditOutcome::None
            }
            Key::Char(character) => {
                self.before.push(character);
                EditOutcome::None
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
            Key::Char('l') => EditOutcome::ClearScreen,
            Key::Char('a') => {
                self.cursor_to_start();
                EditOutcome::None
            }
            Key::Char('e') => {
                self.cursor_to_end();
                EditOutcome::None
            }
            Key::Char('u') => {
                self.before.clear();
                EditOutcome::None
            }
            _ => EditOutcome::None,
        }
    }

    fn on_command_submit(&mut self, shell: &mut Shell) -> EditOutcome {
        let line = self.line();
        self.set_line(String::new());
        let outcome = shell.execute(&line);
        self.history_index = shell.history().len();
        match outcome {
            CommandRunOutcome::LaunchApp => EditOutcome::Launch,
            CommandRunOutcome::Clear => EditOutcome::ClearScreen,
            CommandRunOutcome::RenderText(lines) => EditOutcome::Output(lines),
        }
    }

    fn complete(&mut self, shell: &Shell) -> EditOutcome {
        let line = self.line();
        let candidates = shell.tab_complete(&line);
        match candidates.as_slice() {
            [] => EditOutcome::None,
            [only] => {
                let head = line
                    .rsplit_once(' ')
                    .map_or(String::new(), |(head, _)| format!("{head} "));
                let suffix = if only.ends_with('/') { "" } else { " " };
                self.set_line(format!("{head}{only}{suffix}"));
                EditOutcome::None
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

    fn line(&self) -> String {
        format!("{}{}", self.before, self.after)
    }

    fn set_line(&mut self, line: String) {
        self.before = line;
        self.after.clear();
    }

    fn pop_after(&mut self) -> Option<char> {
        let mut chars = self.after.chars();
        let first = chars.next();
        self.after = chars.as_str().to_string();
        first
    }

    fn cursor_to_start(&mut self) {
        self.after = std::mem::take(&mut self.before) + &self.after;
    }

    fn cursor_to_end(&mut self) {
        self.before.push_str(&std::mem::take(&mut self.after));
    }

    pub(crate) fn prompt_row(&self, shell: &Shell) -> PromptRow {
        let before = self.before.clone();
        let mut chars = self.after.chars();
        let at_cursor = chars.next();
        let after: String = chars.collect();
        PromptRow {
            prompt: shell.prompt_spans(),
            before_cursor: vec![Span::new(before)],
            at_cursor,
            after_cursor: vec![Span::new(after)],
        }
    }
}
