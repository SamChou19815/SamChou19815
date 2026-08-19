//! Full-screen TUI core for <https://developersam.com/terminal>.
//!
//! The crate is backend-agnostic: [`App::draw`] renders through any ratatui
//! `Frame`, so the same code drives the native `sam-tui-cli` binary (crossterm
//! backend) and the WebAssembly build (TestBackend serialized to ANSI for
//! xterm.js). Input arrives as the crate's own [`Input`] events, converted
//! from crossterm in the CLI and from browser events on the web.

pub mod ansi;
pub mod data;
#[cfg(target_arch = "wasm32")]
mod ffi;
mod highlight;
pub mod hit;
pub mod markdown;
pub mod shell;
pub mod theme;
mod view;

pub use hit::{HitAreas, HitRef, LinkRegion};

use ratatui_core::layout::Rect;

pub const TAB_NAMES: [&str; 6] = [
    "About",
    "Timeline",
    "Projects",
    "Work",
    "Education",
    "Contact",
];
pub const TAB_COUNT: usize = 6;
pub const ABOUT_TAB: usize = 0;
pub const TIMELINE_TAB: usize = 1;
pub const PROJECTS_TAB: usize = 2;
pub const WORK_TAB: usize = 3;
pub const EDUCATION_TAB: usize = 4;
pub const CONTACT_TAB: usize = 5;

/// Minimum terminal geometry; below this the app draws a "resize me" screen.
pub const MIN_COLS: u16 = 40;
pub const MIN_ROWS: u16 = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    PageUp,
    PageDown,
    Home,
    End,
    Delete,
    Char(char),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MouseKind {
    Press(MouseButton),
    Release(MouseButton),
    ScrollUp,
    ScrollDown,
}

#[derive(Clone, Copy)]
pub struct MouseEv {
    pub kind: MouseKind,
    pub col: u16,
    pub row: u16,
}

#[derive(Clone, Copy)]
pub enum Input {
    Key { key: Key, mods: Mods },
    Mouse(MouseEv),
}

/// A side effect requested by the app (opening a link).
#[derive(Clone, PartialEq, Eq)]
pub enum Action {
    OpenUrl(String),
}

#[derive(Clone, PartialEq, Eq)]
pub enum Modal {
    Timeline { event: usize, scroll: usize },
    Project { project: usize, scroll: usize },
    Help { scroll: usize },
}

pub struct App {
    pub cols: u16,
    pub rows: u16,
    pub tab: usize,
    visited: u32,
    /// Vertical scroll offset for the scrolling tab panes.
    scroll: [usize; TAB_COUNT],
    /// Selected row for the two list tabs.
    selected: [usize; TAB_COUNT],
    pub modal: Option<Modal>,
    pub quit: bool,
    pub(crate) hit: HitAreas,
    actions: Vec<Action>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        App {
            cols: 0,
            rows: 0,
            tab: ABOUT_TAB,
            visited: 1 << ABOUT_TAB,
            scroll: [0; TAB_COUNT],
            selected: [0; TAB_COUNT],
            modal: None,
            quit: false,
            hit: HitAreas::default(),
            actions: Vec::new(),
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
    }

    /// Renders the app through any ratatui backend and records mouse hit areas.
    pub fn draw(&mut self, frame: &mut ratatui_core::terminal::Frame) {
        view::draw(self, frame);
    }

    pub fn visited_count(&self) -> usize {
        self.visited.count_ones() as usize
    }

    pub fn take_actions(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.actions)
    }

    pub fn handle(&mut self, input: Input) {
        match input {
            Input::Key { key, mods } => self.on_key(key, mods),
            Input::Mouse(mouse) => self.on_mouse(mouse),
        }
    }

    fn on_key(&mut self, key: Key, mods: Mods) {
        if mods.ctrl && matches!(key, Key::Char('c') | Key::Char('d')) {
            self.quit = true;
            return;
        }
        if self.modal.is_some() {
            self.on_modal_key(key);
            return;
        }
        match key {
            Key::Left | Key::Char('h') => self.switch_tab(self.tab + TAB_COUNT - 1),
            Key::Right | Key::Char('l') | Key::Tab => self.switch_tab(self.tab + 1),
            Key::BackTab => self.switch_tab(self.tab + TAB_COUNT - 1),
            Key::Esc => {}
            Key::Char('?') => self.open_help(),
            Key::Char('q') => self.quit = true,
            Key::Char(c @ '1'..='6') => self.switch_tab(c as usize - '1' as usize),
            Key::Up => self.move_selection(-1, 1),
            Key::Char('k') if !mods.ctrl => self.move_selection(-1, 1),
            Key::Down => self.move_selection(1, 1),
            Key::Char('j') if !mods.ctrl => self.move_selection(1, 1),
            Key::PageUp => self.move_selection(-1, 10),
            Key::PageDown => self.move_selection(1, 10),
            Key::Home | Key::Char('g') => self.jump_to_edge(0),
            Key::End | Key::Char('G') => self.jump_to_edge(usize::MAX),
            Key::Enter => self.open_selected(),
            _ => {}
        }
    }

    fn on_modal_key(&mut self, key: Key) {
        // Digits activate the modal's buttons directly (1-9).
        if let Key::Char(digit @ '1'..='9') = key {
            let index = digit as usize - '1' as usize;
            let links: &[data::Link] = match &self.modal {
                Some(Modal::Timeline { event, .. }) => data::TIMELINE[*event].links,
                Some(Modal::Project { project, .. }) => data::PROJECTS[*project].links,
                _ => &[],
            };
            if let Some(link) = links.get(index) {
                self.actions.push(Action::OpenUrl(link.url.to_string()));
            }
            return;
        }
        match key {
            Key::Esc | Key::Backspace | Key::Char('q') | Key::Char(' ') => self.modal = None,
            Key::Up | Key::Char('k') => self.scroll_modal(-1),
            Key::Down | Key::Char('j') => self.scroll_modal(1),
            Key::PageUp => self.scroll_modal(-10),
            Key::PageDown => self.scroll_modal(10),
            Key::Home | Key::Char('g') => {
                if let Some(modal) = &mut self.modal {
                    *modal_scroll(modal) = 0;
                }
            }
            Key::End | Key::Char('G') => {
                if let Some(modal) = self.modal.clone() {
                    let total = view::modal_line_count(&modal);
                    if let Some(scroll) = self.modal.as_mut().map(modal_scroll) {
                        *scroll = total.saturating_sub(1);
                    }
                }
            }
            _ => {}
        }
    }

    fn switch_tab(&mut self, next: usize) {
        self.tab = next % TAB_COUNT;
        self.visited |= 1 << self.tab;
    }

    /// Moves list selection (Timeline/Projects) or scrolls text panes.
    fn move_selection(&mut self, direction: i32, amount: usize) {
        let list_len = match self.tab {
            TIMELINE_TAB => Some(data::TIMELINE.len()),
            PROJECTS_TAB => Some(data::PROJECTS.len()),
            _ => None,
        };
        if let Some(len) = list_len {
            let selected = &mut self.selected[self.tab];
            *selected = if direction < 0 {
                selected.saturating_sub(amount)
            } else {
                (*selected + amount).min(len.saturating_sub(1))
            };
        } else {
            self.scroll_text(direction, amount.max(3));
        }
    }

    fn jump_to_edge(&mut self, target: usize) {
        let list_len = match self.tab {
            TIMELINE_TAB => Some(data::TIMELINE.len()),
            PROJECTS_TAB => Some(data::PROJECTS.len()),
            _ => None,
        };
        if let Some(len) = list_len {
            self.selected[self.tab] = target.min(len.saturating_sub(1));
        } else {
            self.scroll[self.tab] = target;
        }
    }

    fn scroll_text(&mut self, direction: i32, amount: usize) {
        let len = view::tab_line_count(self.tab);
        let scroll = &mut self.scroll[self.tab];
        *scroll = if direction < 0 {
            scroll.saturating_sub(amount)
        } else {
            (*scroll + amount).min(len.saturating_sub(1))
        };
    }

    fn scroll_modal(&mut self, direction: i32) {
        let Some(modal) = self.modal.clone() else {
            return;
        };
        let total = view::modal_line_count(&modal);
        if let Some(scroll) = self.modal.as_mut().map(modal_scroll) {
            *scroll = if direction < 0 {
                scroll.saturating_sub(1)
            } else {
                (*scroll + 1).min(total.saturating_sub(1))
            };
        }
    }

    fn open_selected(&mut self) {
        match self.tab {
            TIMELINE_TAB => {
                let event = self.selected[TIMELINE_TAB].min(data::TIMELINE.len() - 1);
                self.modal = Some(Modal::Timeline { event, scroll: 0 });
            }
            PROJECTS_TAB => {
                let project = self.selected[PROJECTS_TAB].min(data::PROJECTS.len() - 1);
                self.modal = Some(Modal::Project { project, scroll: 0 });
            }
            _ => {}
        }
    }

    fn open_help(&mut self) {
        self.modal = Some(Modal::Help { scroll: 0 });
    }

    fn on_mouse(&mut self, mouse: MouseEv) {
        match mouse.kind {
            MouseKind::ScrollUp => self.on_scroll(mouse, -1),
            MouseKind::ScrollDown => self.on_scroll(mouse, 1),
            MouseKind::Release(_) => {}
            MouseKind::Press(_) => self.on_click(mouse.col, mouse.row),
        }
    }

    fn on_scroll(&mut self, _mouse: MouseEv, direction: i32) {
        if self.modal.is_some() {
            self.scroll_modal(direction);
        } else {
            self.move_selection(direction, 1);
        }
    }

    fn on_click(&mut self, col: u16, row: u16) {
        let click = Rect::new(col, row, 1, 1);
        let contains = |rect: Rect| rect.intersects(click);
        if let Some(modal_rect) = self.hit.modal {
            if contains(modal_rect) {
                for link in &self.hit.links {
                    if contains(link.rect) {
                        self.actions.push(Action::OpenUrl(link.url.clone()));
                        break;
                    }
                }
                return;
            }
            // Click on the backdrop closes the modal.
            self.modal = None;
            return;
        }
        for (index, tab_rect) in self.hit.tabs.iter().enumerate() {
            if contains(*tab_rect) {
                self.switch_tab(index);
                return;
            }
        }
        for link in &self.hit.links {
            if contains(link.rect) {
                self.actions.push(Action::OpenUrl(link.url.clone()));
                return;
            }
        }
        if matches!(self.tab, TIMELINE_TAB | PROJECTS_TAB) {
            for (row_rect, index) in &self.hit.rows {
                if contains(*row_rect) {
                    if self.selected[self.tab] == *index {
                        self.open_selected();
                    } else {
                        self.selected[self.tab] = *index;
                    }
                    return;
                }
            }
        }
    }
}

fn modal_scroll(modal: &mut Modal) -> &mut usize {
    match modal {
        Modal::Timeline { scroll, .. } | Modal::Project { scroll, .. } | Modal::Help { scroll } => {
            scroll
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.resize(100, 30);
        app
    }

    fn key(app: &mut App, key: Key) {
        app.handle(Input::Key {
            key,
            mods: Mods::default(),
        });
    }

    #[test]
    fn tab_navigation() {
        let mut app = app();
        assert_eq!(app.tab, ABOUT_TAB);
        key(&mut app, Key::Right);
        assert_eq!(app.tab, TIMELINE_TAB);
        key(&mut app, Key::Right);
        key(&mut app, Key::Right);
        key(&mut app, Key::Right);
        key(&mut app, Key::Right);
        assert_eq!(app.tab, CONTACT_TAB);
        key(&mut app, Key::Right);
        assert_eq!(app.tab, ABOUT_TAB);
        key(&mut app, Key::Left);
        assert_eq!(app.tab, CONTACT_TAB);
        key(&mut app, Key::Char('3'));
        assert_eq!(app.tab, PROJECTS_TAB);
        key(&mut app, Key::Char('h'));
        assert_eq!(app.tab, TIMELINE_TAB);
        key(&mut app, Key::Char('l'));
        assert_eq!(app.tab, PROJECTS_TAB);
    }

    #[test]
    fn visiting_tabs_is_tracked() {
        let mut app = app();
        assert_eq!(app.visited_count(), 1);
        for _ in 0..TAB_COUNT {
            key(&mut app, Key::Right);
        }
        assert_eq!(app.visited_count(), TAB_COUNT);
    }

    #[test]
    fn timeline_selection_and_modal() {
        let mut app = app();
        key(&mut app, Key::Char('2'));
        assert_eq!(app.selected[TIMELINE_TAB], 0);
        key(&mut app, Key::Down);
        key(&mut app, Key::Down);
        assert_eq!(app.selected[TIMELINE_TAB], 2);
        key(&mut app, Key::Enter);
        assert!(matches!(app.modal, Some(Modal::Timeline { event: 2, .. })));
        key(&mut app, Key::Down);
        key(&mut app, Key::Esc);
        assert!(app.modal.is_none());
    }

    #[test]
    fn quit_keys() {
        let mut app = app();
        key(&mut app, Key::Char('q'));
        assert!(app.quit);
        let mut app = App::new();
        app.resize(100, 30);
        key(&mut app, Key::Char('c'));
        assert!(!app.quit);
        app.handle(Input::Key {
            key: Key::Char('c'),
            mods: Mods {
                ctrl: true,
                ..Mods::default()
            },
        });
        assert!(app.quit);
    }

    #[test]
    fn selection_clamps_at_bounds() {
        let mut app = app();
        key(&mut app, Key::Char('2'));
        key(&mut app, Key::Up);
        key(&mut app, Key::Up);
        assert_eq!(app.selected[TIMELINE_TAB], 0);
        key(&mut app, Key::End);
        assert_eq!(app.selected[TIMELINE_TAB], data::TIMELINE.len() - 1);
        key(&mut app, Key::Down);
        assert_eq!(app.selected[TIMELINE_TAB], data::TIMELINE.len() - 1);
    }
}
