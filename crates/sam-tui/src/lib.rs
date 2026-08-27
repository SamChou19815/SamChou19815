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
pub mod posts;
pub mod shell;
pub mod theme;
pub mod view;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};

pub const TAB_NAMES: [&str; 3] = ["About", "Timeline", "Blog"];
pub const TAB_COUNT: usize = 3;
pub const ABOUT_TAB: usize = 0;
pub const TIMELINE_TAB: usize = 1;
pub const BLOG_TAB: usize = 2;

/// Width assumed before the first resize event tells us the real one.
const ASSUMED_COLS: u16 = 80;

/// Rows the wheel moves at a time — the terminal convention, and few enough
/// that the line the eye was on is still on screen afterwards.
const WHEEL_ROWS: usize = 3;

/// Usable width inside a card's text column: the pane's border and padding
/// plus the card's gutter are spoken for before any text is drawn.
pub fn content_width(cols: u16) -> usize {
    let cols = if cols == 0 { ASSUMED_COLS } else { cols };
    (cols as usize).saturating_sub(7).max(12)
}

/// The widest the blog's column gets, mirroring `max-w-screen-lg` on the web.
/// A maximized terminal is several times wider than a comfortable measure, and
/// prose that runs the whole way across is hard to track from the end of one
/// row to the start of the next.
const BLOG_MAX_COLS: usize = 88;

/// Width of the blog's centered column — the index's cards and the reader's
/// prose — inside the pane's border and its body's padding.
pub fn blog_column_width(cols: u16) -> usize {
    let cols = if cols == 0 { ASSUMED_COLS } else { cols };
    (cols as usize).saturating_sub(4).clamp(12, BLOG_MAX_COLS)
}

/// Text width inside a blog card: the column, less the card's border and
/// padding.
pub fn blog_text_width(cols: u16) -> usize {
    blog_column_width(cols).saturating_sub(4).max(8)
}

/// Rows one blog card occupies: the blank separator above it, then its box —
/// top border, title, date, bottom border. Every card is the same height, as
/// on the web, where a card in the index carries a title and a date and
/// nothing else. Uniform heights are what let the index scroll by the row.
pub const POST_CARD_ROWS: usize = 5;

/// Rows the whole blog index occupies.
pub fn blog_rows() -> usize {
    posts::POSTS.len() * POST_CARD_ROWS
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

/// An open post, filling the content pane. The header and status bar stay put,
/// so the reader is a mode of the Blog tab rather than a dialog over it.
#[derive(Clone, PartialEq, Eq)]
pub struct Reader {
    pub post: usize,
    /// Rows of the post scrolled past the top of the body. Counting rows
    /// rather than blocks is what keeps a tall image from jumping a screenful
    /// at a time.
    pub scroll: usize,
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
    /// Vertical scroll offset for the scrolling tab panes: rows for About and
    /// for the Blog index, whose cards are all one height, and the index of
    /// the topmost card for the Timeline, whose cards are not.
    scroll: [usize; TAB_COUNT],
    /// Selected row for the two list tabs.
    selected: [usize; TAB_COUNT],
    /// The post open in the Blog tab's reader, if any.
    pub reader: Option<Reader>,
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
            reader: None,
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
        if self.reader.is_some() {
            self.on_reader_key(key);
            return;
        }
        match key {
            Key::Left | Key::Char('h') => self.switch_tab(self.tab + TAB_COUNT - 1),
            Key::Right | Key::Char('l') | Key::Tab => self.switch_tab(self.tab + 1),
            Key::BackTab => self.switch_tab(self.tab + TAB_COUNT - 1),
            Key::Esc => {}
            Key::Char('?') => self.modal = Some(Modal::Help { scroll: 0 }),
            Key::Char('q') => self.quit = true,
            Key::Char(c @ '1'..='3') => self.switch_tab(c as usize - '1' as usize),
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

    fn on_reader_key(&mut self, key: Key) {
        match key {
            // `q` closes the reader rather than quitting, matching how it
            // closes the existing dialog.
            Key::Esc | Key::Backspace | Key::Char('q') | Key::Left | Key::Char('h') => {
                self.reader = None;
            }
            Key::Char('?') => self.modal = Some(Modal::Help { scroll: 0 }),
            Key::Up | Key::Char('k') => self.scroll_reader(-1, 1),
            Key::Down | Key::Char('j') => self.scroll_reader(1, 1),
            Key::PageUp => {
                let page = self.reader_page();
                self.scroll_reader(-1, page);
            }
            Key::PageDown => {
                let page = self.reader_page();
                self.scroll_reader(1, page);
            }
            Key::Home | Key::Char('g') => {
                if let Some(reader) = &mut self.reader {
                    reader.scroll = 0;
                }
            }
            Key::End | Key::Char('G') => {
                let limit = self.max_reader_scroll();
                if let Some(reader) = &mut self.reader {
                    reader.scroll = limit;
                }
            }
            _ => {}
        }
    }

    fn scroll_reader(&mut self, direction: i32, amount: usize) {
        let limit = self.max_reader_scroll();
        if let Some(reader) = &mut self.reader {
            reader.scroll = if direction < 0 {
                reader.scroll.saturating_sub(amount)
            } else {
                (reader.scroll + amount).min(limit)
            };
        }
    }

    /// A page of the reader: a screenful less two rows, so the lines the eye
    /// was on are still there after the jump.
    fn reader_page(&self) -> usize {
        view::reader_viewport(self.viewport())
            .saturating_sub(2)
            .max(1)
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

    /// Moves list selection (Timeline, Blog) or scrolls text panes.
    fn move_selection(&mut self, direction: i32, amount: usize) {
        if let Some(len) = self.list_len() {
            let selected = self.selected[self.tab];
            self.select(if direction < 0 {
                selected.saturating_sub(amount)
            } else {
                (selected + amount).min(len.saturating_sub(1))
            });
        } else {
            self.scroll_text(direction, amount.max(3));
        }
    }

    /// Selects a card of the current list tab and brings it into view.
    fn select(&mut self, index: usize) {
        self.selected[self.tab] = index;
        if self.tab == BLOG_TAB {
            self.reveal_blog_selection();
        }
    }

    /// Scrolls the blog index just far enough to bring the selected card fully
    /// into view, and no further. Moving the least it can is what makes
    /// arrowing down the list slide the page a row at a time instead of
    /// snapping the selection to the pane's top edge.
    fn reveal_blog_selection(&mut self) {
        // A card's rows include the blank separator above it, so a revealed
        // card comes with the row of air that separates it from the one before.
        let top = self.selected[BLOG_TAB] * POST_CARD_ROWS;
        let bottom = top + POST_CARD_ROWS;
        let viewport = self.viewport().max(1);
        let scroll = &mut self.scroll[BLOG_TAB];
        if *scroll > top {
            *scroll = top;
        } else if bottom > *scroll + viewport {
            *scroll = bottom - viewport;
        }
    }

    /// Moves the selection to the nearest card the wheel left on screen.
    /// Without it the selection stays where the scrolling started and the next
    /// arrow press snaps the page back to it — the jump that makes wheel
    /// scrolling feel broken.
    fn follow_blog_scroll(&mut self) {
        let last = posts::POSTS.len().saturating_sub(1);
        let viewport = self.viewport().max(1);
        let scroll = self.scroll[BLOG_TAB];
        let first = (scroll / POST_CARD_ROWS).min(last);
        let bottom = ((scroll + viewport) / POST_CARD_ROWS)
            .saturating_sub(1)
            .clamp(first, last);
        self.selected[BLOG_TAB] = self.selected[BLOG_TAB].clamp(first, bottom);
    }

    /// Item count of the current tab, for the tabs that are lists.
    fn list_len(&self) -> Option<usize> {
        match self.tab {
            TIMELINE_TAB => Some(data::TIMELINE.len()),
            BLOG_TAB => Some(posts::POSTS.len()),
            _ => None,
        }
    }

    fn jump_to_edge(&mut self, target: usize) {
        if let Some(len) = self.list_len() {
            self.select(target.min(len.saturating_sub(1)));
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
        match self.tab {
            TIMELINE_TAB => {
                let event = self.selected[TIMELINE_TAB].min(data::TIMELINE.len() - 1);
                self.modal = Some(Modal::Timeline { event, scroll: 0 });
            }
            BLOG_TAB => {
                let post = self.selected[BLOG_TAB].min(posts::POSTS.len() - 1);
                if posts::POSTS[post].is_external() {
                    let url = posts::POSTS[post].url();
                    self.actions.push(Action::OpenUrl(url));
                } else {
                    self.reader = Some(Reader { post, scroll: 0 });
                }
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
                } else if self.reader.is_some() {
                    self.scroll_reader(direction, WHEEL_ROWS);
                } else if self.tab == BLOG_TAB {
                    // The blog moves its page under the selection rather than
                    // dragging the selection along: rolling the wheel asks to
                    // see further down the index, not to pick another post.
                    self.scroll_text(direction, WHEEL_ROWS);
                    self.follow_blog_scroll();
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
                if matches!(self.tab, TIMELINE_TAB | BLOG_TAB) {
                    if self.selected[self.tab] == index {
                        self.open_selected();
                    } else {
                        self.select(index);
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

    /// The furthest the reader scrolls: far enough to bring the last row of
    /// the post to the bottom of the body, and no further.
    fn max_reader_scroll(&self) -> usize {
        let Some(reader) = &self.reader else {
            return 0;
        };
        let viewport = view::reader_viewport(self.viewport()).max(1);
        view::reader_row_count(reader.post, self.cols).saturating_sub(viewport)
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
        let reader_limit = self.max_reader_scroll();
        if let Some(reader) = &mut self.reader {
            reader.scroll = reader.scroll.min(reader_limit);
        }
        if self.tab != TIMELINE_TAB {
            // The Blog index and the About pane both scroll by the row, and
            // the blog's selection moves with its own scroll rather than
            // dragging it, so there is nothing left to anchor here.
            if self.tab == BLOG_TAB {
                let last = posts::POSTS.len().saturating_sub(1);
                self.selected[BLOG_TAB] = self.selected[BLOG_TAB].min(last);
            }
            self.scroll[self.tab] = self.scroll[self.tab].min(self.max_scroll());
            return;
        }
        let heights: Vec<usize> = data::TIMELINE
            .iter()
            .map(|event| card_height(event, self.cols))
            .collect();
        let Some(last) = heights.len().checked_sub(1) else {
            return;
        };
        let selected = self.selected[TIMELINE_TAB].min(last);
        self.selected[TIMELINE_TAB] = selected;
        let viewport = self.viewport().max(1);
        // Scrolling up: never leave the selection above the top item.
        let mut scroll = self.scroll[TIMELINE_TAB].min(selected);
        while scroll < selected && heights[scroll..=selected].iter().sum::<usize>() > viewport {
            scroll += 1;
        }
        self.scroll[TIMELINE_TAB] = scroll;
    }
}

fn modal_scroll(modal: &mut Modal) -> &mut usize {
    match modal {
        Modal::Timeline { scroll, .. } | Modal::Help { scroll } => scroll,
    }
}
