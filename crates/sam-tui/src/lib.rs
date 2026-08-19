//! Full-screen TUI core for <https://developersam.com/terminal>, built on
//! iocraft: components are plain functions, state lives in hooks, and the
//! same tree runs natively (crossterm over stdio) and in the browser
//! (a wasm engine pumped by the web terminal through a plain C ABI).
//!
//! Mouse handling: components register clickable regions ([`hit`]) from
//! their layout rects each frame; the app dispatches
//! [`crossterm`] mouse events against that registry.

pub mod data;
#[cfg(target_arch = "wasm32")]
pub mod ffi;
mod highlight;
pub mod hit;
pub mod markdown;
pub mod shell;
pub mod theme;
pub mod view;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

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

/// Rows of a timeline card: title, optional detail, optional buttons, gap.
pub fn card_height(event: &data::TimelineEvent) -> usize {
    1 + usize::from(event.detail.is_some()) + usize::from(!event.links.is_empty()) + 1
}

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
pub enum MouseEv {
    Click { col: u16, row: u16 },
    ScrollUp,
    ScrollDown,
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

thread_local! {
    /// Actions produced by the latest input, drained by the wasm host.
    pub(crate) static PENDING_ACTIONS: std::cell::RefCell<Vec<Action>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Clone is used to snapshot state for pure rendering.
#[derive(Clone)]
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
            actions: Vec::new(),
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols.max(1);
        self.rows = rows.max(1);
    }

    /// Rows visible inside the content pane (border box minus chrome).
    pub fn viewport(&self) -> usize {
        self.rows.saturating_sub(6) as usize
    }

    pub fn scroll(&self, tab: usize) -> usize {
        self.scroll[tab]
    }

    pub fn selected(&self, tab: usize) -> usize {
        self.selected[tab]
    }

    pub fn visited_count(&self) -> usize {
        self.visited.count_ones() as usize
    }

    pub fn take_actions(&mut self) -> Vec<Action> {
        let actions = std::mem::take(&mut self.actions);
        if !actions.is_empty() {
            PENDING_ACTIONS.with(|pending| {
                pending.borrow_mut().extend(actions.iter().cloned());
            });
        }
        actions
    }

    /// Feeds one crossterm event into the state machine.
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Key(key) => self.handle_key_event(key),
            Event::Mouse(mouse) => self.handle_mouse_event(mouse),
            _ => {}
        }
    }

    fn handle_key_event(&mut self, key: &KeyEvent) {
        let mods = Mods {
            ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
            shift: key.modifiers.contains(KeyModifiers::SHIFT),
            alt: key.modifiers.contains(KeyModifiers::ALT),
        };
        let mapped = match key.code {
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Left => Key::Left,
            KeyCode::Right => Key::Right,
            KeyCode::Enter => Key::Enter,
            KeyCode::Esc => Key::Esc,
            KeyCode::Tab => {
                if mods.shift {
                    Key::BackTab
                } else {
                    Key::Tab
                }
            }
            KeyCode::Backspace => Key::Backspace,
            KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::Delete => Key::Delete,
            KeyCode::Char(character) => Key::Char(character),
            _ => return,
        };
        self.handle(Input::Key { key: mapped, mods });
    }

    fn handle_mouse_event(&mut self, mouse: &MouseEvent) {
        let input = match mouse.kind {
            MouseEventKind::Down(_) => MouseEv::Click {
                col: mouse.column,
                row: mouse.row,
            },
            MouseEventKind::ScrollUp => MouseEv::ScrollUp,
            MouseEventKind::ScrollDown => MouseEv::ScrollDown,
            _ => return,
        };
        self.handle(Input::Mouse(input));
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
            Key::Char('?') => self.modal = Some(Modal::Help { scroll: 0 }),
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

    fn on_mouse(&mut self, mouse: MouseEv) {
        match mouse {
            MouseEv::ScrollUp | MouseEv::ScrollDown => {
                let direction = match mouse {
                    MouseEv::ScrollUp => -1,
                    _ => 1,
                };
                if self.modal.is_some() {
                    self.scroll_modal(direction);
                } else {
                    self.move_selection(direction, 1);
                }
            }
            MouseEv::Click { col, row } => self.on_click(col, row),
        }
    }

    fn on_click(&mut self, col: u16, row: u16) {
        match hit::hit_test(col, row) {
            Some(hit::HitTarget::Link(url)) => {
                self.actions.push(Action::OpenUrl(url));
            }
            Some(hit::HitTarget::Tab(index)) => self.switch_tab(index),
            Some(hit::HitTarget::Item(index)) => {
                if self.tab == TIMELINE_TAB || self.tab == PROJECTS_TAB {
                    if self.selected[self.tab] == index {
                        self.open_selected();
                    } else {
                        self.selected[self.tab] = index;
                    }
                }
            }
            // Clicks outside any region close an open dialog.
            None => {
                if self.modal.is_some() {
                    self.modal = None;
                }
            }
        }
    }

    /// Clamps list scroll so the selection stays inside the viewport.
    pub fn clamp_list_scroll(&mut self) {
        if !matches!(self.tab, TIMELINE_TAB | PROJECTS_TAB) {
            return;
        }
        let (len, heights): (usize, Vec<usize>) = if self.tab == TIMELINE_TAB {
            (
                data::TIMELINE.len(),
                data::TIMELINE.iter().map(card_height).collect(),
            )
        } else {
            (data::PROJECTS.len(), vec![1; data::PROJECTS.len()])
        };
        let selected = self.selected[self.tab].min(len.saturating_sub(1));
        self.selected[self.tab] = selected;
        let viewport = self.viewport().max(1);
        let mut scroll = self.scroll[self.tab];
        let top = offset_for(&heights, selected);
        if top < scroll {
            scroll = top;
        }
        // Ensure the selected item's bottom fits.
        let selected_bottom = top + heights[selected];
        if scroll + viewport < selected_bottom {
            scroll = selected_bottom.saturating_sub(viewport);
        }
        self.scroll[self.tab] = scroll;
    }
}

fn offset_for(heights: &[usize], end: usize) -> usize {
    heights.iter().take(end).sum()
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
        assert_eq!(app.selected(TIMELINE_TAB), 0);
        key(&mut app, Key::Down);
        key(&mut app, Key::Down);
        key(&mut app, Key::Char('j'));
        assert_eq!(app.selected(TIMELINE_TAB), 3);
        key(&mut app, Key::Enter);
        assert!(matches!(app.modal, Some(Modal::Timeline { event: 3, .. })));
        key(&mut app, Key::Down);
        key(&mut app, Key::Esc);
        assert!(app.modal.is_none());
    }

    #[test]
    fn modal_digits_open_buttons() {
        let mut app = app();
        key(&mut app, Key::Char('2'));
        key(&mut app, Key::Enter);
        key(&mut app, Key::Char('1'));
        let actions = app.take_actions();
        assert!(matches!(
            actions.as_slice(),
            [Action::OpenUrl(url)] if url.contains("flow.org")
        ));
    }

    #[test]
    fn quit_keys() {
        let mut app = app();
        key(&mut app, Key::Char('q'));
        assert!(app.quit);
        let mut fresh = App::new();
        fresh.resize(100, 30);
        key(&mut fresh, Key::Char('c'));
        assert!(!fresh.quit);
        fresh.handle(Input::Key {
            key: Key::Char('c'),
            mods: Mods {
                ctrl: true,
                ..Mods::default()
            },
        });
        assert!(fresh.quit);
    }

    #[test]
    fn selection_clamps_at_bounds() {
        let mut app = app();
        key(&mut app, Key::Char('2'));
        key(&mut app, Key::Up);
        key(&mut app, Key::Up);
        assert_eq!(app.selected(TIMELINE_TAB), 0);
        key(&mut app, Key::End);
        assert_eq!(app.selected(TIMELINE_TAB), data::TIMELINE.len() - 1);
        key(&mut app, Key::Down);
        assert_eq!(app.selected(TIMELINE_TAB), data::TIMELINE.len() - 1);
    }

    #[test]
    fn crossterm_events_feed_the_machine() {
        let mut app = app();
        app.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('2'),
            KeyModifiers::NONE,
        )));
        assert_eq!(app.tab, TIMELINE_TAB);
        app.handle_event(&Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }));
        // Column 0/row 0 is the header title, not a tab; nothing changes.
        assert_eq!(app.tab, TIMELINE_TAB);
    }
}
