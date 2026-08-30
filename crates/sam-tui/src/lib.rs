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

/// The site path each tab is served at. The web front-end keeps the URL bar on
/// whatever the app is showing, so every view the app can be in has to be a
/// place the site can be entered at — see [`App::route`] and [`App::go_to`].
pub const TAB_ROUTES: [&str; TAB_COUNT] = ["/about", "/timeline", "/blog"];

/// Where the blog index lives, and the prefix every post's permalink shares.
pub const BLOG_ROUTE: &str = "/blog";

/// Width assumed before the first resize event tells us the real one.
const ASSUMED_COLS: u16 = 80;

/// Rows the wheel moves at a time — the terminal convention, and few enough
/// that the line the eye was on is still on screen afterwards. A touch host
/// meters a drag into notches of this many rows, so the content keeps up with
/// the finger; see `wheelRows` in [`ffi`].
pub const WHEEL_ROWS: usize = 3;

/// The widest a centered column gets, mirroring `max-w-screen-lg` on the web.
/// A maximized terminal is several times wider than a comfortable measure, and
/// prose that runs the whole way across is hard to track from the end of one
/// row to the start of the next.
const MAX_COLUMN_COLS: usize = 88;

/// Rows the pane spends on its own chrome, whatever is inside it: its border
/// top and bottom, and its title row. The header's rows and the status bar's
/// are on top of these and depend on the size — see [`view::header_rows`] and
/// [`view::status_rows`].
const PANE_CHROME_ROWS: usize = 3;

/// The cells a timeline card's rail takes before its body starts — `│` and the
/// two spaces that set the body off from it (`view::RAIL`).
const GUTTER_COLS: usize = 3;

/// The air a card keeps inside its own left and right edges. A card is only
/// drawn as a box when it is the selected one, and this is what holds that tint
/// off its marker and its category tag instead of running it flush to both.
const CARD_PAD_COLS: usize = 1;

/// Width of a centered column inside the pane's border and its body's padding.
fn column_width(cols: u16) -> usize {
    let cols = if cols == 0 { ASSUMED_COLS } else { cols };
    (cols as usize).saturating_sub(4).clamp(12, MAX_COLUMN_COLS)
}

/// Width of the timeline's centered column: the rail and the card bodies
/// beside it. The same measure the blog's column keeps, so moving between the
/// two list tabs does not move the column they are read down.
pub fn timeline_column_width(cols: u16) -> usize {
    column_width(cols)
}

/// Usable width inside a card's text column: the column, less the card's
/// padding and its rail.
pub fn content_width(cols: u16) -> usize {
    timeline_column_width(cols)
        .saturating_sub(GUTTER_COLS + 2 * CARD_PAD_COLS)
        .max(8)
}

/// Width of the blog's centered column — the index's cards and the reader's
/// prose — inside the pane's border and its body's padding.
pub fn blog_column_width(cols: u16) -> usize {
    column_width(cols)
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

/// Rows of a timeline card, mirroring the homepage card: the blank row that
/// opens it, the title, the time as a subheader, then the artwork, the wrapped
/// detail and a button row — each behind a blank rail row of its own — and the
/// blank row that closes it. Sections a card leaves out cost it nothing,
/// spacer included. The rows top and bottom are both the air between one card
/// and the next and, on the selected card, the padding inside its tint.
pub fn card_height(event: &data::TimelineEvent, cols: u16) -> usize {
    let inner = content_width(cols);
    let detail = event.detail.map_or(0, |detail| wrapped_rows(detail, inner));
    let links = if event.links.is_empty() {
        0
    } else {
        wrapped_rows(&link_row_label(event.links), inner)
    };
    let image = image::rows(event.image, image::thumbnail_bounds(cols));
    let body: usize = [image, detail, links]
        .into_iter()
        .filter(|rows| *rows > 0)
        .map(|rows| rows + 1)
        .sum();
    // Top padding, title, time, the body, then bottom padding.
    1 + 1 + 1 + body + 1
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

/// Something only the browser can do. The wasm bridge hands these to the host
/// one line at a time; [`HostEvent::encode`] is the wire form.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HostEvent {
    /// Open a URL that is not a view of this app.
    Open(String),
    /// Put the URL bar and the document title on a view.
    Route {
        replace: bool,
        path: String,
        title: String,
    },
}

impl HostEvent {
    /// `open <url>` or `route push|replace <path>\t<title>`. Space-separated
    /// with a tab before the title, since a title may contain spaces and a
    /// path never contains a tab.
    pub fn encode(&self) -> String {
        match self {
            HostEvent::Open(url) => format!("open {url}"),
            HostEvent::Route {
                replace,
                path,
                title,
            } => {
                let verb = if *replace { "replace" } else { "push" };
                format!("route {verb} {path}\t{title}")
            }
        }
    }
}

/// Queues work for the host.
pub fn push_host_event(event: HostEvent) {
    HOST_EVENTS.with(|queue| queue.borrow_mut().push_back(event));
}

/// Takes the next thing the host has to do, if any.
pub fn poll_host_event() -> Option<HostEvent> {
    HOST_EVENTS.with(|queue| queue.borrow_mut().pop_front())
}

/// Forgets that this run ever synced a route, so the next one replaces rather
/// than pushes. Called when a session boots.
pub fn reset_route_sync() {
    ROUTE_SYNCED.with(|synced| synced.set(false));
    CURRENT_ROUTE.with(|route| route.borrow_mut().clear());
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
    /// Work for the host, drained one line at a time by the wasm bridge.
    static HOST_EVENTS: std::cell::RefCell<std::collections::VecDeque<HostEvent>> =
        const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
    /// Whether this run has put the URL bar on a view yet. The first one
    /// replaces, so booting from `/` leaves no shell entry behind for the back
    /// button; every later one pushes.
    static ROUTE_SYNCED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// A view the host has asked for — a URL entered, a link followed, or the
    /// back button. Applied by the next [`App`] to look, which is either the
    /// one being built ([`App::new`]) or the one handling the next event.
    static PENDING_ROUTE: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
    /// Set when the host has decided the visitor has left the app — the back
    /// button landing somewhere the app has no view for. Applied by the next
    /// frame, so the app exits through its own path and restores the screen.
    static PENDING_QUIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The view the app is on, republished every frame for the host to read.
    static CURRENT_ROUTE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Asks the app to show the view at `path`. Takes effect on the next frame.
pub fn request_route(path: &str) {
    PENDING_ROUTE.with(|pending| *pending.borrow_mut() = Some(path.to_string()));
}

/// Records the view a frame is about to draw, queueing a [`HostEvent::Route`]
/// whenever it changes so the URL bar and the document title follow the app.
pub fn publish_route(route: String) {
    let changed = CURRENT_ROUTE.with(|current| {
        let mut current = current.borrow_mut();
        if *current == route {
            return false;
        }
        current.clone_from(&route);
        true
    });
    if !changed {
        return;
    }
    let replace = !ROUTE_SYNCED.with(|synced| synced.replace(true));
    let title = title_for(&route);
    push_host_event(HostEvent::Route {
        replace,
        path: route,
        title,
    });
}

/// Takes whatever view the host last asked for, leaving nothing behind: a
/// request is applied once, by the first frame to look.
pub(crate) fn take_pending_route() -> Option<String> {
    PENDING_ROUTE.with(|pending| pending.borrow_mut().take())
}

/// Asks the app to exit. Takes effect on the next frame.
pub fn request_quit() {
    PENDING_QUIT.with(|pending| pending.set(true));
}

/// Takes the host's exit request, leaving nothing behind.
pub(crate) fn take_pending_quit() -> bool {
    PENDING_QUIT.with(|pending| pending.replace(false))
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
        let mut app = App {
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
        };
        // A URL entered before the app booted names the view it opens on, so
        // the first frame a deep link draws is already the right one.
        if let Some(route) = take_pending_route() {
            app.go_to(&route);
        }
        app
    }

    /// The view the app is on, as a site path — what the URL bar should read.
    /// An open post is its own permalink; everything else is its tab.
    pub fn route(&self) -> String {
        match &self.reader {
            Some(reader) => posts::POSTS[reader.post].path(),
            None => TAB_ROUTES[self.tab].to_string(),
        }
    }

    /// Shows the view a site path names, and reports whether it named one. A
    /// permalink opens its post in the reader; the blog index and the other
    /// tabs close it.
    pub fn go_to(&mut self, path: &str) -> bool {
        match view_at(path) {
            Some(View::Post(post)) => {
                self.switch_tab(BLOG_TAB);
                self.selected[BLOG_TAB] = post;
                self.reveal_blog_selection();
                self.reader = Some(Reader { post, scroll: 0 });
                true
            }
            Some(View::Tab(tab)) => {
                self.switch_tab(tab);
                self.reader = None;
                true
            }
            None => false,
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
        // The header is measured at the width the layout will use, so the two
        // agree about where the pane starts even in the frame before the first
        // resize event lands.
        let cols = if self.cols == 0 {
            ASSUMED_COLS
        } else {
            self.cols
        };
        (rows as usize).saturating_sub(
            PANE_CHROME_ROWS + view::header_rows(cols, rows) + view::status_rows(cols, rows),
        )
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

    /// Drains the side effects the latest input produced. A link naming a view
    /// of this app is followed here rather than handed to the browser, so the
    /// two never disagree about what this app is responsible for.
    pub fn take_actions(&mut self) {
        for Action::OpenUrl(url) in std::mem::take(&mut self.actions) {
            match link_target(&url) {
                LinkTarget::View(path) => {
                    self.go_to(&path);
                }
                LinkTarget::External(url) => push_host_event(HostEvent::Open(url)),
                LinkTarget::Ignore => {}
            }
        }
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
            // The same way out `q` and Esc take, for a pointer that has
            // neither: a phone reads posts with nothing but taps.
            Some(hit::HitTarget::Close) => self.reader = None,
            // A pointer names the card it means by landing on it, so there is
            // nothing left for a second click to say. Selecting on the first
            // click and opening only on the second is what left every card on
            // a phone needing two taps.
            Some(hit::HitTarget::Item(index)) => {
                if matches!(self.tab, TIMELINE_TAB | BLOG_TAB) {
                    self.select(index);
                    self.open_selected();
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

/// What a site path opens.
enum View {
    Tab(usize),
    Post(usize),
}

/// The view a site path names, if the app has one. An unknown post still asks
/// for the blog, so it lands on the index; a path that is no view at all —
/// `/`, `/budget` — belongs to the browser, not the app.
fn view_at(path: &str) -> Option<View> {
    let path = path.strip_suffix('/').unwrap_or(path);
    if let Some(post) = posts::find(path) {
        return Some(View::Post(post));
    }
    TAB_ROUTES
        .iter()
        .position(|route| *route == path)
        // Anything else under the blog — a post that has since been unpublished,
        // say — still asked for the blog, so the index is where it lands.
        .or_else(|| {
            path.strip_prefix(BLOG_ROUTE)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
                .then_some(BLOG_TAB)
        })
        .map(View::Tab)
}

/// Whether the app has a view at `path`. The host asks before following a link
/// or a back button itself rather than handing it to the browser, so the two
/// never disagree about what this app is responsible for.
pub fn has_view(path: &str) -> bool {
    view_at(path).is_some()
}

/// What the document is called while the shell, rather than a view, is up.
pub const SHELL_TITLE: &str = "Developer Sam — Terminal";

/// What the document is called while the view at `path` is on screen. The post
/// titles come from `posts.rs`, which build.rs compiles out of the same
/// sources the site renders, so the tab and the page cannot disagree.
pub fn title_for(path: &str) -> String {
    let path = path.strip_suffix('/').unwrap_or(path);
    if let Some(post) = posts::find(path) {
        return format!("{} | {}", posts::POSTS[post].title, posts::BLOG_TITLE);
    }
    match view_at(path) {
        Some(View::Tab(ABOUT_TAB)) => "About | Developer Sam".to_string(),
        Some(View::Tab(TIMELINE_TAB)) => "Timeline | Developer Sam".to_string(),
        // The blog index, and anything else under it that is no longer a post.
        Some(_) => posts::BLOG_TITLE.to_string(),
        None => SHELL_TITLE.to_string(),
    }
}

/// Where activating a link should lead.
pub enum LinkTarget {
    /// A view of this app: follow it here, without touching the browser.
    View(String),
    /// Somewhere else on the web: the host opens it in a new tab.
    External(String),
    /// Neither, so nothing happens. The page renders whatever bytes reach it,
    /// and a `javascript:` URL would run in that document — so anything but
    /// http(s) is refused at the source rather than at the host's `window.open`.
    Ignore,
}

/// Where the URL a link carries should lead.
pub fn link_target(url: &str) -> LinkTarget {
    if let Some(path) = site_path(url) {
        if has_view(&path) {
            return LinkTarget::View(path);
        }
    }
    if starts_with_ignore_case(url, "https://") || starts_with_ignore_case(url, "http://") {
        LinkTarget::External(url.to_string())
    } else {
        LinkTarget::Ignore
    }
}

/// The site path a URL points at, if it points at this site. Both forms turn up
/// in post bodies: relative links as the author wrote them, and absolute ones
/// the native binary needs a host for.
fn site_path(url: &str) -> Option<String> {
    if url.starts_with('/') {
        return Some(url.to_string());
    }
    let rest = strip_prefix_ignore_case(url, "https://")
        .or_else(|| strip_prefix_ignore_case(url, "http://"))?;
    let rest = strip_prefix_ignore_case(rest, "www.").unwrap_or(rest);
    let rest = strip_prefix_ignore_case(rest, "developersam.com")?;
    match rest {
        "" => Some("/".to_string()),
        _ if rest.starts_with('/') => Some(rest.to_string()),
        // A different host that merely starts the same way, e.g.
        // `developersam.com.example.org`.
        _ => None,
    }
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.len() >= prefix.len() && text[..prefix.len()].eq_ignore_ascii_case(prefix)
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    starts_with_ignore_case(text, prefix).then(|| &text[prefix.len()..])
}

fn modal_scroll(modal: &mut Modal) -> &mut usize {
    match modal {
        Modal::Timeline { scroll, .. } | Modal::Help { scroll } => scroll,
    }
}
