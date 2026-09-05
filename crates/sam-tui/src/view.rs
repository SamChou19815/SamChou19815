//! The iocraft component tree: one component per screen region, mirroring
//! the homepage's design ([`crate::theme`]). Interactive regions register
//! themselves with [`crate::hit`] as they paint, one `use_hit_region` call
//! each, and the containers that scroll bound them with `use_hit_clip`.

use crate::crypt::EncryptedString;
use crate::frame::UseFrame;
use crate::hit::{HitTarget, UseHit};
use crate::image::{self, Image};
use crate::{
    data, markdown, posts, theme, App, Modal, Reader, ABOUT_TAB, BLOG_TAB, TAB_COUNT, TAB_NAMES,
    TIMELINE_TAB,
};
use crossterm::style::Color;
use iocraft::components::MixedTextContent;
use iocraft::prelude::*;
use iocraft::AnyElement;

const TITLE: &str = " DEVELOPER SAM ";

/// The one column everything the app draws is laid out down: the wordmark and
/// the tabs, the pane's counter, the cards, and the hints along the bottom. It
/// is the same measure and the same centering the pane's own body uses
/// ([`crate::column_width`]) — the pane's border and padding are inset by
/// exactly what the centering gives back, so every one of those lines up on the
/// same left and right edge whatever the terminal's width.
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
/// pane's body. The About pane insets its program by a row top and bottom
/// ([`about_tree`]), so it fits two fewer lines than the body is tall.
pub fn tab_viewport(tab: usize, body_rows: usize) -> usize {
    match tab {
        ABOUT_TAB => body_rows.saturating_sub(2).max(1),
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
        _ => 0,
    }
}

/// Total number of lines a modal's scrollable body can show.
pub fn modal_line_count(modal: &Modal, cols: u16) -> usize {
    modal_lines(modal, cols as usize).len()
}

/// Rows of text the open dialog shows at once: its body, less the hero
/// artwork and the blank row under it.
pub fn modal_viewport(modal: &Modal, cols: u16, rows: u16) -> usize {
    let hero = hero_bounds(cols as usize, rows)
        .zip(modal_image(modal))
        .map_or(0, |(bounds, url)| {
            match image::rows(Some(&url.decrypt()), bounds) {
                0 => 0,
                drawn => drawn + 1,
            }
        });
    usize::from(dialog_body_rows(rows))
        .saturating_sub(hero)
        .max(1)
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
#[component]
fn Line(props: &LineProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_region(props.url.clone().map(HitTarget::Link));
    element! {
        MixedText(contents: props.contents.clone())
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
    // so what the host reads and what a click hits is exactly this frame — and
    // an open dialog raises the top layer, hiding the card artwork it covers.
    let dialog_open = app.modal.is_some();
    hooks.use_frame(if dialog_open {
        image::LAYER_DIALOG
    } else {
        image::LAYER_PANE
    });
    // The URL bar is one more thing the host mirrors from the frame it is
    // about to see, so it is published here with the rest of them.
    crate::publish_route(app.route());
    let counter = pane_counter(&app);
    let cols = terminal_width as usize;
    let pane_title = pane_title(&app, &counter, cols);
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: terminal_width,
            height: terminal_height,
            background_color: theme::SURFACE,
        ) {
            Header(tab: app.tab, cols: cols, rows: terminal_height as usize)
            Pane(
                title: pane_title,
                counter: counter,
                closable: app.reader.is_some(),
                column: column_cols(cols),
            ) {
                #(content_element(&app))
            }
            #((status_rows(terminal_width, terminal_height) > 0).then(|| element! {
                StatusChrome(
                    tab: app.tab,
                    modal_open: app.modal.is_some(),
                    reading: app.reader.is_some(),
                    cols: cols,
                    rows: terminal_height as usize,
                )
            }))
            #(app.modal.map(|modal| modal_element(&modal, cols, terminal_height)))
        }
    }
}

/// What the pane's title row names, if anything. Only the reader names itself,
/// centered over the pane: a tab is already named — and marked as the one in
/// front — by the header a row above, so a pane that repeats it spends a row of
/// chrome saying what has just been said. The title is cut to what is left
/// beside the counter, so the row stays exactly one line however narrow the
/// terminal or long the post.
fn pane_title(app: &App, counter: &str, cols: usize) -> PaneTitle {
    let Some(reader) = &app.reader else {
        return PaneTitle::None;
    };
    // What the title has to itself: the row less its padding, the space either
    // side of the title, the counter, and the spacer that balances the counter
    // and the close button on the other side.
    let room = cols
        .saturating_sub(2 * counter.chars().count() + 2 * CLOSE_LABEL.len() + 4)
        .max(MIN_TITLE_COLS);
    PaneTitle::Centered(markdown::truncate(
        &posts::POSTS[reader.post].title().decrypt(),
        room,
    ))
}

/// "position/total" for the pane's title row. The list tabs count items, so
/// the counter tracks the selection the reader is actually moving; an open
/// reader counts the blocks it scrolls through instead.
fn pane_counter(app: &App) -> String {
    let (position, total) = if let Some(reader) = &app.reader {
        (reader.scroll + 1, reader_row_count(reader.post, app.cols))
    } else {
        match app.tab {
            TIMELINE_TAB => (app.selected(TIMELINE_TAB) + 1, data::TIMELINE.len()),
            BLOG_TAB => (app.selected(BLOG_TAB) + 1, posts::POSTS.len()),
            tab => (app.scroll(tab) + 1, tab_line_count(tab, app.cols)),
        }
    };
    // No trailing space: the counter ends on the column's right edge, where the
    // cards' own right edge is.
    format!(" {}/{}", position, total.max(1))
}

/// What the pane's title row carries beside its counter: nothing, or a name
/// centered over the pane.
#[derive(Clone, Default, PartialEq, Eq)]
enum PaneTitle {
    #[default]
    None,
    Centered(String),
}

/// The narrowest a title is cut to, however little room the counter leaves.
const MIN_TITLE_COLS: usize = 8;

/// The reader's close button, padded either side so the target is three cells
/// wide rather than one — it is aimed at with a fingertip.
const CLOSE_LABEL: &str = " x ";

/// The homepage's white card: a bordered box filling the remaining height,
/// with a title and scroll counter as its title row.
#[component]
fn Pane(props: &mut PaneProps) -> impl Into<AnyElement<'static>> {
    let counter = props.counter.clone();
    let closable = props.closable;
    // Centering is done against a spacer as wide as everything on the right
    // rather than by centering the whole row: the title is then centered over
    // the pane, not over the space the counter happens to leave.
    let right = counter.chars().count() + if closable { CLOSE_LABEL.len() } else { 0 };
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
            border_style: BorderStyle::Single,
            border_color: theme::BORDER_SUBTLE,
            background_color: theme::SURFACE,
            overflow: Overflow::Hidden,
        ) {
            View(flex_direction: FlexDirection::Row, width: 100pct, height: 1) {
                Column(width: props.column) {
                    View(flex_direction: FlexDirection::Row, width: 100pct) {
                        View(width: spacer, flex_shrink: 0.0_f32)
                        View(flex_grow: 1.0_f32, justify_content: justify, overflow: Overflow::Hidden) {
                            #(title.map(|title| element! {
                                Text(
                                    content: format!(" {title} "),
                                    color: theme::ACCENT_TEXT,
                                    weight: Weight::Bold,
                                    wrap: TextWrap::NoWrap,
                                )
                            }))
                        }
                        Text(content: counter, color: theme::MUTED, wrap: TextWrap::NoWrap)
                        #(closable.then(|| element! { CloseButton }))
                    }
                }
            }
            PaneBody {
                #(props.children.drain(..))
            }
        }
    }
}

#[derive(Props, Default)]
struct PaneProps {
    title: PaneTitle,
    counter: String,
    /// Whether the title row carries the reader's close button.
    closable: bool,
    /// The app's column, which the title row keeps to so that the counter sits
    /// over the right edge of the cards under it rather than out at the border.
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
/// edge, over the status bar — close enough to click. Clipping its children's
/// regions to this box keeps those clicks off it.
#[component]
fn PaneBody(props: &mut PaneBodyProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_clip();
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            overflow: Overflow::Hidden,
            padding_left: 1,
            padding_right: 1,
        ) {
            #(props.children.drain(..))
        }
    }
}

#[derive(Props, Default)]
struct PaneBodyProps {
    children: Vec<AnyElement<'static>>,
}

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

/// The size at which the app takes itself to be on a phone. The host knows it
/// is a touch device but only tells the shell (`ffi::sam_start`), not the app,
/// so size is the only signal the app has: a phone held upright is far narrower
/// than this, and on its side it is wide enough but only around twenty rows
/// tall. What it costs a visitor there is the wordmark drawn big and the
/// keyboard hints along the bottom — neither of which a thumb has any use for.
const PHONE_COLS: usize = 60;
const PHONE_ROWS: usize = 24;

/// Whether the app is being read on something the size of a phone.
fn touch_sized(cols: usize, rows: usize) -> bool {
    cols < PHONE_COLS || rows < PHONE_ROWS
}

/// The air between the title and the tabs when they share a row.
const HEADER_GAP: usize = 2;

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
    labels: Vec<String>,
    /// Whether the tabs ride the title's last row, at the right edge, rather
    /// than taking a row of their own under it.
    inline_tabs: bool,
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

    /// Whether the block under the title needs a row of air under it before the
    /// pane. The wordmark's last row is drawn in half blocks — `▀`, ink in the
    /// top half of the cell — so it already carries half a row of air under its
    /// feet, and the pane's border is drawn half a cell into its own row on top
    /// of that. A whole row on top of those two put twice as much space under
    /// the wordmark as the margin above it. The tabs and the plain title are
    /// ordinary text, ink through the middle of the cell, and do need it.
    fn air_under(&self) -> bool {
        !matches!(self.title, HeaderTitle::Banner(_)) || !self.inline_tabs
    }

    /// Rows the whole header takes: a row of air above the title block, the
    /// block itself, the tabs' own row if they did not fit beside it, and the
    /// row of air that sets the pane off from it. The margin above the wordmark
    /// is the one under the hints at the other end of the screen — the app is
    /// inset from the terminal's edges by a row, top and bottom.
    /// [`header_rows`] is what the scroll math reads this through, so the two
    /// can never disagree about where the pane starts.
    fn rows(&self) -> usize {
        1 + self.title_rows() + usize::from(!self.inline_tabs) + usize::from(self.air_under())
    }
}

/// Picks the richest header that fits, the way the site's nav collapses on
/// narrow viewports: the wordmark drawn big, then set as plain text, then
/// dropped; the tabs beside it, then under it; full tab names, then numbers
/// with only the current tab named, then bare numbers. Always fits, so no row
/// of it ever wraps.
fn header_plan(tab: usize, cols: usize, rows: usize) -> HeaderPlan {
    // The header keeps to the app's column, inset like everything in it, so
    // that is what it has to fit in.
    let room = (column_cols(cols) as usize).saturating_sub(2 * crate::CARD_PAD_COLS);
    let named = |index: usize| format!("{} {}", index + 1, TAB_NAMES[index]);
    let full: Vec<String> = (0..TAB_NAMES.len()).map(named).collect();
    let compact: Vec<String> = (0..TAB_NAMES.len())
        .map(|index| {
            if index == tab {
                named(index)
            } else {
                (index + 1).to_string()
            }
        })
        .collect();
    let numbers: Vec<String> = (1..=TAB_NAMES.len()).map(|n| n.to_string()).collect();
    let width = |labels: &[String]| {
        labels.iter().map(|l| l.chars().count()).sum::<usize>()
            + TAB_GAP * labels.len().saturating_sub(1)
    };
    let labels = if width(&full) <= room {
        full
    } else if width(&compact) <= room {
        compact
    } else {
        numbers
    };

    // The wordmark keeps its own rows, so all it has to fit is the width.
    let banner = (!touch_sized(cols, rows))
        .then(|| banner_rows(TITLE.trim()))
        .flatten()
        .filter(|banner| banner[0].chars().count() <= room);
    let title = match banner {
        Some(banner) => HeaderTitle::Banner(Box::new(banner)),
        None if TITLE.trim().chars().count() <= room => HeaderTitle::Plain(TITLE.trim()),
        None => HeaderTitle::None,
    };
    let title_width = match &title {
        HeaderTitle::Banner(banner) => banner[0].chars().count(),
        HeaderTitle::Plain(title) => title.chars().count(),
        HeaderTitle::None => 0,
    };
    // The tabs follow the name on its own row whenever the two fit side by
    // side, a couple of cells apart, as the site's nav sits beside its title;
    // they drop to a row of their own — starting at the same column the name
    // does — only when they would otherwise crowd it.
    let inline_tabs = title_width > 0 && title_width + HEADER_GAP + width(&labels) <= room;
    HeaderPlan {
        title,
        labels,
        inline_tabs,
    }
}

/// Rows the header takes at this size, for the scroll math that has to know
/// where the pane below it starts.
pub fn header_rows(cols: u16, rows: u16) -> usize {
    header_plan(0, cols as usize, rows as usize).rows()
}

/// Rows the status bar takes: the hints, the row of air under them that matches
/// the one above the wordmark, and — when the header keeps a row of air over
/// the pane — the row that matches it under the pane. The bottom of the screen
/// is the top of it, mirrored, so the app sits evenly between the two edges.
/// The same at every size: a phone shows the hints too.
pub fn status_rows(cols: u16, rows: u16) -> usize {
    2 + usize::from(header_plan(0, cols as usize, rows as usize).air_under())
}

#[component]
fn Header(props: &HeaderProps) -> impl Into<AnyElement<'static>> {
    let tab = props.tab;
    let plan = header_plan(tab, props.cols, props.rows);
    let height = plan.rows() as u16;
    let lines: Vec<String> = match &plan.title {
        HeaderTitle::Banner(banner) => banner.to_vec(),
        HeaderTitle::Plain(title) => vec![(*title).to_string()],
        HeaderTitle::None => Vec::new(),
    };
    let tabs = |labels: Vec<String>| {
        element! {
            View(flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::NoWrap) {
                #(labels.into_iter().enumerate().map(|(index, label)| {
                    element! {
                        View(flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::NoWrap) {
                            #((index > 0).then(|| element! { View(width: TAB_GAP as u16) }))
                            TabLabel(label: label, selected: index == tab, index: index)
                        }
                    }
                }))
            }
        }
    };
    let inline = plan.inline_tabs;
    let air_under = plan.air_under();
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
            // The margin above the wordmark, matched by the one under the hints.
            View(height: 1)
            Column(width: column_cols(props.cols)) {
                View(flex_direction: FlexDirection::Row, width: 100pct, align_items: AlignItems::Center) {
                    View(flex_direction: FlexDirection::Column) {
                        #(lines.into_iter().map(|line| element! {
                            Text(content: line, color: theme::ACCENT_TEXT, wrap: TextWrap::NoWrap)
                        }))
                    }
                    // Beside the name, a couple of cells off it and centered on
                    // it, rather than adrift at the far edge of the screen.
                    #(inline.then(|| element! { View(width: HEADER_GAP as u16) }))
                    #(inline.then(|| tabs(labels.clone())))
                }
                #((!inline).then(|| tabs(labels.clone())))
            }
            // The air between the header and the pane, when what sits above it
            // is not already carrying half a row of its own.
            #(air_under.then(|| element! { View(height: 1) }))
        }
    }
}

#[derive(Props, Default)]
struct StatusChromeProps {
    tab: usize,
    modal_open: bool,
    reading: bool,
    cols: usize,
    rows: usize,
}

/// The hints and the rows of air either side of them, in the app's column so
/// that they start and end on the same edges as the cards over them. Rendering
/// the three together is what keeps [`status_rows`] honest.
#[component]
fn StatusChrome(props: &StatusChromeProps) -> impl Into<AnyElement<'static>> {
    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            // The mirror of the header's own air over the pane.
            #(header_plan(0, props.cols, props.rows).air_under().then(|| element! { View(height: 1) }))
            Column(width: column_cols(props.cols)) {
                StatusBar(
                    tab: props.tab,
                    modal_open: props.modal_open,
                    reading: props.reading,
                    cols: (column_cols(props.cols) as usize).saturating_sub(2 * crate::CARD_PAD_COLS),
                )
            }
            // The margin under the hints, matched by the one above the wordmark.
            View(height: 1)
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
/// `alt` captions the placeholder frame drawn for dimensions-only images.
fn image_row(
    url: Option<EncryptedString>,
    gutter: &'static str,
    selected: bool,
    cols: u16,
    alt: &str,
) -> Option<AnyElement<'static>> {
    let url = url?.decrypt();
    let bounds = image::thumbnail_bounds(cols);
    let (_, rows) = image::size(&url, bounds)?;
    Some(gutter_block(
        gutter,
        rows,
        selected,
        element_to_any(element! {
            Image(url: url, bounds: bounds, alt: alt.to_string())
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
        image_row(event.image, RAIL, selected, cols, &title),
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
                        #(event.links.iter().map(|link| element! {
                            Button(label: format!("{} ", link.name.decrypt().to_uppercase()), url: link.url.decrypt())
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
                image::rows(Some(url.as_str()), image::reader_bounds(cols))
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
            // An image nothing was baked under counts as no rows at all;
            // drawing one anyway would put every row below it out of step with
            // the scroll offset.
            markdown::Block::Image { url, alt } => {
                let bounds = image::reader_bounds(app.cols);
                (image::rows(Some(url.as_str()), bounds) > 0).then(|| {
                    element_to_any(element! {
                        Image(url: url.clone(), bounds: bounds, alt: alt.clone())
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

fn about_tree(scroll: usize, cols: u16) -> impl Into<AnyElement<'static>> {
    let lines = about_lines();
    let rows: Vec<AnyElement<'static>> = lines.iter().skip(scroll).map(line_element).collect();
    // The program keeps to the app's column like every other tab's content, so
    // the code starts under the wordmark rather than out at the pane's border.
    let column = column_cols(cols as usize);
    // The portrait sits beside the program rather than above it, so it stays
    // put while the code scrolls and `tab_line_count` keeps counting lines. It
    // is the column that has to hold the two of them side by side, not the
    // screen, so that is what decides whether there is room for it.
    let portrait = image::enabled(column).then(|| {
        element_to_any(element! {
            View(margin_left: 2, flex_shrink: 0.0_f32) {
                Image(url: image::PORTRAIT.to_string(), bounds: image::AVATAR)
            }
        })
    });
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            align_items: AlignItems::Center,
            overflow: Overflow::Hidden,
        ) {
            View(
                flex_direction: FlexDirection::Row,
                width: column,
                flex_grow: 1.0_f32,
                flex_shrink: 0.0_f32,
                overflow: Overflow::Hidden,
                background_color: theme::SURFACE,
                padding: 1,
            ) {
                View(
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0_f32,
                    overflow: Overflow::Hidden,
                ) {
                    #(rows)
                }
                #(portrait)
            }
        }
    }
}

// --- Modal --------------------------------------------------------------------

fn modal_lines(modal: &Modal, cols: usize) -> Vec<markdown::ContentLine> {
    let mut lines = Vec::new();
    let (fields, links): (Vec<String>, &[data::Link]) = match modal {
        Modal::Timeline { event, .. } => {
            let event = &data::TIMELINE[*event];
            (
                vec![
                    format!("time: {}", event.time),
                    format!("category: {}", event.category.label()),
                ],
                event.links,
            )
        }
        Modal::Help { .. } => (Vec::new(), &[]),
    };
    let had_fields = !fields.is_empty();
    for field in fields {
        lines.push(markdown::ContentLine {
            contents: vec![colored(field, theme::TEXT)],
            link: None,
        });
    }
    // Only separate the fields from what follows when there were any; a short
    // screen cannot spare a blank row that divides nothing.
    if had_fields {
        lines.push(markdown::ContentLine {
            contents: Vec::new(),
            link: None,
        });
    }
    if !matches!(modal, Modal::Help { .. }) {
        lines.push(markdown::ContentLine {
            contents: vec![bold_colored("links (press 1-9)", theme::ACCENT_TEXT)],
            link: None,
        });
        for link in links {
            lines.push(markdown::bullet_link(
                &link.name.decrypt(),
                &link.url.decrypt(),
            ));
        }
    } else {
        // The key column only earns its keep when the description still fits
        // beside it; below that, stack the description on its own line.
        let stacked = cols < 56;
        for (keys, description) in [
            ("←/→ or h/l", "switch between tabs"),
            ("1 … 3", "jump to a tab"),
            ("↑/↓ or j/k", "move selection / scroll"),
            ("Enter", "read a post / open details"),
            ("1 … 9", "open a button of the open dialog"),
            ("g / G", "jump to top / bottom"),
            ("Esc", "close the reader or this dialog"),
            ("?", "toggle this help"),
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

/// Rows inside the dialog's scrolling body: the dialog is 80% of the screen,
/// less its border, title row and padding.
fn dialog_body_rows(rows: u16) -> u16 {
    (u32::from(rows) * 4 / 5).saturating_sub(5) as u16
}

/// The hero's cell box, shrunk to whatever the dialog can spare in both
/// directions. Six rows are held back for the fields and links so the artwork
/// can never crowd them out. `None` when what is left is too short to read as
/// a picture.
fn hero_bounds(cols: usize, rows: u16) -> Option<(u16, u16)> {
    let max_rows = dialog_body_rows(rows).saturating_sub(6).min(image::HERO.1);
    // Below six rows the artwork reads as a smear rather than a picture, so a
    // short dialog spends its rows on the fields instead.
    (max_rows >= 6).then(|| image::fit_width(dialog_content_width(cols), (image::HERO.0, max_rows)))
}

/// The cells inside the dialog's border and its body's padding — all a hero
/// has to fill, the dialog being a fraction of a screen that is itself
/// narrower than [`image::HERO`] on a phone.
fn dialog_content_width(cols: usize) -> usize {
    let percent = if cols < NARROW_DIALOG_COLS { 96 } else { 80 };
    (cols * percent / 100).saturating_sub(4)
}

/// Narrower than this and the dialog has no room to spare for a margin.
const NARROW_DIALOG_COLS: usize = 60;

/// The artwork the open dialog leads with, if any.
fn modal_image(modal: &Modal) -> Option<EncryptedString> {
    match modal {
        Modal::Timeline { event, .. } => data::TIMELINE[*event].image,
        Modal::Help { .. } => None,
    }
}

fn modal_element(modal: &Modal, cols: usize, rows: u16) -> AnyElement<'static> {
    element_to_any(modal_tree(modal, cols, rows))
}

fn modal_tree(modal: &Modal, cols: usize, rows: u16) -> impl Into<AnyElement<'static>> {
    let scroll = match modal {
        Modal::Timeline { scroll, .. } | Modal::Help { scroll } => *scroll,
    };
    let title = match modal {
        Modal::Timeline { event, .. } => format!(" {} ", data::TIMELINE[*event].title),
        Modal::Help { .. } => " help ".to_string(),
    };
    // The hero is fixed above the scrolling body rather than part of it, so
    // `modal_line_count` keeps counting only text and every line stays
    // reachable however tall the artwork is.
    let hero = hero_bounds(cols, rows)
        .zip(modal_image(modal))
        .map(|(bounds, url)| {
            element_to_any(element! {
                View(width: 100pct, justify_content: JustifyContent::Center, padding_bottom: 1) {
                    Image(url: url.decrypt(), bounds: bounds, layer: image::LAYER_DIALOG)
                }
            })
        });
    let lines = modal_lines(modal, cols);
    let rows: Vec<AnyElement<'static>> = lines.iter().skip(scroll).map(line_element).collect();
    element! {
        ModalOverlay {
            Dialog(title: title, narrow: cols < NARROW_DIALOG_COLS) {
                #(hero)
                #(rows)
            }
        }
    }
}

#[derive(Props, Default)]
struct ModalOverlayProps {
    children: Vec<AnyElement<'static>>,
}

/// The whole screen, under the open dialog and centering it. It is the last
/// thing the tree paints, so its region covers every other one — the cards, the
/// tabs, the reader's close button — and a click anywhere it is still showing
/// means the one thing it can mean with a dialog up: dismiss it. The dialog
/// paints inside it and takes back the cells it covers.
#[component]
fn ModalOverlay(props: &mut ModalOverlayProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_region(Some(HitTarget::Dismiss));
    element! {
        View(
            position: Position::Absolute,
            inset: 0,
            width: 100pct,
            height: 100pct,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        ) {
            #(props.children.drain(..))
        }
    }
}

#[derive(Props, Default)]
struct DialogProps {
    title: String,
    narrow: bool,
    children: Vec<AnyElement<'static>>,
}

/// The dialog's own box. It registers itself as a click region that does
/// nothing: a click anywhere on the dialog is not a click on the pane it
/// covers, so it must neither select the card underneath nor — as a click on
/// the [`ModalOverlay`] around it does — dismiss it. Its own contents paint
/// after it and answer first, so its links still open.
#[component]
fn Dialog(props: &mut DialogProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_region(Some(HitTarget::Dialog));
    element! {
        View(
            width: Percent(if props.narrow { 96.0 } else { 80.0 }),
            height: 80pct,
            border_style: BorderStyle::Single,
            border_color: theme::BORDER_SUBTLE,
            background_color: theme::SURFACE,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::Hidden,
        ) {
            View(width: 100pct, padding_left: 1, padding_right: 1) {
                Text(content: props.title.clone(), color: theme::SELECT_FG, weight: Weight::Bold)
            }
            DialogBody {
                #(props.children.drain(..))
            }
        }
    }
}

#[derive(Props, Default)]
struct DialogBodyProps {
    children: Vec<AnyElement<'static>>,
}

/// The dialog's scrolling body, split out for the same reason [`PaneBody`] is:
/// its lines run past the bottom edge, out under the dialog — where a click
/// means "dismiss", not "open the link that is scrolled out of sight".
#[component]
fn DialogBody(props: &mut DialogBodyProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    hooks.use_hit_clip();
    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct, padding: 1, overflow: Overflow::Hidden) {
            #(props.children.drain(..))
        }
    }
}

// --- Status bar ----------------------------------------------------------------

const HINT_SEPARATOR: &str = " · ";

/// Width one hint occupies, including the separator before it.
fn hint_width(keys: &str, description: &str, first: bool) -> usize {
    let separator = if first {
        0
    } else {
        HINT_SEPARATOR.chars().count()
    };
    separator + keys.chars().count() + 1 + description.chars().count()
}

/// Keeps the hints that fit in `budget`, in the order given, so the status bar
/// sheds its least important hints instead of wrapping onto a second row.
fn fitted_hints<'a>(hints: &[(&'a str, &'a str)], budget: usize) -> Vec<(&'a str, &'a str)> {
    let mut kept: Vec<(&str, &str)> = Vec::new();
    let mut used = 0;
    for (keys, description) in hints {
        let width = hint_width(keys, description, kept.is_empty());
        if used + width > budget {
            continue;
        }
        used += width;
        kept.push((keys, description));
    }
    kept
}

#[derive(Props, Default)]
struct StatusBarProps {
    tab: usize,
    modal_open: bool,
    reading: bool,
    cols: usize,
}

#[component]
fn StatusBar(props: &StatusBarProps) -> impl Into<AnyElement<'static>> {
    let (tab, modal_open, reading) = (props.tab, props.modal_open, props.reading);
    // Hints in priority order; the least useful ones drop first when narrow.
    let hints: &[(&str, &str)] = if modal_open {
        &[("Esc", "close"), ("↑/↓", "scroll"), ("1-9", "open button")]
    } else if reading {
        &[("Esc", "back"), ("↑/↓", "scroll"), ("?", "help")]
    } else if tab == TIMELINE_TAB {
        &[
            ("←/→", "tabs"),
            ("q", "quit"),
            ("↑/↓", "select"),
            ("Enter", "details"),
            ("?", "help"),
        ]
    } else if tab == BLOG_TAB {
        &[
            ("←/→", "tabs"),
            ("q", "quit"),
            ("↑/↓", "select"),
            ("Enter", "read"),
            ("?", "help"),
        ]
    } else {
        &[
            ("←/→", "tabs"),
            ("q", "quit"),
            ("↑/↓", "scroll"),
            ("?", "help"),
        ]
    };
    let budget = props.cols;
    let mut contents: Vec<MixedTextContent> = Vec::new();
    for (index, (keys, description)) in fitted_hints(hints, budget).into_iter().enumerate() {
        if index > 0 {
            contents.push(muted(HINT_SEPARATOR));
        }
        contents.push(bold_colored(keys.to_string(), theme::ACCENT_TEXT));
        contents.push(muted(format!(" {description}")));
    }
    element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            height: 1,
            flex_wrap: FlexWrap::NoWrap,
        ) {
            MixedText(contents: contents)
        }
    }
}

/// Clones enough of the app to render a pure frame.
impl App {
    pub fn clone_snapshot(&self) -> AppSnapshot {
        AppSnapshot {
            tab: self.tab,
            scroll: self.scroll,
            selected: self.selected,
            reader: self.reader.clone(),
            modal: self.modal.clone(),
            visited_count: self.visited_count(),
        }
    }
}

pub struct AppSnapshot {
    pub tab: usize,
    pub scroll: [usize; TAB_COUNT],
    pub selected: [usize; TAB_COUNT],
    pub reader: Option<Reader>,
    pub modal: Option<Modal>,
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
