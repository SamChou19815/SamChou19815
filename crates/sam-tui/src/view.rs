//! The iocraft component tree: one component per screen region, mirroring
//! the homepage's design ([`crate::theme`]). Interactive regions register
//! themselves with [`crate::hit`] as they paint, one `use_hit_region` call
//! each, and the containers that scroll bound them with `use_hit_clip`.

use crate::crypt::EncryptedString;
use crate::frame::UseFrame;
use crate::hit::{HitTarget, UseHit};
use crate::image::{self, Image};
use crate::site_path::SitePath;
use crate::{
    data, markdown, posts, theme, App, Reader, ABOUT_TAB, BLOG_TAB, HELP_TAB, TAB_COUNT, TAB_NAMES,
    TIMELINE_TAB,
};
use crossterm::style::Color;
use iocraft::components::MixedTextContent;
use iocraft::prelude::*;
use iocraft::AnyElement;

const TITLE: &str = " DEV SAM ";

/// The one column everything the app draws is laid out down: the wordmark and
/// the tabs, the cards, and the reader's title row. It
/// is the same measure and the same centering the pane's own body uses
/// ([`crate::column_width`]) — the body's margin is exactly what the centering
/// gives back, so every one of those lines up on the same left and right edge
/// whatever the terminal's width.
fn column_cols(cols: usize) -> u16 {
    crate::column_width(cols as u16) as u16
}

#[derive(Props, Default)]
struct ColumnProps {
    width: u16,
    children: Vec<AnyElement<'static>>,
}

/// Lays its children out in [`column_cols`], centered on the screen and inset
/// by the cell a card keeps inside its own edges ([`CARD_PAD`]), so that a line
/// of the header or of the status bar starts exactly under a card's marker
/// rather than a cell to the left of it.
#[component]
fn Column(props: &mut ColumnProps) -> impl Into<AnyElement<'static>> {
    element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            justify_content: JustifyContent::Center,
        ) {
            View(
                flex_direction: FlexDirection::Column,
                width: props.width,
                flex_shrink: 0.0_f32,
                padding_left: CARD_PAD,
                padding_right: CARD_PAD,
            ) {
                #(props.children.drain(..))
            }
        }
    }
}

fn text(content: impl ToString) -> MixedTextContent {
    MixedTextContent::new(content)
}

fn colored(content: impl ToString, color: Color) -> MixedTextContent {
    text(content).color(color)
}

fn bold_colored(content: impl ToString, color: Color) -> MixedTextContent {
    colored(content, color).weight(Weight::Bold)
}

fn muted(content: impl ToString) -> MixedTextContent {
    colored(content, theme::MUTED)
}

/// Rows of text a scrolling tab pane shows at once, given the height of the
/// pane's body. The two listing tabs open with a row of air ([`listing`]), so
/// they fit one fewer line than the body is tall — and their last screenful is
/// their last line on the last row of it.
pub fn tab_viewport(tab: usize, body_rows: usize) -> usize {
    match tab {
        ABOUT_TAB | HELP_TAB => body_rows.saturating_sub(LISTING_AIR_ROWS).max(1),
        _ => body_rows.max(1),
    }
}

/// Total number of lines a scrolling tab pane can show.
pub fn tab_line_count(tab: usize, cols: u16) -> usize {
    match tab {
        TIMELINE_TAB => data::TIMELINE
            .iter()
            .map(|event| crate::card_height(event, cols))
            .sum(),
        BLOG_TAB => crate::blog_rows(),
        ABOUT_TAB => about_lines().len(),
        HELP_TAB => help_lines(cols as usize).len(),
        _ => 0,
    }
}

// --- Leaf components that register hit regions --------------------------------

#[derive(Props, Default)]
struct TabLabelProps {
    label: String,
    selected: bool,
    index: usize,
}

#[component]
fn TabLabel(props: &TabLabelProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_region(Some(HitTarget::Tab(props.index)));
    element! {
        Text(
            content: props.label.clone(),
            color: if props.selected { theme::SELECT_FG } else { theme::SUBTLE },
            weight: if props.selected { Weight::Bold } else { Weight::Normal },
            wrap: TextWrap::NoWrap,
        )
    }
}

/// Wraps a whole list item in one click region, so clicking anywhere on a
/// card selects it — as the homepage cards do — rather than only its title.
/// A link inside the card is a descendant of it, so it paints after the card
/// and takes the click back off it.
#[derive(Props, Default)]
struct HitBlockProps {
    index: usize,
    children: Vec<AnyElement<'static>>,
}

#[component]
fn HitBlock(props: &mut HitBlockProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_region(Some(HitTarget::Item(props.index)));
    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            #(props.children.drain(..))
        }
    }
}

#[derive(Props, Default)]
struct TimelineTitleProps {
    marker: String,
    contents: Vec<MixedTextContent>,
    tag: String,
    tag_color: Option<Color>,
    selected: bool,
}

#[component]
fn TimelineTitle(props: &TimelineTitleProps) -> impl Into<AnyElement<'static>> {
    element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            padding_left: CARD_PAD,
            padding_right: CARD_PAD,
            background_color: if props.selected { Some(theme::SELECT_BG) } else { None },
        ) {
            Text(content: props.marker.clone(), color: theme::ACCENT_TEXT, weight: Weight::Bold, wrap: TextWrap::NoWrap)
            View(flex_direction: FlexDirection::Row, flex_grow: 1.0_f32) {
                MixedText(contents: props.contents.clone())
            }
            Text(
                content: props.tag.clone(),
                color: props.tag_color.unwrap_or(theme::MUTED),
                wrap: TextWrap::NoWrap,
            )
        }
    }
}

#[derive(Props, Default)]
struct ButtonProps {
    label: String,
    url: String,
}

#[component]
fn Button(props: &ButtonProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_region(Some(HitTarget::Link(props.url.clone())));
    element! {
        Text(content: props.label.clone(), color: theme::ACCENT_TEXT, weight: Weight::Bold)
    }
}

#[derive(Props, Default)]
struct LineProps {
    contents: Vec<MixedTextContent>,
    url: Option<String>,
}

/// One line of rendered markdown, clickable when it carries a link. Where it is
/// — a post in the pane, or an open dialog's body — is a question of where it
/// paints and what clipped it, so it is not a question the line has to answer.
///
/// It is one row, never two: a post's prose is wrapped into lines before it
/// gets here ([`markdown::post_blocks`]), and the listings and the dialog would
/// otherwise paint a row their scroll math has not counted — every one of those
/// counts lines. What will not fit is clipped at the pane's edge, which for the
/// About tab is what a code listing wants anyway: an indent that survives a
/// narrow screen, rather than a line broken across two rows under it.
#[component]
fn Line(props: &LineProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_region(props.url.clone().map(HitTarget::Link));
    element! {
        MixedText(contents: props.contents.clone(), wrap: TextWrap::NoWrap)
    }
}

// --- Root --------------------------------------------------------------------

#[component]
fn Root(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let app = hooks.use_state(App::new);
    let bump = hooks.use_state(|| 0u32);
    // The render loop hands the tree a max width, not a definite size, so
    // percentage width/height resolve to content. Anchor the tree to the
    // real terminal size so the pane stretches and the card fills the screen.
    let (terminal_width, terminal_height) = hooks.use_terminal_size();

    {
        let mut app = app;
        let mut bump = bump;
        hooks.use_terminal_events(move |event| {
            let current = (*app.read()).clone();
            let mut next = current;
            // Keep the state machine's idea of the size current: layout and
            // scroll math both depend on it, and the native front-end only
            // learns the size from resize events it may never receive.
            next.resize(terminal_width, terminal_height);
            // A view the host asked for — a URL entered, a link followed, the
            // back button. It cannot reach into the running app, so it leaves
            // the path here and wakes the loop with an event of its own.
            if let Some(route) = crate::take_pending_route() {
                next.go_to(&route);
            }
            // The back button left the app's views behind, so the app leaves
            // too — through its own exit, which restores the screen.
            if crate::take_pending_quit() {
                next.quit = true;
            }
            if let Some(event) = terminal_event_to_crossterm(&event) {
                next.handle_event(&event);
                // Surface OpenUrl actions to the host.
                next.take_actions();
            }
            app.set(next);
            bump.set(bump.get() + 1);
        });
    }

    if app.read().quit {
        system.exit();
    }
    let app = (*app.read()).clone();

    // Every image and every click region paints itself into an empty registry,
    // so what the host reads and what a click hits is exactly this frame.
    hooks.use_frame(image::LAYER_PANE);
    // The URL bar is one more thing the host mirrors from the frame it is
    // about to see, so it is published here with the rest of them.
    crate::publish_route(app.route());
    let cols = terminal_width as usize;
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: terminal_width,
            height: terminal_height,
            background_color: theme::SURFACE,
        ) {
            #(chrome(&app, cols, terminal_height as usize, false))
        }
    }
}

/// The header bar and the pane under it: everything the app draws for one
/// state. What bounds them is the caller's business — a screen, for the app
/// that owns one, and nothing at all for the page [`TouchRoot`] hands over.
/// `touch` is which of the two is drawing: the size answers everything else the
/// header decides, but not what a host with no keyboard has any use for.
fn chrome(app: &App, cols: usize, rows: usize, touch: bool) -> Vec<AnyElement<'static>> {
    let title = pane_title(app, cols);
    vec![
        element_to_any(element! { Header(tab: app.tab, cols: cols, rows: rows, touch: touch) }),
        element_to_any(element! {
            Pane(
                title: title,
                closable: app.reader.is_some(),
                column: column_cols(cols),
            ) {
                #(content_element(app))
            }
        }),
    ]
}

#[derive(Props, Default)]
struct TouchRootProps {
    path: SitePath,
    cols: u16,
}

/// The same app, drawn for a host that scrolls the terminal itself: one view,
/// whole, with no height of its own. Everything a screenful would have clipped
/// is simply drawn, so the visitor's finger moves the terminal's own scrollback
/// rather than asking the app for the next screenful — and there is no next
/// frame to ask for, because a page has no state to change.
#[component]
fn TouchRoot(props: &TouchRootProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_frame(image::LAYER_PANE);
    let app = App::page(&props.path, props.cols);
    crate::publish_route(app.route());
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: props.cols,
            background_color: theme::SURFACE,
        ) {
            // The header is planned against the fewest rows that are not a
            // phone's, so that width alone decides how the site is named
            // ([`touch_sized`]). A page has no screenful of rows to spend on a
            // wordmark or to run short of, and the browser reports a new height
            // every time its address bar slides away under a scrolling finger —
            // which must not be able to change a line of what is drawn.
            #(chrome(&app, props.cols as usize, PHONE_ROWS, true))
        }
    }
}

/// The view at `path` as one page, for the touch build to render once.
pub fn touch_element(path: SitePath, cols: u16) -> AnyElement<'static> {
    element!(TouchRoot(path: path, cols: cols)).into_any()
}

/// What the pane's title row names, if anything. Only the reader names itself,
/// centered over the pane: a tab is already named — and marked as the one in
/// front — by the header a row above, so a pane that repeats it spends a row of
/// chrome saying what has just been said. The title is cut to what is left
/// beside the close button, so the row stays exactly one line however narrow the
/// terminal or long the post.
fn pane_title(app: &App, cols: usize) -> PaneTitle {
    let Some(reader) = &app.reader else {
        return PaneTitle::None;
    };
    // What the title has to itself: the column the row is laid out down, less
    // the cell it keeps inside its own edges, the close button and the spacer
    // that balances it on the other side, and the space either side of the
    // title itself.
    let room = (column_cols(cols) as usize)
        .saturating_sub(2 * crate::CARD_PAD_COLS + 2 * CLOSE_LABEL.len() + 2)
        .max(MIN_TITLE_COLS);
    PaneTitle::Centered(markdown::truncate(
        &posts::POSTS[reader.post].title().decrypt(),
        room,
    ))
}

/// What the pane's title row carries: nothing, or a name centered over the
/// pane.
#[derive(Clone, Default, PartialEq, Eq)]
enum PaneTitle {
    #[default]
    None,
    Centered(String),
}

/// The narrowest a title is cut to, however little room the close button leaves.
const MIN_TITLE_COLS: usize = 8;

/// The reader's close button, padded either side so the target is three cells
/// wide rather than one — it is aimed at with a fingertip.
const CLOSE_LABEL: &str = " x ";

/// Everything under the header's rule: the app's body, filling the remaining
/// height, with the reader's title row over it when a post is open. It draws no
/// box of its own — the rule is the one edge the app's chrome has, and the body
/// hangs off it as the page does on the web.
#[component]
fn Pane(props: &mut PaneProps) -> impl Into<AnyElement<'static>> {
    let closable = props.closable;
    // Centering is done against a spacer as wide as everything on the right
    // rather than by centering the whole row: the title is then centered over
    // the pane, not over the space the close button leaves.
    let right = if closable { CLOSE_LABEL.len() } else { 0 };
    let spacer = matches!(props.title, PaneTitle::Centered(_))
        .then_some(right as u16)
        .unwrap_or(0);
    let justify = match props.title {
        PaneTitle::Centered(_) => JustifyContent::Center,
        _ => JustifyContent::Start,
    };
    let title = match &props.title {
        PaneTitle::None => None,
        PaneTitle::Centered(title) => Some(title.clone()),
    };
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            background_color: theme::SURFACE,
            overflow: Overflow::Hidden,
        ) {
            // A row only the reader spends: a tab is named by the header, so
            // the rest of the app gives the row back to what it is showing.
            #(title.map(|title| element! {
                View(flex_direction: FlexDirection::Row, width: 100pct, height: 1) {
                    Column(width: props.column) {
                        View(flex_direction: FlexDirection::Row, width: 100pct) {
                            View(width: spacer, flex_shrink: 0.0_f32)
                            View(flex_grow: 1.0_f32, justify_content: justify, overflow: Overflow::Hidden) {
                                Text(
                                    content: format!(" {title} "),
                                    color: theme::ACCENT_TEXT,
                                    weight: Weight::Bold,
                                    wrap: TextWrap::NoWrap,
                                )
                            }
                            #(closable.then(|| element! { CloseButton }))
                        }
                    }
                }
            }))
            PaneBody {
                #(props.children.drain(..))
            }
        }
    }
}

#[derive(Props, Default)]
struct PaneProps {
    title: PaneTitle,
    /// Whether the title row carries the reader's close button.
    closable: bool,
    /// The app's column, which the title row keeps to so the close button sits
    /// over the right edge of the cards under it rather than out at the edge of
    /// the screen.
    column: u16,
    children: Vec<AnyElement<'static>>,
}

/// The reader's way out for a pointer, at the top right inside the pane's
/// border. It sits in the pane's title row, which nothing clips and which an
/// open dialog paints over — a post cannot be closed out from under one.
#[component]
fn CloseButton(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_region(Some(HitTarget::Close));
    element! {
        Text(
            content: CLOSE_LABEL,
            color: theme::ACCENT_TEXT,
            weight: Weight::Bold,
            wrap: TextWrap::NoWrap,
        )
    }
}

/// The pane's scrolling body, a component of its own so that it can bound what
/// it holds. The pane lays out every card of a tab and shows only the ones that
/// fit, so the card after the last visible one is laid out past the bottom
/// edge, off the screen — close enough to click. Clipping its children's
/// regions to this box keeps those clicks off it.
///
/// Its margin is the app's page margin: the two cells the pane's border and its
/// padding used to hold back between them, which is what [`crate::column_width`]
/// still counts on.
#[component]
fn PaneBody(props: &mut PaneBodyProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_clip();
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            overflow: Overflow::Hidden,
            padding_left: BODY_MARGIN,
            padding_right: BODY_MARGIN,
        ) {
            #(props.children.drain(..))
        }
    }
}

#[derive(Props, Default)]
struct PaneBodyProps {
    children: Vec<AnyElement<'static>>,
}

/// The cells the body keeps clear of the left and right edges of the screen.
const BODY_MARGIN: u16 = 2;

/// The element tree for a given app state; exposed for tests.
pub fn root_element() -> AnyElement<'static> {
    element!(Root).into_any()
}

// --- Header ------------------------------------------------------------------

#[derive(Props, Default)]
struct HeaderProps {
    tab: usize,
    cols: usize,
    rows: usize,
    /// Whether the touch build is drawing this header — see [`chrome`].
    touch: bool,
}

/// The wordmark's letters, three cells wide and three rows tall, drawn out of
/// half blocks. A terminal has one font size — the web host sets it once, for
/// the whole grid (`screen.ts`) — so the only way to set the site's name at
/// anything like a heading's size is to draw the letters out of cells. Three
/// rows is what it takes for a letter to still read as that letter: at two the
/// crossbars have nowhere to go and the name stops being legible. Only the
/// letters [`TITLE`] spells are here; [`banner_rows`] gives up on anything else
/// and the header falls back to the plain one-row title.
const GLYPHS: [(char, [&str; 3]); 11] = [
    (' ', [" ", " ", " "]),
    ('A', ["█▀█", "█▀█", "▀ ▀"]),
    ('D', ["█▀▄", "█ █", "▀▀ "]),
    ('E', ["█▀▀", "█▀ ", "▀▀▀"]),
    ('L', ["█  ", "█  ", "▀▀▀"]),
    ('M', ["█▄█", "█ █", "▀ ▀"]),
    ('O', ["█▀█", "█ █", "▀▀▀"]),
    ('P', ["█▀█", "█▀▀", "▀  "]),
    ('R', ["█▀█", "█▀▄", "▀ ▀"]),
    ('S', ["█▀▀", "▀▀█", "▀▀▀"]),
    ('V', ["█ █", "█ █", " ▀ "]),
];

/// `text` set in [`GLYPHS`], a cell of air between letters. `None` if it spells
/// a letter the wordmark has never needed.
fn banner_rows(text: &str) -> Option<[String; 3]> {
    let mut rows: [String; 3] = Default::default();
    for ch in text.chars() {
        let glyph = GLYPHS.iter().find(|(letter, _)| *letter == ch)?.1;
        for (row, cells) in rows.iter_mut().zip(glyph) {
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(cells);
        }
    }
    Some(rows)
}

/// The size at which the app takes itself to be on a phone. Size is the signal
/// rather than the device: the full-screen app is never told which it is on —
/// the host tells the shell (`ffi::sam_start`), not the app — and a phone held
/// upright is far narrower than this, while on its side it is wide enough but
/// only around twenty rows tall. What it costs a visitor there is the site's
/// name over the tabs — a screenful of rows is worth more to a thumb than a
/// wordmark. [`TouchRoot`], which has no screenful to spend, holds the rows here
/// so that only the width answers.
const PHONE_COLS: usize = 60;
const PHONE_ROWS: usize = 24;

/// Whether the app is being read on something the size of a phone.
fn touch_sized(cols: usize, rows: usize) -> bool {
    cols < PHONE_COLS || rows < PHONE_ROWS
}

/// The least air the header leaves between the name and the tabs when they
/// share a row, and what the fit is tested against. The gap the tabs actually
/// sit at grows past this with the room the name leaves — see [`header_plan`].
const HEADER_GAP_MIN: usize = 4;

/// The air between one tab and the next. It is a gap rather than padding baked
/// into the labels so that the first tab starts at the same column the title
/// does, whether the two share a row or the tabs sit under it.
const TAB_GAP: usize = 3;

/// How the header names the site at the current size.
enum HeaderTitle {
    /// The wordmark, drawn three rows tall out of [`GLYPHS`].
    Banner(Box<[String; 3]>),
    /// The name as plain text, one row, when the wordmark will not fit.
    Plain(&'static str),
    /// Neither, on a terminal too narrow to spare the room for a name.
    None,
}

/// What the header can afford to show at the current size.
struct HeaderPlan {
    title: HeaderTitle,
    labels: Vec<(usize, &'static str)>,
    /// Whether the tabs ride the title's last row rather than taking a row of
    /// their own under it.
    inline_tabs: bool,
    /// The air between the name and the tabs when they do share a row.
    gap: usize,
}

impl HeaderPlan {
    /// Rows the title block takes: the wordmark's three, the plain title's one,
    /// or none at all when the header is too narrow to name the site.
    fn title_rows(&self) -> usize {
        match &self.title {
            HeaderTitle::Banner(_) => 3,
            HeaderTitle::Plain(_) => 1,
            HeaderTitle::None => 0,
        }
    }

    /// Rows the whole header takes: the air above the title block, the block
    /// itself, the tabs' own row if they did not fit beside it, the air under
    /// them, and the rule that closes the bar off from the body. [`header_rows`]
    /// is what the scroll math reads this through, so the two can never
    /// disagree about where the body starts.
    fn rows(&self) -> usize {
        HEADER_MARGIN_ROWS + self.title_rows() + usize::from(!self.inline_tabs) + RULE_ROWS
    }
}

/// The rule under the header, which is one row.
const RULE_ROWS: usize = 1;

/// The row of air the app keeps above the header, which is what sets the bar
/// off from the edge of the glass. A phone used to give it up — a screenful
/// there is barely thirty rows — but it no longer draws the site's name at all
/// ([`header_plan`]), so the row the wordmark cost is spent on air instead: a
/// bar of tabs run flush to the top of the screen reads as the edge of the
/// glass having cut it off rather than as a bar. Nothing matches it at the foot
/// of the screen: the body scrolls, so it runs to the last row the way a page
/// runs to the bottom of a window, and a row of air under a card cut off
/// mid-way would read as the pane ending there.
const HEADER_MARGIN_ROWS: usize = 1;

/// Picks the richest header that fits, the way the site's nav collapses on
/// narrow viewports: the wordmark drawn big, then set as plain text, then
/// dropped; the tabs beside it, then under it; every name, then only the tab
/// in front. Always fits, so no row of it ever wraps.
fn header_plan(tab: usize, cols: usize, rows: usize, touch: bool) -> HeaderPlan {
    let phone = touch_sized(cols, rows);
    // The header keeps to the app's column, inset like everything in it, so
    // that is what it has to fit in.
    let room = (column_cols(cols) as usize).saturating_sub(2 * crate::CARD_PAD_COLS);
    // Help is a table of key bindings, and the touch build's host has no keys
    // to press: it leaves the tab out of the bar and gives the room to the ones
    // that lead somewhere. This is the one thing the header takes from the
    // build rather than from the size — a phone turned on its side is as wide
    // as a laptop and still has no keyboard, and a narrow window on a desktop
    // has one. It is still named while it is the tab in front, so a visitor who
    // arrives at `/help` is never left reading a bar that marks none of its
    // tabs.
    let full: Vec<(usize, &str)> = (0..TAB_NAMES.len())
        .filter(|index| !touch || *index != HELP_TAB || tab == HELP_TAB)
        .map(|index| (index, TAB_NAMES[index]))
        .collect();
    // When the names do not fit, only the tab in front is named: the arrow
    // keys and the number keys still reach the rest.
    let compact: Vec<(usize, &str)> = vec![(tab, TAB_NAMES[tab])];
    let width = |labels: &[(usize, &str)]| {
        labels
            .iter()
            .map(|(_, label)| label.chars().count())
            .sum::<usize>()
            + TAB_GAP * labels.len().saturating_sub(1)
    };
    let labels = if width(&full) <= room { full } else { compact };

    // A phone is not named at all: the wordmark drawn big is rows it has not
    // got, and set as plain text it is a whole row spent repeating what the
    // browser's own tab already says ([`crate::title_for`]) — the tabs alone
    // tell the visitor where they are. Everything wider takes the wordmark if
    // it fits, the plain name if only that does, and neither on a terminal too
    // narrow to spare the room.
    // The wordmark keeps its own rows, so all it has to fit is the width.
    let banner = (!phone)
        .then(|| banner_rows(TITLE.trim()))
        .flatten()
        .filter(|banner| banner[0].chars().count() <= room);
    let title = match banner {
        Some(banner) => HeaderTitle::Banner(Box::new(banner)),
        None if !phone && TITLE.trim().chars().count() <= room => HeaderTitle::Plain(TITLE.trim()),
        None => HeaderTitle::None,
    };
    let title_width = match &title {
        HeaderTitle::Banner(banner) => banner[0].chars().count(),
        HeaderTitle::Plain(title) => title.chars().count(),
        HeaderTitle::None => 0,
    };
    // The tabs follow the name on its own row whenever the two fit side by
    // side with air to spare, as the site's nav sits beside its title; they
    // drop to a row of their own — starting at the same column the name does —
    // only when they would otherwise crowd it.
    let inline_tabs = title_width > 0 && title_width + HEADER_GAP_MIN + width(&labels) <= room;
    // Where the tabs sit on that row: centered in whatever the name leaves, so
    // a wide screen puts real air between the two rather than the couple of
    // cells a narrow one can spare, and neither leaves them adrift at the far
    // edge. The column is capped ([`crate::MAX_COLUMN_COLS`]), so this is too.
    let gap = if inline_tabs {
        (room.saturating_sub(title_width + width(&labels)) / 2).max(HEADER_GAP_MIN)
    } else {
        0
    };
    HeaderPlan {
        title,
        labels,
        inline_tabs,
        gap,
    }
}

/// Rows the header takes at this size, for the scroll math that has to know
/// where the body below it starts. The tab in front is part of that: it is what
/// the header names when it is too narrow to name them all, and a shorter name
/// can be what lets the tabs share the title's row.
pub fn header_rows(tab: usize, cols: u16, rows: u16) -> usize {
    // Only the app that owns a screen has a body to scroll, and that is never
    // the touch build: its page is drawn once, whole, and scrolled by the host.
    header_plan(tab, cols as usize, rows as usize, false).rows()
}

#[component]
fn Header(props: &HeaderProps) -> impl Into<AnyElement<'static>> {
    let tab = props.tab;
    let plan = header_plan(tab, props.cols, props.rows, props.touch);
    let height = plan.rows() as u16;
    let lines: Vec<String> = match &plan.title {
        HeaderTitle::Banner(banner) => banner.to_vec(),
        HeaderTitle::Plain(title) => vec![(*title).to_string()],
        HeaderTitle::None => Vec::new(),
    };
    let tabs = |labels: Vec<(usize, &'static str)>| {
        element! {
            View(flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::NoWrap) {
                #(labels.into_iter().enumerate().map(|(position, (index, label))| {
                    element! {
                        View(flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::NoWrap) {
                            #((position > 0).then(|| element! { View(width: TAB_GAP as u16) }))
                            TabLabel(label: label.to_string(), selected: index == tab, index: index)
                        }
                    }
                }))
            }
        }
    };
    let inline = plan.inline_tabs;
    let gap = plan.gap as u16;
    let labels = plan.labels;
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            // Exactly the rows the plan asked for: the fit cascade guarantees
            // every one of them fits, so nothing here has to wrap or clip.
            height: height,
            flex_wrap: FlexWrap::NoWrap,
        ) {
            // The air above the bar's first row, whatever that row carries.
            View(height: HEADER_MARGIN_ROWS as u16)
            Column(width: column_cols(props.cols)) {
                View(flex_direction: FlexDirection::Row, width: 100pct, align_items: AlignItems::Center) {
                    View(flex_direction: FlexDirection::Column) {
                        #(lines.into_iter().map(|line| element! {
                            Text(content: line, color: theme::ACCENT_TEXT, wrap: TextWrap::NoWrap)
                        }))
                    }
                    // Beside the name and centered on it, as far off it as the
                    // room the name leaves allows.
                    #(inline.then(|| element! { View(width: gap) }))
                    #(inline.then(|| tabs(labels.clone())))
                }
                #((!inline).then(|| tabs(labels.clone())))
            }
            // The bar's own edge, run the whole width of the screen: what the
            // header is set on, and what separates it from the body under it.
            // The wordmark's last row is drawn in half blocks — `▀`, ink in the
            // top half of the cell — so it comes down on half a row of air; the
            // tabs and the plain title are ordinary text and sit on it directly,
            // the way a nav sits on the line under it.
            Text(
                content: "─".repeat(props.cols),
                color: theme::BORDER_SUBTLE,
                wrap: TextWrap::NoWrap,
            )
        }
    }
}

// --- Content panes -------------------------------------------------------------

fn content_element(app: &App) -> AnyElement<'static> {
    element_to_any(content_tree(app))
}

fn content_tree(app: &App) -> AnyElement<'static> {
    match app.tab {
        TIMELINE_TAB => timeline_element(app),
        ABOUT_TAB => about_element(app.scroll(ABOUT_TAB), app.cols),
        HELP_TAB => help_element(app.scroll(HELP_TAB), app.cols),
        BLOG_TAB => match &app.reader {
            Some(reader) => reader_element(app, reader),
            None => blog_element(app),
        },
        _ => element!(View).into_any(),
    }
}

fn timeline_element(app: &App) -> AnyElement<'static> {
    element_to_any(timeline_tree(app))
}

/// The timeline, as a column of cards down the middle of the pane. The column
/// is capped at the same measure the blog's is ([`crate::timeline_column_width`])
/// rather than stretched: a card's title and its category tag are held together
/// at a readable distance, and a wrapped description keeps a measure the eye can
/// track, however wide the terminal is opened.
fn timeline_tree(app: &App) -> impl Into<AnyElement<'static>> {
    let selected = app.selected(TIMELINE_TAB);
    let width = crate::timeline_column_width(app.cols) as u16;
    let inner = crate::content_width(app.cols);
    let cards: Vec<AnyElement<'static>> = data::TIMELINE
        .iter()
        .enumerate()
        .skip(app.scroll(TIMELINE_TAB))
        .map(|(index, event)| card_element(event, index, index == selected, inner, app.cols))
        .collect();
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            align_items: AlignItems::Center,
            overflow: Overflow::Hidden,
        ) {
            View(flex_direction: FlexDirection::Column, width: width, flex_shrink: 0.0_f32) {
                #(cards)
            }
        }
    }
}

/// A card's rail and the air between it and the card's body: as wide as the
/// title row's marker, and what [`crate::content_width`] holds back for.
const RAIL: &str = "│  ";

/// The cells of air inside a card's left and right edges, as cells rather than
/// columns — the other half of what [`crate::content_width`] holds back for.
const CARD_PAD: u16 = crate::CARD_PAD_COLS as u16;

fn card_element(
    event: &data::TimelineEvent,
    index: usize,
    selected: bool,
    inner: usize,
    cols: u16,
) -> AnyElement<'static> {
    element_to_any(card_tree(event, index, selected, inner, cols))
}

/// A card's artwork, indented under the timeline rail like the rest of its
/// body. `None` whenever [`crate::image::rows`] would report zero, so the row
/// count the scroll math assumes and the row count drawn stay the same number.
fn image_row(
    url: Option<EncryptedString>,
    gutter: &'static str,
    selected: bool,
    cols: u16,
) -> Option<AnyElement<'static>> {
    let url = SitePath::new(url?.decrypt());
    let bounds = image::thumbnail_bounds(cols);
    let (_, rows) = image::size(&url, bounds)?;
    Some(gutter_block(
        gutter,
        rows,
        selected,
        element_to_any(element! {
            Image(url: url, bounds: bounds)
        }),
    ))
}

/// One row of a card: the timeline gutter, then a content column wide enough
/// to wrap inside. Keeping the text in its own flex child is what makes a
/// wrapped line indent under itself instead of restarting at the pane edge.
fn gutter_row(
    gutter: &'static str,
    selected: bool,
    body: AnyElement<'static>,
) -> AnyElement<'static> {
    gutter_block(gutter, 1, selected, body)
}

/// A blank row that carries the rail on: the air between a card's sections, and
/// the row of it a card keeps top and bottom. Every row of the column carries
/// the rail, so the spine is one unbroken line from the top of the timeline to
/// the bottom with the cards' markers sitting on it — a gap in it would read as
/// the timeline itself stopping between two events.
fn rail_row(selected: bool) -> AnyElement<'static> {
    gutter_row(
        RAIL,
        selected,
        element_to_any(element! { Text(content: "") }),
    )
}

/// The same, for a body that is `height` rows tall. The gutter is one text per
/// row rather than a single line beside a tall body: a card's rail is drawn,
/// not stretched, so an eight-row thumbnail with one `│` next to it would break
/// the timeline's spine into pieces wherever a card has artwork.
fn gutter_block(
    gutter: &'static str,
    height: u16,
    selected: bool,
    body: AnyElement<'static>,
) -> AnyElement<'static> {
    element_to_any(element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            padding_left: CARD_PAD,
            padding_right: CARD_PAD,
            background_color: if selected { Some(theme::SELECT_BG) } else { None },
        ) {
            View(flex_direction: FlexDirection::Column, flex_shrink: 0.0_f32) {
                #((0..height).map(|_| element! {
                    Text(content: gutter, color: theme::ACCENT_TEXT, wrap: TextWrap::NoWrap)
                }))
            }
            View(flex_direction: FlexDirection::Column, flex_grow: 1.0_f32) {
                #(Some(body))
            }
        }
    })
}

fn card_tree(
    event: &data::TimelineEvent,
    index: usize,
    selected: bool,
    inner: usize,
    cols: u16,
) -> impl Into<AnyElement<'static>> {
    let selected_color = if selected {
        theme::SELECT_FG
    } else {
        theme::TEXT
    };
    let time_color = if selected {
        theme::SELECT_FG
    } else {
        theme::MUTED
    };
    let tag_color = if selected {
        theme::SELECT_FG
    } else {
        event.category.color()
    };
    let marker = if selected { "▸  " } else { "●  " };
    // The title row keeps to one line: the category tag holds the right edge
    // and the title takes whatever is left, as the homepage card header does.
    let tag = format!("[{}]", event.category.label());
    let title_room = inner.saturating_sub(tag.chars().count() + 2);
    let title = event.title.decrypt();
    let title_contents =
        vec![colored(markdown::truncate(&title, title_room), selected_color).weight(Weight::Bold)];
    let detail_color = if selected {
        theme::SELECT_FG
    } else {
        theme::SUBTLE
    };
    // What the card carries under its subheader: the artwork first, as the
    // homepage card leads with its media, then the description and the links.
    // Each is opened by a blank rail row, so a card reads as stacked blocks
    // rather than one paragraph of mixed content — and a card that leaves a
    // section out spends no rows on it, spacer included. [`crate::card_height`]
    // counts the same rows.
    let sections = [
        image_row(event.image, RAIL, selected, cols),
        event.detail.map(|detail| {
            gutter_row(
                RAIL,
                selected,
                element_to_any(element! {
                    Text(content: detail.decrypt(), color: detail_color)
                }),
            )
        }),
        (!event.links.is_empty()).then(|| {
            gutter_row(
                RAIL,
                selected,
                element_to_any(element! {
                    View(flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap) {
                        #(event.links.iter().enumerate().map(|(index, link)| element! {
                            Button(label: crate::link_button_label(index, link), url: link.url.decrypt())
                        }))
                    }
                }),
            )
        }),
    ];
    let body: Vec<AnyElement<'static>> = sections
        .into_iter()
        .flatten()
        .flat_map(|section| [rail_row(selected), section])
        .collect();
    element! {
        HitBlock(index: index) {
            // The row of air above every card, as the blog's index has: what
            // separates one card from the last, and what keeps the first one
            // off the pane's title row. On the selected card it is tinted with
            // the rest, so the highlight opens a row above the title rather
            // than cutting flush against it.
            #(rail_row(selected))
            // Title and category tag, pushed to opposite edges.
            TimelineTitle(
                marker: marker,
                contents: title_contents,
                tag: tag,
                tag_color: Some(tag_color),
                selected: selected,
            )
            // The time, as the card's subheader.
            #(gutter_row(RAIL, selected, element_to_any(element! {
                Text(content: event.time.decrypt(), color: time_color, wrap: TextWrap::NoWrap)
            })))
            #(body)
            // The row of air that closes the card, so the tint under a selected
            // one ends a row past its last line rather than on it.
            #(rail_row(selected))
        }
    }
}

fn line_element(line: &markdown::ContentLine) -> AnyElement<'static> {
    element_to_any(element! {
        Line(contents: line.contents.clone(), url: line.link.clone())
    })
}

// --- Blog ---------------------------------------------------------------------

fn blog_element(app: &App) -> AnyElement<'static> {
    element_to_any(blog_tree(app))
}

/// The blog index, as `/blog` reads on the web: a centered column of cards,
/// each carrying a post's title and its date and nothing else. The column is
/// capped at a comfortable measure ([`crate::blog_column_width`]) rather than
/// stretched, and every card is the same height, so the pane scrolls by the
/// row — `scroll` counts rows, and the card it lands part-way into is pulled
/// up by the rows already above the top edge.
fn blog_tree(app: &App) -> impl Into<AnyElement<'static>> {
    let selected = app.selected(BLOG_TAB);
    let width = crate::blog_column_width(app.cols) as u16;
    let text_width = crate::blog_text_width(app.cols);
    let scroll = app.scroll(BLOG_TAB);
    let cards: Vec<AnyElement<'static>> = posts::POSTS
        .iter()
        .enumerate()
        .skip(scroll / crate::POST_CARD_ROWS)
        .map(|(index, post)| post_card_element(post, index, index == selected, text_width))
        .collect();
    let offset = (scroll % crate::POST_CARD_ROWS) as i32;
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            align_items: AlignItems::Center,
            overflow: Overflow::Hidden,
        ) {
            View(
                flex_direction: FlexDirection::Column,
                width: width,
                flex_shrink: 0.0_f32,
                margin_top: -offset,
            ) {
                #(cards)
            }
        }
    }
}

fn post_card_element(
    post: &posts::Post,
    index: usize,
    selected: bool,
    text_width: usize,
) -> AnyElement<'static> {
    element_to_any(post_card_tree(post, index, selected, text_width))
}

/// A blog card, mirroring the web's: a hairline box carrying the title as its
/// heading and the date under it. A post that lives elsewhere is marked with
/// the same `↗` the web index appends to its title.
fn post_card_tree(
    post: &posts::Post,
    index: usize,
    selected: bool,
    text_width: usize,
) -> impl Into<AnyElement<'static>> {
    let title_color = if selected {
        theme::SELECT_FG
    } else {
        // The title is the card's link on the web, and reads as one here.
        theme::ACCENT_TEXT
    };
    let date_color = if selected {
        theme::SELECT_FG
    } else {
        theme::MUTED
    };
    let title = if post.is_external() {
        format!("{} ↗", post.title())
    } else {
        post.title().to_string()
    };
    element! {
        HitBlock(index: index) {
            // The row of air above every card: the gap between the web's
            // cards, and what keeps the first one off the pane's title row.
            Text(content: "")
            View(
                flex_direction: FlexDirection::Column,
                width: 100pct,
                border_style: BorderStyle::Single,
                border_color: if selected { theme::ACCENT } else { theme::BORDER_SUBTLE },
                background_color: if selected { theme::SELECT_BG } else { theme::SURFACE },
                padding_left: 1,
                padding_right: 1,
            ) {
                Text(
                    content: markdown::truncate(&title, text_width),
                    color: title_color,
                    weight: Weight::Bold,
                    wrap: TextWrap::NoWrap,
                )
                Text(
                    content: post.formatted_date(),
                    color: date_color,
                    wrap: TextWrap::NoWrap,
                )
            }
        }
    }
}

// --- The reader ----------------------------------------------------------------

fn reader_element(app: &App, reader: &Reader) -> AnyElement<'static> {
    element_to_any(reader_tree(app, reader))
}

/// The reader's blocks, wrapped to the blog's column width. Reparsing the body
/// on each call is a few hundred lines a handful of times per keystroke — the
/// same order as the height math over 28 timeline cards — so it is not cached.
pub fn reader_blocks(post: usize, cols: u16) -> Vec<markdown::Block> {
    markdown::post_blocks(
        &posts::POSTS[post].body().decrypt(),
        crate::blog_column_width(cols),
    )
}

/// Rows each block occupies: one for a line, the artwork's height for an image.
pub fn reader_block_heights(post: usize, cols: u16) -> Vec<usize> {
    reader_blocks(post, cols)
        .iter()
        .map(|block| match block {
            markdown::Block::Line(_) => 1,
            markdown::Block::Image { url, .. } => {
                image::rows(Some(url), image::reader_bounds(cols))
            }
        })
        .collect()
}

/// Rows a post occupies in full, which is how far it scrolls.
pub fn reader_row_count(post: usize, cols: u16) -> usize {
    reader_block_heights(post, cols).iter().sum()
}

/// Rows the reader's scrolling body shows: the pane body, less the fixed
/// date/permalink row and the blank row under it.
pub fn reader_viewport(body_rows: usize) -> usize {
    body_rows.saturating_sub(2).max(1)
}

/// The block a scroll offset lands in, and how many of that block's rows are
/// already above the top edge. Every block but artwork is exactly one row, so
/// the second number is zero except part-way down an image.
fn block_at_row(heights: &[usize], scroll: usize) -> (usize, usize) {
    let mut row = 0;
    for (index, height) in heights.iter().enumerate() {
        if row + height > scroll {
            return (index, scroll - row);
        }
        row += height;
    }
    (heights.len(), 0)
}

fn reader_tree(app: &App, reader: &Reader) -> impl Into<AnyElement<'static>> {
    let post = &posts::POSTS[reader.post];
    let width = crate::blog_column_width(app.cols) as u16;
    let blocks = reader_blocks(reader.post, app.cols);
    let heights = reader_block_heights(reader.post, app.cols);
    let (start, offset) = block_at_row(&heights, reader.scroll);
    // The date under the title, as the web's post header carries it. The post
    // is read here, so the row is a subheader and not a way out to a browser.
    let meta = markdown::ContentLine {
        contents: vec![muted(post.formatted_date())],
        link: None,
    };
    let body: Vec<AnyElement<'static>> = blocks
        .iter()
        .skip(start)
        .filter_map(|block| match block {
            markdown::Block::Line(line) => Some(line_element(line)),
            // An image with no recorded size counts as no rows at all;
            // drawing one anyway would put every row below it out of step with
            // the scroll offset.
            markdown::Block::Image { url } => {
                let bounds = image::reader_bounds(app.cols);
                let url = url.clone();
                (image::rows(Some(&url), bounds) > 0).then(|| {
                    element_to_any(element! {
                        Image(url: url, bounds: bounds)
                    })
                })
            }
        })
        .collect();
    // `Overflow::Hidden` plus `PaneBody`'s clip crops the first and last
    // partially visible images at the body's edges, and `Image::draw` measures
    // the visible rectangle from the canvas, so the web overlay crops with it.
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            align_items: AlignItems::Center,
            overflow: Overflow::Hidden,
        ) {
            View(flex_direction: FlexDirection::Column, width: width, flex_shrink: 0.0_f32) {
                Line(contents: meta.contents.clone(), url: meta.link.clone())
                Line(contents: Vec::new(), url: None)
            }
            View(
                flex_direction: FlexDirection::Column,
                width: width,
                flex_grow: 1.0_f32,
                flex_shrink: 0.0_f32,
                overflow: Overflow::Hidden,
            ) {
                // Whatever of the top block is above the edge is pulled out of
                // sight, so the post scrolls a row at a time however tall the
                // block the viewport starts inside.
                View(flex_direction: FlexDirection::Column, width: 100pct, margin_top: -(offset as i32)) {
                    #(body)
                }
            }
        }
    }
}

fn about_lines() -> Vec<markdown::ContentLine> {
    let mut lines = crate::highlight::doc_comment_lines();
    lines.push(markdown::ContentLine {
        contents: Vec::new(),
        link: None,
    });
    for code in crate::highlight::program_lines() {
        lines.push(markdown::ContentLine {
            contents: code,
            link: None,
        });
    }
    lines
}

fn about_element(scroll: usize, cols: u16) -> AnyElement<'static> {
    element_to_any(about_tree(scroll, cols))
}

fn help_element(scroll: usize, cols: u16) -> AnyElement<'static> {
    element_to_any(help_tree(scroll, cols))
}

fn help_tree(scroll: usize, cols: u16) -> impl Into<AnyElement<'static>> {
    let lines = help_lines(cols as usize);
    let rows: Vec<AnyElement<'static>> = lines.iter().skip(scroll).map(line_element).collect();
    listing(rows)
}

fn about_tree(scroll: usize, _cols: u16) -> impl Into<AnyElement<'static>> {
    let lines = about_lines();
    let rows: Vec<AnyElement<'static>> = lines.iter().skip(scroll).map(line_element).collect();
    listing(rows)
}

/// The row of air a listing opens with, under the header's rule: the row the
/// timeline opens with over its first card and the blog over its first, so all
/// four tabs start their content on the same row. [`tab_viewport`] is what the
/// scroll math reads it through.
const LISTING_AIR_ROWS: usize = 1;

/// `rows` as a plain block down the middle of the pane. It hugs the listing on
/// a wide pane and never outgrows a narrow one: `max_width` caps it, and the
/// lines — `MixedText`, wrapping by default — wrap to whatever it then has.
///
/// It is drawn in no box of its own. A frame here can only hug what it holds,
/// so on any screen taller than the listing it would close a few rows above the
/// bottom of the pane and leave the gap under it reading as chrome; the rule
/// under the header is the one edge the app draws, and this hangs off it like
/// everything else.
fn listing(rows: Vec<AnyElement<'static>>) -> AnyElement<'static> {
    element_to_any(element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            align_items: AlignItems::Center,
            padding_top: LISTING_AIR_ROWS as u16,
        ) {
            View(
                flex_direction: FlexDirection::Column,
                max_width: 100pct,
                flex_shrink: 0.0_f32,
            ) {
                #(rows)
            }
        }
    })
}

// --- Help ----------------------------------------------------------------------

/// The Help tab: every key the app listens for, one line each. The key column
/// only earns its keep when the description still fits beside it; below that,
/// the description stacks on its own line.
fn help_lines(cols: usize) -> Vec<markdown::ContentLine> {
    let mut lines = Vec::new();
    {
        // The key column only earns its keep when the description still fits
        // beside it; below that, stack the description on its own line.
        let stacked = cols < 56;
        for (keys, description) in [
            ("←/→ or h/l", "switch between tabs"),
            (
                "1 … 9",
                "open a link of the selected card (timeline) · jump to a tab elsewhere",
            ),
            ("↑/↓ or j/k", "move selection / scroll"),
            ("Enter", "read a post / open a card's link"),
            ("g / G", "jump to top / bottom"),
            ("Esc", "close the reader"),
            ("?", "open this tab"),
            ("q / Ctrl+C", "quit"),
            ("mouse", "click tabs, cards and buttons · wheel scrolls"),
        ] {
            if stacked {
                lines.push(markdown::ContentLine {
                    contents: vec![bold_colored(format!("  {keys}"), theme::ACCENT_TEXT)],
                    link: None,
                });
                lines.push(markdown::ContentLine {
                    contents: vec![colored(format!("    {description}"), theme::SUBTLE)],
                    link: None,
                });
            } else {
                lines.push(markdown::ContentLine {
                    contents: vec![
                        bold_colored(format!("  {keys:<14}"), theme::ACCENT_TEXT),
                        colored(description, theme::SUBTLE),
                    ],
                    link: None,
                });
            }
        }
    }
    lines
}

pub struct AppSnapshot {
    pub tab: usize,
    pub scroll: [usize; TAB_COUNT],
    pub selected: [usize; TAB_COUNT],
    pub reader: Option<Reader>,
    pub visited_count: usize,
}

fn terminal_event_to_crossterm(event: &TerminalEvent) -> Option<crossterm::event::Event> {
    match event {
        // Carry the kind across. iocraft turns on the keyboard enhancement
        // flags, so terminals that support them (Ghostty, Kitty, WezTerm)
        // report a release for every press; rebuilding both as presses made
        // every keystroke act twice.
        TerminalEvent::Key(key) => Some(crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code: key.code,
            modifiers: key.modifiers,
            kind: key.kind,
            state: crossterm::event::KeyEventState::NONE,
        })),
        // iocraft wraps crossterm mouse events losslessly; route them back so
        // handle_event can hit-test clicks against the registered rects.
        TerminalEvent::FullscreenMouse(mouse) => Some(crossterm::event::Event::Mouse(
            crossterm::event::MouseEvent {
                kind: mouse.kind,
                column: mouse.column,
                row: mouse.row,
                modifiers: mouse.modifiers,
            },
        )),
        _ => None,
    }
}

fn element_to_any(element: impl Into<AnyElement<'static>>) -> AnyElement<'static> {
    element.into()
}
