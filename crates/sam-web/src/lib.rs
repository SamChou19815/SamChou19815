//! The developersam.com app: a Leptos site set in the terminal's monospace
//! type, compiled to wasm and hosted by the Next.js site.
//!
//! The [`App`] state machine is the heart: which tab is in front, which card is
//! selected, and whether a post is open. It holds no scroll position of its own
//! — the pane under the header is an ordinary scrolling box, so the browser
//! remembers where the visitor is and the app only ever asks it to move
//! ([`HostEvent::Scroll`]). What renders is a pure function of the state; every
//! measure of the layout is CSS, in `ch` and `lh`, that the browser resolves
//! against the real viewport.
//!
//! Routing: the app owns a set of site paths ([`TAB_ROUTES`] and permalinks),
//! publishes the one it is on so the URL bar can follow, and accepts a path
//! from the host (a URL entered, a link followed, the back button) through
//! [`request_route`].

pub mod crypt;
pub mod data;
pub mod highlight;
pub mod hit;
pub mod markdown;
pub mod posts;
pub mod shell;
pub mod site_path;
pub mod style;
pub mod theme;

#[cfg(target_arch = "wasm32")]
pub mod ffi;
#[cfg(target_arch = "wasm32")]
pub mod ui;

pub use site_path::SitePath;

/// The tabs' names, encrypted like every other string so the binary spells out
/// none of the site's structure — read one back with `.decrypt()`.
pub const TAB_NAMES: [crypt::EncryptedString; 4] = [
    encrypted_str!("About"),
    encrypted_str!("Timeline"),
    encrypted_str!("Blog"),
    encrypted_str!("Help"),
];
pub const TAB_COUNT: usize = 4;
pub const ABOUT_TAB: usize = 0;
pub const TIMELINE_TAB: usize = 1;
pub const BLOG_TAB: usize = 2;
pub const HELP_TAB: usize = 3;

/// The site path each tab is served at. The web front-end keeps the URL bar on
/// whatever the app is showing, so every view the app can be in has to be a
/// place the site can be entered at — see [`App::route`] and [`App::go_to`].
pub const TAB_ROUTES: [crypt::EncryptedString; TAB_COUNT] = [
    encrypted_str!("/about"),
    encrypted_str!("/timeline"),
    encrypted_str!("/blog"),
    encrypted_str!("/help"),
];

/// Where the blog index lives, and the prefix every post's permalink shares.
pub const BLOG_ROUTE: crypt::EncryptedString = encrypted_str!("/blog");

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

/// How far a keystroke moves the pane. The wheel is not here: the pane is a
/// scrolling box, so a wheel, a trackpad, a drag and a scrollbar are all the
/// browser's business and the app never hears about them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scroll {
    /// Rows, negative up.
    Rows(i32),
    /// Screenfuls, negative up.
    Pages(i32),
    Top,
    Bottom,
}

/// Something only the browser can do. The app hands these to the host one at a
/// time; the front-end applies them as they arrive.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HostEvent {
    /// Open a URL that is not a view of this app.
    Open(String),
    /// Load one of this app's own views, as the browser loads a page. Only the
    /// touch build asks for this: it draws one view, whole, and has nowhere to
    /// put another — so moving between them is a navigation rather than a
    /// [`HostEvent::Route`] over a view that changed underneath.
    Navigate(SitePath),
    /// Put the URL bar and the document title on a view.
    Route {
        replace: bool,
        path: SitePath,
        title: String,
    },
    /// Move the pane the app is showing.
    Scroll(Scroll),
    /// Bring the selected card into view, and no further: arrowing down a list
    /// slides the pane by a card, not by a screenful.
    Reveal,
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
    CURRENT_ROUTE.with(|route| *route.borrow_mut() = None);
}

/// The one view the app is showing: a tab, or a post open in the reader — never
/// both. The reader is a mode of the Blog tab, so an open post carries that tab
/// with it rather than sitting in a field beside one; showing anything else
/// replaces this whole value, which is what closes the reader without anyone
/// having to remember to.
#[derive(Clone, PartialEq, Eq)]
pub enum Screen {
    Tab(usize),
    Post(usize),
}

impl Screen {
    /// The tab the header marks as the one in front. A post is read on the Blog
    /// tab, so an open post marks that one.
    pub fn tab(&self) -> usize {
        match self {
            Screen::Tab(tab) => *tab,
            Screen::Post(_) => BLOG_TAB,
        }
    }

    /// The post in the reader, if this view is one.
    pub fn reader(&self) -> Option<usize> {
        match self {
            Screen::Tab(_) => None,
            Screen::Post(post) => Some(*post),
        }
    }

    /// This view as a site path — what the URL bar should read. An open post is
    /// its own permalink; a tab is the path it is served at.
    pub fn route(&self) -> SitePath {
        match self {
            Screen::Tab(tab) => SitePath::new(TAB_ROUTES[*tab].decrypt()),
            Screen::Post(post) => posts::POSTS[*post].path(),
        }
    }
}

thread_local! {
    /// Work for the host, taken one event at a time by the front-end.
    static HOST_EVENTS: std::cell::RefCell<std::collections::VecDeque<HostEvent>> =
        const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
    /// Whether this run has put the URL bar on a view yet. The first one
    /// replaces, so booting from `/` leaves no shell entry behind for the back
    /// button; every later one pushes.
    static ROUTE_SYNCED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// A view the host has asked for — a URL entered, a link followed, or the
    /// back button. Applied by the next [`App`] to look, which is either the
    /// one being built ([`App::new`]) or the one handling the next event.
    static PENDING_ROUTE: std::cell::RefCell<Option<SitePath>> =
        const { std::cell::RefCell::new(None) };
    /// Set when the host has decided the visitor has left the app — the back
    /// button landing somewhere the app has no view for. Applied by the next
    /// frame, so the app exits through its own path.
    static PENDING_QUIT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The view the app is on, republished every frame for the host to read.
    /// `None` until the first one is drawn.
    static CURRENT_ROUTE: std::cell::RefCell<Option<SitePath>> =
        const { std::cell::RefCell::new(None) };
}

/// Asks the app to show the view at `path`. Takes effect on the next frame.
pub fn request_route(path: &SitePath) {
    PENDING_ROUTE.with(|pending| *pending.borrow_mut() = Some(path.clone()));
}

/// Records the view a frame is about to draw, queueing a [`HostEvent::Route`]
/// whenever it changes so the URL bar and the document title follow the app.
pub fn publish_route(route: SitePath) {
    let changed = CURRENT_ROUTE.with(|current| {
        let mut current = current.borrow_mut();
        if current.as_ref() == Some(&route) {
            return false;
        }
        *current = Some(route.clone());
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
pub(crate) fn take_pending_route() -> Option<SitePath> {
    PENDING_ROUTE.with(|pending| pending.borrow_mut().take())
}

/// Asks the app to exit. Takes effect on the next frame.
pub fn request_quit() {
    PENDING_QUIT.with(|pending| pending.set(true));
}

/// Takes the host's exit request, leaving nothing behind.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn take_pending_quit() -> bool {
    PENDING_QUIT.with(|pending| pending.replace(false))
}

/// What [`App::selected`] reads while nothing is: an index no list can reach,
/// so every `index == selected` test a card makes comes out false. Only the
/// touch build ever sees it — a page drawn once has nothing to move.
const NO_SELECTION: usize = usize::MAX;

/// Clone is used to snapshot state for pure rendering.
#[derive(Clone)]
pub struct App {
    /// The view on screen. Everything that changes what the app is showing goes
    /// through [`App::switch_tab`] or [`App::open_post`], each of which replaces
    /// this outright — so the app can never be showing a tab with a post's
    /// chrome still over it.
    screen: Screen,
    /// The selected card of the two list tabs. Kept across a tab switch, and
    /// across a post being opened and closed, so a tab is returned to where it
    /// was left.
    selected: [usize; TAB_COUNT],
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
            screen: Screen::Tab(ABOUT_TAB),
            selected: [0; TAB_COUNT],
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

    /// The app as the touch build shows it: the view at `path`, and no state
    /// beyond it. Nothing is selected — a page drawn once has no cursor to move.
    pub fn page(path: &SitePath) -> Self {
        let mut app = App {
            screen: Screen::Tab(ABOUT_TAB),
            selected: [NO_SELECTION; TAB_COUNT],
            quit: false,
            actions: Vec::new(),
        };
        app.go_to(path);
        app.selected = [NO_SELECTION; TAB_COUNT];
        app
    }

    /// The browser errand a tap on the page becomes. Everything the full-screen
    /// app would have handled itself — a tab, a card, the reader's way out — is
    /// a navigation here: the page holds one view, so reaching another one means
    /// loading it.
    pub fn tap(&self, target: &hit::HitTarget) -> Option<HostEvent> {
        match target {
            hit::HitTarget::Tab(index) => Some(HostEvent::Navigate(SitePath::new(
                TAB_ROUTES[*index].decrypt(),
            ))),
            // Back to the index the post was opened from, which is where Esc
            // and `q` leave a reader that has a screen to go back to.
            hit::HitTarget::Close => Some(HostEvent::Navigate(SitePath::new(BLOG_ROUTE.decrypt()))),
            hit::HitTarget::Link(url) => errand_for(url),
            hit::HitTarget::Item(index) => self.item_errand(*index),
        }
    }

    /// What a click on a region of the current frame does in the full-screen
    /// app: a pointer names the card it means by landing on it, so selecting
    /// and opening are one act.
    pub fn activate(&mut self, target: &hit::HitTarget) {
        match target {
            hit::HitTarget::Link(url) => {
                self.actions.push(Action::OpenUrl(url.clone()));
            }
            hit::HitTarget::Tab(index) => self.switch_tab(*index),
            // The same way out `q` and Esc take, for a pointer that has
            // neither: a phone reads posts with nothing but taps.
            hit::HitTarget::Close => self.close_reader(),
            hit::HitTarget::Item(index) => {
                if matches!(self.tab(), TIMELINE_TAB | BLOG_TAB) {
                    self.selected[self.tab()] = *index;
                    self.open_selected();
                }
            }
        }
    }

    /// Where tapping a whole card leads: the post it names, or — on the
    /// timeline, whose cards are their own detail view — the first of its links,
    /// exactly as Enter opens them.
    fn item_errand(&self, index: usize) -> Option<HostEvent> {
        match self.tab() {
            BLOG_TAB => {
                let post = posts::POSTS.get(index)?;
                if post.is_external() {
                    errand_for(&post.url())
                } else {
                    Some(HostEvent::Navigate(post.path()))
                }
            }
            TIMELINE_TAB => {
                let event = data::TIMELINE.get(index)?;
                errand_for(&event.links.first()?.url.decrypt())
            }
            _ => None,
        }
    }

    /// The tab the header marks as the one in front — the Blog tab whenever a
    /// post is open, since the reader is a mode of it.
    pub fn tab(&self) -> usize {
        self.screen.tab()
    }

    /// The post open in the reader, if one is.
    pub fn reader(&self) -> Option<usize> {
        self.screen.reader()
    }

    /// The view the app is on, as a site path — what the URL bar should read.
    pub fn route(&self) -> SitePath {
        self.screen.route()
    }

    /// Shows the view a site path names, and reports whether it named one. A
    /// permalink opens its post in the reader; the blog index and the other tabs
    /// are tabs, so landing on one is what closes it.
    pub fn go_to(&mut self, path: &SitePath) -> bool {
        let Some(screen) = screen_at(path) else {
            return false;
        };
        match screen {
            Screen::Tab(tab) => self.switch_tab(tab),
            Screen::Post(post) => self.open_post(post),
        }
        true
    }

    pub fn selected(&self, tab: usize) -> usize {
        self.selected[tab]
    }

    /// Holds the selection to the cards the pane is actually showing, which the
    /// front-end reports as the visitor scrolls. Without it the selection stays
    /// where the scrolling started and the next arrow press snaps the pane back
    /// to it — the jump that makes wheel scrolling feel broken.
    pub fn selection_in_view(&mut self, first: usize, last: usize) {
        let tab = self.tab();
        if self.reader().is_some() || !matches!(tab, TIMELINE_TAB | BLOG_TAB) {
            return;
        }
        if self.selected[tab] != NO_SELECTION && first <= last {
            self.selected[tab] = self.selected[tab].clamp(first, last);
        }
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

    /// Feeds one keystroke into the state machine.
    pub fn handle_key(&mut self, key: Key, mods: Mods) {
        if mods.ctrl && matches!(key, Key::Char('c') | Key::Char('d')) {
            self.quit = true;
            return;
        }
        if self.reader().is_some() {
            self.on_reader_key(key);
            return;
        }
        match key {
            Key::Left | Key::Char('h') => self.switch_tab(self.tab() + TAB_COUNT - 1),
            Key::Right | Key::Char('l') | Key::Tab => self.switch_tab(self.tab() + 1),
            Key::BackTab => self.switch_tab(self.tab() + TAB_COUNT - 1),
            Key::Esc => {}
            Key::Char('?') => self.switch_tab(HELP_TAB),
            Key::Char('q') => self.quit = true,
            // On the Timeline the digits belong to the selected card's links,
            // every one of them; the tabs stay a keystroke away on the arrows
            // and Tab.
            Key::Char(c @ '1'..='9') if self.tab() == TIMELINE_TAB => {
                let index = c as usize - '1' as usize;
                let event = self.selected[TIMELINE_TAB].min(data::TIMELINE.len() - 1);
                if let Some(link) = data::TIMELINE[event].links.get(index) {
                    self.actions.push(Action::OpenUrl(link.url.decrypt()));
                }
            }
            Key::Char(c @ '1'..='4') => self.switch_tab(c as usize - '1' as usize),
            Key::Up | Key::Char('k') => self.step(-1),
            Key::Down | Key::Char('j') => self.step(1),
            Key::PageUp => scroll(Scroll::Pages(-1)),
            Key::PageDown => scroll(Scroll::Pages(1)),
            Key::Home | Key::Char('g') => self.jump_to_edge(0),
            Key::End | Key::Char('G') => self.jump_to_edge(usize::MAX),
            Key::Enter => self.open_selected(),
            _ => {}
        }
    }

    fn on_reader_key(&mut self, key: Key) {
        match key {
            // `q` closes the reader rather than quitting the app: the way out of
            // a post is the way out of anything else the app opens.
            Key::Esc | Key::Backspace | Key::Char('q') | Key::Left | Key::Char('h') => {
                self.close_reader();
            }
            Key::Char('?') => self.switch_tab(HELP_TAB),
            Key::Up | Key::Char('k') => scroll(Scroll::Rows(-1)),
            Key::Down | Key::Char('j') => scroll(Scroll::Rows(1)),
            Key::PageUp => scroll(Scroll::Pages(-1)),
            Key::PageDown => scroll(Scroll::Pages(1)),
            Key::Home | Key::Char('g') => scroll(Scroll::Top),
            Key::End | Key::Char('G') => scroll(Scroll::Bottom),
            _ => {}
        }
    }

    /// Shows a tab, whole. A tab is a view in its own right rather than a layer
    /// under whatever else is up, so this replaces the screen — and an open post
    /// goes with it.
    fn switch_tab(&mut self, next: usize) {
        self.screen = Screen::Tab(next % TAB_COUNT);
    }

    /// Opens a post in the reader, over the Blog tab, with the index behind it
    /// left on the card the post was opened from — which is what it is scrolled
    /// to when the reader closes.
    fn open_post(&mut self, post: usize) {
        self.switch_tab(BLOG_TAB);
        self.selected[BLOG_TAB] = post;
        self.screen = Screen::Post(post);
    }

    /// Leaves the reader for the index the post was opened from.
    fn close_reader(&mut self) {
        self.switch_tab(BLOG_TAB);
        push_host_event(HostEvent::Reveal);
    }

    /// One step of the arrow keys: the next card on a list tab, the next row on
    /// a tab that is only text.
    fn step(&mut self, direction: i32) {
        let Some(len) = self.list_len() else {
            scroll(Scroll::Rows(direction));
            return;
        };
        let selected = self.selected[self.tab()];
        self.select(if direction < 0 {
            selected.saturating_sub(1)
        } else {
            (selected + 1).min(len.saturating_sub(1))
        });
    }

    /// Selects a card of the current list tab and asks for it to be brought
    /// into view.
    fn select(&mut self, index: usize) {
        self.selected[self.tab()] = index;
        push_host_event(HostEvent::Reveal);
    }

    /// Item count of the current tab, for the tabs that are lists.
    fn list_len(&self) -> Option<usize> {
        match self.tab() {
            TIMELINE_TAB => Some(data::TIMELINE.len()),
            BLOG_TAB => Some(posts::POSTS.len()),
            _ => None,
        }
    }

    fn jump_to_edge(&mut self, target: usize) {
        match self.list_len() {
            Some(len) => self.select(target.min(len.saturating_sub(1))),
            None => scroll(if target == 0 {
                Scroll::Top
            } else {
                Scroll::Bottom
            }),
        }
    }

    fn open_selected(&mut self) {
        match self.tab() {
            BLOG_TAB => {
                let post = self.selected[BLOG_TAB].min(posts::POSTS.len() - 1);
                if posts::POSTS[post].is_external() {
                    let url = posts::POSTS[post].url();
                    self.actions.push(Action::OpenUrl(url));
                } else {
                    self.open_post(post);
                }
            }
            // A timeline card is its own detail view, so the most Enter can ask
            // of one is what its buttons are for: open the first link.
            TIMELINE_TAB => {
                let event = self.selected[TIMELINE_TAB].min(data::TIMELINE.len() - 1);
                if let Some(link) = data::TIMELINE[event].links.first() {
                    self.actions.push(Action::OpenUrl(link.url.decrypt()));
                }
            }
            _ => {}
        }
    }
}

/// Asks the host to move the pane.
fn scroll(by: Scroll) {
    push_host_event(HostEvent::Scroll(by));
}

/// The view a site path names, if the app has one — a post at the top of it,
/// since a permalink says which post to read and not where in it to start. An
/// unknown post still asks for the blog, so it lands on the index; a path that
/// is no view at all — `/`, `/budget` — belongs to the browser, not the app.
fn screen_at(path: &SitePath) -> Option<Screen> {
    if let Some(post) = posts::find(path) {
        return Some(Screen::Post(post));
    }
    TAB_ROUTES
        .iter()
        .position(|route| route.decrypt() == path.as_str())
        // Anything else under the blog — a post that has since been unpublished,
        // say — still asked for the blog, so the index is where it lands.
        .or_else(|| {
            path.as_str()
                .strip_prefix(BLOG_ROUTE.decrypt().as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
                .then_some(BLOG_TAB)
        })
        .map(Screen::Tab)
}

/// Whether the app has a view at `path`. The host asks before following a link
/// or a back button itself rather than handing it to the browser, so the two
/// never disagree about what this app is responsible for.
pub fn has_view(path: &SitePath) -> bool {
    screen_at(path).is_some()
}

/// What the document is called while the shell, rather than a view, is up.
/// Encrypted like every other title, so the binary names no page.
pub const SHELL_TITLE: crypt::EncryptedString = encrypted_str!("Developer Sam — Terminal");

/// What the document is called while the view at `path` is on screen. The post
/// titles come from `posts.rs`, which build.rs compiles out of the same sources
/// the site renders, so the tab and the page cannot disagree.
pub fn title_for(path: &SitePath) -> String {
    match screen_at(path) {
        Some(Screen::Post(post)) => {
            format!("{} | {}", posts::POSTS[post].title(), posts::blog_title())
        }
        Some(Screen::Tab(ABOUT_TAB)) => encrypted_str!("About | Developer Sam").to_string(),
        Some(Screen::Tab(TIMELINE_TAB)) => encrypted_str!("Timeline | Developer Sam").to_string(),
        Some(Screen::Tab(HELP_TAB)) => encrypted_str!("Help | Developer Sam").to_string(),
        // The blog index, and anything else under it that is no longer a post.
        Some(Screen::Tab(_)) => posts::blog_title().to_string(),
        None => SHELL_TITLE.to_string(),
    }
}

/// A side effect requested by the app (opening a link).
#[derive(Clone, PartialEq, Eq)]
pub enum Action {
    OpenUrl(String),
}

/// Where activating a link should lead.
pub enum LinkTarget {
    /// A view of this app: follow it here, without touching the browser.
    View(SitePath),
    /// Somewhere else on the web: the host opens it in a new tab.
    External(String),
    /// Neither, so nothing happens. The page renders whatever bytes reach it,
    /// and a `javascript:` URL would run in that document — so anything but
    /// http(s) is refused at the source rather than at the host's `window.open`.
    Ignore,
}

/// [`link_target`] as the touch build acts on it: a view of this app is loaded
/// as a page, since that build has nowhere to put a second one, and anything
/// else is still the browser's to open.
pub(crate) fn errand_for(url: &str) -> Option<HostEvent> {
    match link_target(url) {
        LinkTarget::View(path) => Some(HostEvent::Navigate(path)),
        LinkTarget::External(url) => Some(HostEvent::Open(url)),
        LinkTarget::Ignore => None,
    }
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
/// carrying the host.
fn site_path(url: &str) -> Option<SitePath> {
    if url.starts_with('/') {
        return Some(SitePath::new(url));
    }
    let rest = strip_prefix_ignore_case(url, "https://")
        .or_else(|| strip_prefix_ignore_case(url, "http://"))?;
    let rest = strip_prefix_ignore_case(rest, "www.").unwrap_or(rest);
    let rest = strip_prefix_ignore_case(rest, "developersam.com")?;
    match rest {
        "" => Some(SitePath::root()),
        _ if rest.starts_with('/') => Some(SitePath::new(rest)),
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
