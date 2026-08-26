//! Full-screen TUI core for <https://developersam.com>, built on
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
pub mod image;
pub mod markdown;
pub mod shell;
pub mod theme;
pub mod view;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};

pub const TAB_NAMES: [&str; 2] = ["About", "Timeline"];
pub const TAB_COUNT: usize = 2;
pub const ABOUT_TAB: usize = 0;
pub const TIMELINE_TAB: usize = 1;

/// Width assumed before the first resize event tells us the real one.
const ASSUMED_COLS: u16 = 80;

/// Usable width inside a card's text column: the pane's border and padding
/// plus the card's gutter are spoken for before any text is drawn.
pub fn content_width(cols: u16) -> usize {
    let cols = if cols == 0 { ASSUMED_COLS } else { cols };
    (cols as usize).saturating_sub(7).max(12)
}

/// Rows a string occupies once wrapped into `width` columns.
fn wrapped_rows(text: &str, width: usize) -> usize {
    text.chars().count().div_ceil(width.max(1)).max(1)
}

/// Rows of a timeline card, mirroring the homepage card: title, the time as a
/// subheader, the artwork, the wrapped detail, a button row, then a blank
/// separator.
pub fn card_height(event: &data::TimelineEvent, cols: u16) -> usize {
    let inner = content_width(cols);
    let detail = event.detail.map_or(0, |detail| wrapped_rows(detail, inner));
    let links = if event.links.is_empty() {
        0
    } else {
        wrapped_rows(&link_row_label(event.links), inner)
    };
    let image = image::rows(event.image, cols, image::THUMBNAIL);
    1 + 1 + image + detail + links + 1
}

/// The button row as it is rendered, for width math.
fn link_row_label(links: &[data::Link]) -> String {
    links
        .iter()
        .map(|link| format!(" {} ", link.name.to_uppercase()))
        .collect()
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

    /// Rows visible inside the content pane: the screen minus the header,
    /// the pane's border and title row, and the status bar.
    pub fn viewport(&self) -> usize {
        let rows = if self.rows == 0 { 24 } else { self.rows };
        rows.saturating_sub(5) as usize
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
            // A key reports twice where the keyboard enhancement flags are
            // supported — once pressed, once released. Act on the press only,
            // and let a held key's repeats through.
            Event::Key(key) if key.kind != KeyEventKind::Release => self.handle_key_event(key),
            Event::Mouse(mouse) => self.handle_mouse_event(mouse),
            Event::Resize(cols, rows) => self.resize(*cols, *rows),
            _ => {}
        }
        // Selection and viewport both move under these events; re-anchor the
        // scroll so the selected row stays on screen and no pane is scrolled
        // past its last line.
        self.clamp_scroll();
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
            Key::Char(c @ '1'..='2') => self.switch_tab(c as usize - '1' as usize),
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
                    let limit = self.max_modal_scroll(&modal);
                    if let Some(scroll) = self.modal.as_mut().map(modal_scroll) {
                        *scroll = limit;
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

    /// Moves list selection (Timeline) or scrolls text panes.
    fn move_selection(&mut self, direction: i32, amount: usize) {
        if let Some(len) = self.list_len() {
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

    /// Item count of the current tab, for the one tab that is a list.
    fn list_len(&self) -> Option<usize> {
        (self.tab == TIMELINE_TAB).then_some(data::TIMELINE.len())
    }

    fn jump_to_edge(&mut self, target: usize) {
        if let Some(len) = self.list_len() {
            self.selected[self.tab] = target.min(len.saturating_sub(1));
        } else {
            self.scroll[self.tab] = target.min(self.max_scroll());
        }
    }

    /// The furthest a text pane scrolls: far enough to bring its last line to
    /// the bottom of the viewport, and no further. Past that there is nothing
    /// left to read, and scrolling blank space up the screen is not scrolling.
    fn max_scroll(&self) -> usize {
        let visible = view::tab_viewport(self.tab, self.viewport());
        view::tab_line_count(self.tab, self.cols).saturating_sub(visible)
    }

    /// The same for the open dialog's body.
    fn max_modal_scroll(&self, modal: &Modal) -> usize {
        view::modal_line_count(modal, self.cols)
            .saturating_sub(view::modal_viewport(modal, self.cols, self.rows))
    }

    fn scroll_text(&mut self, direction: i32, amount: usize) {
        let limit = self.max_scroll();
        let scroll = &mut self.scroll[self.tab];
        *scroll = if direction < 0 {
            scroll.saturating_sub(amount)
        } else {
            (*scroll + amount).min(limit)
        };
    }

    fn scroll_modal(&mut self, direction: i32) {
        let Some(modal) = self.modal.clone() else {
            return;
        };
        let limit = self.max_modal_scroll(&modal);
        if let Some(scroll) = self.modal.as_mut().map(modal_scroll) {
            *scroll = if direction < 0 {
                scroll.saturating_sub(1)
            } else {
                (*scroll + 1).min(limit)
            };
        }
    }

    fn open_selected(&mut self) {
        if self.tab == TIMELINE_TAB {
            let event = self.selected[TIMELINE_TAB].min(data::TIMELINE.len() - 1);
            self.modal = Some(Modal::Timeline { event, scroll: 0 });
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
                if self.tab == TIMELINE_TAB {
                    if self.selected[self.tab] == index {
                        self.open_selected();
                    } else {
                        self.selected[self.tab] = index;
                    }
                }
            }
            // A click that lands on the open dialog itself, on anything but one
            // of its links: it is already where the reader wants to be.
            Some(hit::HitTarget::Dialog) => {}
            // Clicks outside any region close an open dialog.
            None => {
                if self.modal.is_some() {
                    self.modal = None;
                }
            }
        }
    }

    /// Rendered height of each row of the current list tab.
    fn item_heights(&self) -> Vec<usize> {
        if self.tab != TIMELINE_TAB {
            return Vec::new();
        }
        data::TIMELINE
            .iter()
            .map(|event| card_height(event, self.cols))
            .collect()
    }

    /// Re-anchors the scroll of whatever is on screen. A resize moves the
    /// bottom of every viewport, so a scroll that ended at the last line can
    /// be left pointing past it — and the list pane's selection can be left
    /// off screen entirely.
    pub fn clamp_scroll(&mut self) {
        if let Some(modal) = self.modal.clone() {
            let limit = self.max_modal_scroll(&modal);
            if let Some(scroll) = self.modal.as_mut().map(modal_scroll) {
                *scroll = (*scroll).min(limit);
            }
        }
        if self.tab != TIMELINE_TAB {
            self.scroll[self.tab] = self.scroll[self.tab].min(self.max_scroll());
            return;
        }
        let heights = self.item_heights();
        let Some(last) = heights.len().checked_sub(1) else {
            return;
        };
        let selected = self.selected[self.tab].min(last);
        self.selected[self.tab] = selected;
        let viewport = self.viewport().max(1);
        // Scrolling up: never leave the selection above the top item.
        let mut scroll = self.scroll[self.tab].min(selected);
        while scroll < selected && heights[scroll..=selected].iter().sum::<usize>() > viewport {
            scroll += 1;
        }
        self.scroll[self.tab] = scroll;
    }
}

fn modal_scroll(modal: &mut Modal) -> &mut usize {
    match modal {
        Modal::Timeline { scroll, .. } | Modal::Help { scroll } => scroll,
    }
}
