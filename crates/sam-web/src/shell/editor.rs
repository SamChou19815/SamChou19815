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
    line: String,
    /// Byte offset.
    cursor: usize,
    /// `== history.len()` when typing a fresh line.
    history_index: usize,
}

impl LineEditor {
    pub(crate) fn new() -> Self {
        LineEditor {
            line: String::new(),
            cursor: 0,
            history_index: 0,
        }
    }

    /// Pre-types `dev-sam` so Enter launches the app.
    pub(crate) fn opening_screen(&mut self) -> Vec<Line> {
        let out = vec![
            one(colored(
                encrypted_str!("sam-sh 1.0 — developer sam's terminal").decrypt(),
                theme::MUTED,
            )),
            one(colored(
                encrypted_str!("type help for commands, or run dev-sam").decrypt(),
                theme::MUTED,
            )),
            Line::new(),
        ];
        self.set_line(CMD_DEV_SAM.decrypt());
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
                if let Some((offset, _)) = self.line[..self.cursor].char_indices().next_back() {
                    self.cursor = offset;
                }
                EditOutcome::None
            }
            Key::Right => {
                if let Some(character) = self.line[self.cursor..].chars().next() {
                    self.cursor += character.len_utf8();
                }
                EditOutcome::None
            }
            Key::Home => {
                self.cursor = 0;
                EditOutcome::None
            }
            Key::End => {
                self.cursor = self.line.len();
                EditOutcome::None
            }
            Key::Backspace => {
                if let Some((offset, _)) = self.line[..self.cursor].char_indices().next_back() {
                    self.line.remove(offset);
                    self.cursor = offset;
                }
                EditOutcome::None
            }
            Key::Delete => {
                if self.cursor < self.line.len() {
                    self.line.remove(self.cursor);
                }
                EditOutcome::None
            }
            Key::Char(character) => {
                self.line.insert(self.cursor, character);
                self.cursor += character.len_utf8();
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
                self.cursor = 0;
                EditOutcome::None
            }
            Key::Char('e') => {
                self.cursor = self.line.len();
                EditOutcome::None
            }
            Key::Char('u') => {
                self.line.drain(..self.cursor);
                self.cursor = 0;
                EditOutcome::None
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
            CommandRunOutcome::LaunchApp => EditOutcome::Launch,
            CommandRunOutcome::Clear => EditOutcome::ClearScreen,
            CommandRunOutcome::RenderText(lines) => EditOutcome::Output(lines),
        }
    }

    fn complete(&mut self, shell: &Shell) -> EditOutcome {
        let candidates = shell.tab_complete(&self.line);
        match candidates.as_slice() {
            [] => EditOutcome::None,
            [only] => {
                let start = self.line.rfind(' ').map_or(0, |position| position + 1);
                let suffix = if only.ends_with('/') { "" } else { " " };
                self.set_line(format!("{}{only}{suffix}", &self.line[..start]));
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

    fn set_line(&mut self, line: String) {
        self.cursor = line.len();
        self.line = line;
    }

    pub(crate) fn prompt_row(&self, shell: &Shell) -> PromptRow {
        let before = self.line[..self.cursor].to_string();
        let mut chars = self.line[self.cursor..].chars();
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
