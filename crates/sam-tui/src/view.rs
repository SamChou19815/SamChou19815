//! The iocraft component tree: one component per screen region, mirroring
//! the homepage's design ([`crate::theme`]). Interactive regions register
//! themselves with [`crate::hit`] each frame via `use_component_rect`.

use crate::hit::{self, HitTarget, Rect};
use crate::image::{self, Image};
use crate::{data, markdown, theme, App, Modal, ABOUT_TAB, TAB_COUNT, TAB_NAMES, TIMELINE_TAB};
use crossterm::style::Color;
use iocraft::components::MixedTextContent;
use iocraft::prelude::*;
use iocraft::AnyElement;

/// The rect type reported by `use_component_rect`.
type ComponentRect = taffy::Rect<i32>;

const TITLE: &str = " DEVELOPER SAM ";
const TAG: &str = " rust · wasm ";

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

/// Total number of lines a scrolling tab pane can show.
pub fn tab_line_count(tab: usize, cols: u16) -> usize {
    match tab {
        TIMELINE_TAB => data::TIMELINE
            .iter()
            .map(|event| crate::card_height(event, cols))
            .sum(),
        ABOUT_TAB => about_lines().len(),
        _ => 0,
    }
}

/// Truncates to `width` columns, marking the cut with an ellipsis.
fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let keep = width.saturating_sub(1);
    text.chars()
        .take(keep)
        .chain(std::iter::once('…'))
        .collect()
}

/// Total number of lines a modal's scrollable body can show.
pub fn modal_line_count(modal: &Modal, cols: u16) -> usize {
    modal_lines(modal, cols as usize).len()
}

// --- Leaf components that register hit regions --------------------------------

fn to_rect(rect: ComponentRect) -> Rect {
    Rect {
        x: rect.left.max(0) as u16,
        y: rect.top.max(0) as u16,
        width: (rect.right - rect.left).max(0) as u16,
        height: (rect.bottom - rect.top).max(0) as u16,
    }
}

fn register_rect(rect: Option<ComponentRect>, surface: u8, target: HitTarget) {
    if let Some(rect) = rect {
        hit::register(surface, to_rect(rect), target);
    }
}

/// Records a clipping container's rect as the bounds of its surface. Both
/// containers that do this hide their overflow, and iocraft clips children to
/// the container's box, so the rect is exactly what the surface painted.
fn register_clip(rect: Option<ComponentRect>, surface: u8) {
    if let Some(rect) = rect {
        hit::clip(surface, to_rect(rect));
    }
}

#[derive(Props, Default)]
struct TabLabelProps {
    label: String,
    selected: bool,
    index: usize,
}

#[component]
fn TabLabel(props: &TabLabelProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    register_rect(rect, hit::CHROME, HitTarget::Tab(props.index));
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
/// Link regions inside still win, because [`hit::hit_test`] ranks them higher.
#[derive(Props, Default)]
struct HitBlockProps {
    index: usize,
    children: Vec<AnyElement<'static>>,
}

#[component]
fn HitBlock(props: &mut HitBlockProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    register_rect(rect, hit::PANE, HitTarget::Item(props.index));
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
    let rect = hooks.use_component_rect();
    register_rect(rect, hit::PANE, HitTarget::Link(props.url.clone()));
    element! {
        Text(content: props.label.clone(), color: theme::ACCENT_TEXT, weight: Weight::Bold)
    }
}

#[derive(Props, Default)]
struct LineProps {
    contents: Vec<MixedTextContent>,
    url: Option<String>,
    /// The surface the line is drawn on: the pane, or an open dialog's body.
    surface: u8,
}

#[component]
fn Line(props: &LineProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    if let Some(url) = props.url.as_deref() {
        register_rect(rect, props.surface, HitTarget::Link(url.to_string()))
    }
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

    // The top of the update pass, which iocraft always follows with a draw pass
    // before handing control back. Every image and every click region redraws
    // itself into an empty registry, so what the host reads and what a click
    // hits is exactly this frame — and an open dialog raises the top layer,
    // hiding the card artwork and the card hit regions it covers.
    let dialog_open = app.modal.is_some();
    image::begin_frame(if dialog_open {
        image::LAYER_DIALOG
    } else {
        image::LAYER_PANE
    });
    hit::begin_frame(dialog_open);
    let counter = pane_counter(&app);
    let cols = terminal_width as usize;
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: terminal_width,
            height: terminal_height,
            background_color: theme::CARD_BG,
        ) {
            Header(tab: app.tab, cols: cols)
            Pane(title: TAB_NAMES[app.tab].to_string(), counter: counter) {
                #(content_element(&app))
            }
            StatusBar(
                tab: app.tab,
                modal_open: app.modal.is_some(),
                visited: app.visited_count(),
                cols: cols,
            )
            #(app.modal.map(|modal| modal_element(&modal, cols, terminal_height)))
        }
    }
}

/// "position/total" for the pane's title row. The list tabs count items, so
/// the counter tracks the selection the reader is actually moving.
fn pane_counter(app: &App) -> String {
    let (position, total) = match app.tab {
        TIMELINE_TAB => (app.selected(app.tab) + 1, data::TIMELINE.len()),
        tab => (app.scroll(tab) + 1, tab_line_count(tab, app.cols)),
    };
    format!(" {}/{} ", position, total.max(1))
}

/// The homepage's white card: a bordered box filling the remaining height,
/// with the tab name and scroll counter as its title row.
#[component]
fn Pane(props: &mut PaneProps) -> impl Into<AnyElement<'static>> {
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            border_style: BorderStyle::Round,
            border_color: theme::BORDER,
            background_color: theme::CARD_BG,
            overflow: Overflow::Hidden,
        ) {
            View(flex_direction: FlexDirection::Row, width: 100pct, justify_content: JustifyContent::SpaceBetween, padding_left: 1, padding_right: 1) {
                Text(content: format!(" {} ", props.title), color: theme::ACCENT_TEXT, weight: Weight::Bold)
                Text(content: props.counter.clone(), color: theme::MUTED)
            }
            PaneBody {
                #(props.children.drain(..))
            }
        }
    }
}

#[derive(Props, Default)]
struct PaneProps {
    title: String,
    counter: String,
    children: Vec<AnyElement<'static>>,
}

/// The pane's scrolling body, a component of its own so that it can report
/// where it painted. The pane lays out every card of a tab and shows only the
/// ones that fit, so the card after the last visible one is laid out past the
/// bottom edge, over the status bar — close enough to click. Recording this box
/// as the [`hit::PANE`] surface's bounds keeps those clicks off it.
#[component]
fn PaneBody(props: &mut PaneBodyProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    register_clip(rect, hit::PANE);
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
}

/// What the header can afford to show at the current width.
struct HeaderPlan {
    title: Option<&'static str>,
    labels: Vec<String>,
    tag: Option<&'static str>,
}

/// Picks the richest header that fits, the way the site's nav collapses on
/// narrow viewports: full tab names, then numbers with only the current tab
/// named, then bare numbers. Always fits, so the row never wraps.
fn header_plan(tab: usize, cols: usize) -> HeaderPlan {
    let named = |index: usize| format!(" {} {} ", index + 1, TAB_NAMES[index]);
    let full: Vec<String> = (0..TAB_NAMES.len()).map(named).collect();
    let compact: Vec<String> = (0..TAB_NAMES.len())
        .map(|index| {
            if index == tab {
                named(index)
            } else {
                format!(" {} ", index + 1)
            }
        })
        .collect();
    let numbers: Vec<String> = (1..=TAB_NAMES.len()).map(|n| format!(" {n} ")).collect();

    let fits = |title: Option<&str>, labels: &[String], tag: Option<&str>| {
        let width = title.map_or(0, |t| t.chars().count())
            + labels.iter().map(|l| l.chars().count()).sum::<usize>()
            + tag.map_or(0, |t| t.chars().count());
        width <= cols
    };
    if fits(Some(TITLE), &full, Some(TAG)) {
        HeaderPlan {
            title: Some(TITLE),
            labels: full,
            tag: Some(TAG),
        }
    } else if fits(Some(TITLE), &full, None) {
        HeaderPlan {
            title: Some(TITLE),
            labels: full,
            tag: None,
        }
    } else if fits(Some(TITLE), &compact, None) {
        HeaderPlan {
            title: Some(TITLE),
            labels: compact,
            tag: None,
        }
    } else if fits(None, &compact, None) {
        HeaderPlan {
            title: None,
            labels: compact,
            tag: None,
        }
    } else {
        HeaderPlan {
            title: None,
            labels: numbers,
            tag: None,
        }
    }
}

#[component]
fn Header(props: &HeaderProps) -> impl Into<AnyElement<'static>> {
    let tab = props.tab;
    let plan = header_plan(tab, props.cols);
    element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            // Exactly one row: the fit cascade guarantees the content fits,
            // so no clipping is needed to keep this from wrapping.
            height: 1,
            flex_wrap: FlexWrap::NoWrap,
        ) {
            #(plan.title.map(|title| element! {
                Text(content: title, color: theme::ACCENT_TEXT, weight: Weight::Bold, wrap: TextWrap::NoWrap)
            }))
            #(plan.labels.into_iter().enumerate().map(|(index, label)| {
                element! {
                    TabLabel(label: label, selected: index == tab, index: index)
                }
            }))
            View(flex_grow: 1.0_f32)
            #(plan.tag.map(|tag| element! {
                Text(content: tag, color: theme::BORDER, wrap: TextWrap::NoWrap)
            }))
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
        _ => element!(View).into_any(),
    }
}

fn timeline_element(app: &App) -> AnyElement<'static> {
    element_to_any(timeline_tree(app))
}

fn timeline_tree(app: &App) -> impl Into<AnyElement<'static>> {
    let selected = app.selected(TIMELINE_TAB);
    let inner = crate::content_width(app.cols);
    let cards: Vec<AnyElement<'static>> = data::TIMELINE
        .iter()
        .enumerate()
        .skip(app.scroll(TIMELINE_TAB))
        .map(|(index, event)| card_element(event, index, index == selected, inner, app.cols))
        .collect();
    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct, flex_grow: 1.0_f32, overflow: Overflow::Hidden) {
            View(flex_direction: FlexDirection::Column, width: 100pct) {
                #(cards)
            }
        }
    }
}

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
    url: Option<&'static str>,
    gutter: &'static str,
    selected: bool,
    cols: u16,
) -> Option<AnyElement<'static>> {
    let url = url.filter(|_| image::enabled(cols))?;
    let (_, rows) = image::size(url, image::THUMBNAIL)?;
    Some(gutter_block(
        gutter,
        rows,
        selected,
        element_to_any(element! {
            Image(url: url, bounds: image::THUMBNAIL)
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
    let title_contents =
        vec![colored(truncate(event.title, title_room), selected_color).weight(Weight::Bold)];
    let detail_color = if selected {
        theme::SELECT_FG
    } else {
        theme::SUBTLE
    };
    element! {
        HitBlock(index: index) {
            // Title and category tag, pushed to opposite edges.
            TimelineTitle(
                marker: marker,
                contents: title_contents,
                tag: tag,
                tag_color: Some(tag_color),
                selected: selected,
            )
            // The time, as the card's subheader.
            #(gutter_row("│  ", selected, element_to_any(element! {
                Text(content: event.time, color: time_color, wrap: TextWrap::NoWrap)
            })))
            // The artwork, between the date and the description, as the
            // homepage card leads with its media.
            #(image_row(event.image, "│  ", selected, cols))
            #(event.detail.map(|detail| gutter_row("│  ", selected, element_to_any(element! {
                Text(content: detail, color: detail_color)
            }))))
            #(if event.links.is_empty() { None } else { Some(gutter_row("│  ", selected, element_to_any(element! {
                View(flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap) {
                    #(event.links.iter().map(|link| element! {
                        Button(label: format!("{} ", link.name.to_uppercase()), url: link.url.to_string())
                    }))
                }
            }))) })
            // Blank separator, so cards read as separate blocks.
            #(gutter_row("   ", false, element_to_any(element! { Text(content: "") })))
        }
    }
}

fn line_element(line: &markdown::ContentLine, surface: u8) -> AnyElement<'static> {
    element_to_any(element! {
        Line(contents: line.contents.clone(), url: line.link.clone(), surface: surface)
    })
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
    let rows: Vec<AnyElement<'static>> = lines
        .iter()
        .skip(scroll)
        .map(|line| line_element(line, hit::PANE))
        .collect();
    // The portrait sits beside the program rather than above it, so it stays
    // put while the code scrolls and `tab_line_count` keeps counting lines.
    let portrait = image::enabled(cols).then(|| {
        element_to_any(element! {
            View(margin_left: 2, flex_shrink: 0.0_f32) {
                Image(url: image::PORTRAIT, bounds: image::AVATAR)
            }
        })
    });
    element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            flex_grow: 1.0_f32,
            overflow: Overflow::Hidden,
            background_color: theme::CODE_BG,
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
            lines.push(markdown::bullet_link(link.name, link.url));
        }
    } else {
        // The key column only earns its keep when the description still fits
        // beside it; below that, stack the description on its own line.
        let stacked = cols < 56;
        for (keys, description) in [
            ("←/→ or h/l", "switch between tabs"),
            ("1 … 2", "jump to a tab"),
            ("↑/↓ or j/k", "move selection / scroll"),
            ("Enter", "open details (timeline)"),
            ("1 … 9", "open a button of the open dialog"),
            ("g / G", "jump to top / bottom"),
            ("Esc", "close this dialog"),
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

/// The hero's cell box, shrunk to whatever the dialog can spare. The dialog is
/// 80% of the screen, less its border, title row and padding; six rows are held
/// back for the fields and links so the artwork can never crowd them out.
/// `None` when what is left is too short to read as a picture.
fn hero_bounds(cols: usize, rows: u16) -> Option<(u16, u16)> {
    if !image::enabled(cols as u16) {
        return None;
    }
    let body = (u32::from(rows) * 4 / 5).saturating_sub(5) as u16;
    let max_rows = body.saturating_sub(6).min(image::HERO.1);
    // Below six rows the artwork reads as a smear rather than a picture, so a
    // short dialog spends its rows on the fields instead.
    (max_rows >= 6).then_some((image::HERO.0, max_rows))
}

/// The artwork the open dialog leads with, if any.
fn modal_image(modal: &Modal) -> Option<&'static str> {
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
                    Image(url: url, bounds: bounds, layer: image::LAYER_DIALOG)
                }
            })
        });
    let lines = modal_lines(modal, cols);
    let rows: Vec<AnyElement<'static>> = lines
        .iter()
        .skip(scroll)
        .map(|line| line_element(line, hit::DIALOG_BODY))
        .collect();
    element! {
        View(
            position: Position::Absolute,
            inset: 0,
            width: 100pct,
            height: 100pct,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        ) {
            // A narrow screen has no room to spare for a margin.
            Dialog(title: title, narrow: cols < 60) {
                #(hero)
                #(rows)
            }
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
/// covers, so it must neither select the card underneath nor — as a click
/// outside the dialog does — dismiss it.
#[component]
fn Dialog(props: &mut DialogProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    register_rect(rect, hit::DIALOG, HitTarget::Dialog);
    element! {
        View(
            width: Percent(if props.narrow { 96.0 } else { 80.0 }),
            height: 80pct,
            border_style: BorderStyle::Round,
            border_color: theme::BORDER,
            background_color: theme::CARD_BG,
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
    let rect = hooks.use_component_rect();
    register_clip(rect, hit::DIALOG_BODY);
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
    visited: usize,
    cols: usize,
}

#[component]
fn StatusBar(props: &StatusBarProps) -> impl Into<AnyElement<'static>> {
    let (tab, modal_open, visited) = (props.tab, props.modal_open, props.visited);
    // Hints in priority order; the least useful ones drop first when narrow.
    let hints: &[(&str, &str)] = if modal_open {
        &[("Esc", "close"), ("↑/↓", "scroll"), ("1-9", "open button")]
    } else if tab == TIMELINE_TAB {
        &[
            ("←/→", "tabs"),
            ("q", "quit"),
            ("↑/↓", "select"),
            ("Enter", "details"),
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
    let complete = visited == TAB_COUNT;
    let progress = format!(
        " {} {}/{} ",
        if complete { "★" } else { "◆" },
        visited,
        TAB_COUNT
    );
    let progress_color = if complete { theme::STAR } else { theme::MUTED };

    let budget = props.cols.saturating_sub(progress.chars().count());
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
            justify_content: JustifyContent::SpaceBetween,
            flex_wrap: FlexWrap::NoWrap,
        ) {
            MixedText(contents: contents)
            MixedText(contents: vec![colored(progress, progress_color)])
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
            modal: self.modal.clone(),
            visited_count: self.visited_count(),
        }
    }
}

pub struct AppSnapshot {
    pub tab: usize,
    pub scroll: [usize; TAB_COUNT],
    pub selected: [usize; TAB_COUNT],
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::StreamExt;

    /// Yields once, so that whatever is waiting on this task — the render loop
    /// — gets to run before the next poll.
    async fn yield_once() {
        let mut yielded = false;
        std::future::poll_fn(move |context| {
            if yielded {
                std::task::Poll::Ready(())
            } else {
                yielded = true;
                context.waker().wake_by_ref();
                std::task::Poll::Pending
            }
        })
        .await;
    }

    /// Drives the whole app through a mock terminal and leaves the last frame's
    /// state — its click regions, its image regions — behind for inspection.
    ///
    /// The events are paced apart. A stream that is always ready hands the app
    /// its whole script in a single poll, which the app answers with a single
    /// frame — and `use_component_rect` reports the *previous* frame's layout,
    /// so nothing on screen would ever have registered a region. Yielding
    /// between events lets the loop draw, learn its rects and draw again before
    /// the next one, which is what the frames between two keystrokes do.
    ///
    /// The script has to quit at the end: the loop runs until the app exits.
    fn run(events: Vec<TerminalEvent>) -> Vec<Canvas> {
        crate::PENDING_ACTIONS.with(|pending| pending.borrow_mut().clear());
        let paced = futures::stream::iter(events).then(|event| async move {
            for _ in 0..4 {
                yield_once().await;
            }
            event
        });
        futures::executor::block_on(
            root_element()
                .mock_terminal_render_loop(MockTerminalConfig::with_events(paced))
                .collect::<Vec<_>>(),
        )
    }

    fn press(character: char) -> TerminalEvent {
        TerminalEvent::Key(KeyEvent::new(
            crossterm::event::KeyEventKind::Press,
            crossterm::event::KeyCode::Char(character),
        ))
    }

    /// Ends the run from outside the app. The terminal breaks the render loop
    /// on Ctrl+C without drawing another frame, so the frame the last click was
    /// tested against — an open dialog and all — is the one left to inspect.
    fn interrupt() -> TerminalEvent {
        let mut event = KeyEvent::new(
            crossterm::event::KeyEventKind::Press,
            crossterm::event::KeyCode::Char('c'),
        );
        event.modifiers = crossterm::event::KeyModifiers::CONTROL;
        TerminalEvent::Key(event)
    }

    fn enter() -> TerminalEvent {
        TerminalEvent::Key(KeyEvent::new(
            crossterm::event::KeyEventKind::Press,
            crossterm::event::KeyCode::Enter,
        ))
    }

    fn click_at(col: u16, row: u16) -> TerminalEvent {
        TerminalEvent::FullscreenMouse(FullscreenMouseEvent::new(
            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            col,
            row,
        ))
    }

    /// The URLs the run asked the host to open.
    fn opened_urls() -> Vec<String> {
        crate::PENDING_ACTIONS.with(|pending| {
            pending
                .borrow()
                .iter()
                .map(|crate::Action::OpenUrl(url)| url.clone())
                .collect()
        })
    }

    /// Where the About pane's portrait landed in the last frame.
    fn portrait_region() -> image::Region {
        image::regions()
            .into_iter()
            .find(|region| region.url == image::PORTRAIT)
            .expect("the About pane draws the portrait")
    }

    /// A reader's way to the About tab, through everything that registers a
    /// region: the timeline's cards and buttons, then home.
    fn tour_back_to_about() -> Vec<TerminalEvent> {
        vec![TerminalEvent::Resize(100, 40), press('2'), press('1')]
    }

    /// A tab's regions belong to that tab. They used to outlive it: the cards
    /// and links of the tabs visited before answered clicks on the pane that
    /// replaced them, which is how a click on the About portrait opened a link
    /// belonging to another pane.
    #[test]
    fn a_tab_leaves_no_click_regions_behind() {
        let mut script = tour_back_to_about();
        script.push(press('q'));
        run(script);
        for row in 0..40 {
            for col in 0..100 {
                match hit::hit_test(col, row) {
                    None => {}
                    Some(HitTarget::Tab(_)) => assert_eq!(row, 0, "a tab label off the header row"),
                    // The docblock's `@` tags, and nothing else: no card of the
                    // timeline, no link of a markdown pane.
                    Some(HitTarget::Link(url)) => assert!(
                        data::ABOUT_DOC_LINKS.iter().any(|link| link.url == url),
                        "the About pane answers a click at {col},{row} with {url}, \
                         which belongs to another tab"
                    ),
                    Some(target) => {
                        panic!("the About pane answers a click at {col},{row} with {target:?}")
                    }
                }
            }
        }
    }

    /// The docblock's `@` tags are links, as they are on the homepage.
    #[test]
    fn the_about_docblock_opens_its_links() {
        let blog = data::ABOUT_DOC_LINKS
            .iter()
            .find(|link| link.name == "blog")
            .expect("the docblock links the blog");
        // One run to find the line the blog tag is on, one to click it.
        let mut locate = tour_back_to_about();
        locate.push(press('q'));
        run(locate);
        let &(col, row) =
            cells_answering(|target| matches!(target, HitTarget::Link(url) if url == blog.url))
                .first()
                .expect("the @blog line answers a click");
        let mut script = tour_back_to_about();
        script.push(click_at(col, row));
        script.push(press('q'));
        run(script);
        assert_eq!(
            opened_urls(),
            vec![blog.url.to_string()],
            "clicking the docblock's @blog line at {col},{row}"
        );
    }

    /// The report this all came from: on the About tab, clicking the portrait
    /// opened a link belonging to another pane — its rows sat exactly where
    /// that pane's link lines had been, and the regions had never been retired.
    #[test]
    fn a_click_on_the_about_portrait_opens_nothing() {
        // One run to find the portrait, one to click its every row.
        let mut locate = tour_back_to_about();
        locate.push(press('q'));
        run(locate);
        let portrait = portrait_region();
        let mut script = tour_back_to_about();
        for row in portrait.y..portrait.y + portrait.rows {
            for col in [
                portrait.x,
                portrait.x + portrait.cols / 2,
                portrait.x + portrait.cols - 1,
            ] {
                script.push(click_at(col, row));
            }
        }
        script.push(press('q'));
        run(script);
        assert_eq!(
            opened_urls(),
            Vec::<String>::new(),
            "clicking the portrait opened links"
        );
    }

    /// Every cell of the last frame whose click lands on `target`.
    fn cells_answering(matches: impl Fn(&HitTarget) -> bool) -> Vec<(u16, u16)> {
        (0..40)
            .flat_map(|row| (0..100).map(move |col| (col, row)))
            .filter(|&(col, row)| hit::hit_test(col, row).as_ref().is_some_and(&matches))
            .collect()
    }

    /// The pane goes on laying its cards out behind an open dialog, buttons and
    /// all. A click on the dialog must not reach them.
    #[test]
    fn a_click_on_a_dialog_never_reaches_the_pane_behind_it() {
        let timeline = || vec![TerminalEvent::Resize(100, 40), press('2')];
        // The card buttons, as the pane alone registers them...
        let mut probe = timeline();
        probe.push(interrupt());
        run(probe);
        let buttons = cells_answering(|target| matches!(target, HitTarget::Link(_)));
        assert!(!buttons.is_empty(), "the timeline draws link buttons");
        // ...and the cells the dialog covers once one is open.
        let mut probe = timeline();
        probe.extend([enter(), interrupt()]);
        run(probe);
        let covered = cells_answering(|target| matches!(target, HitTarget::Dialog));
        assert!(!covered.is_empty(), "the dialog registers itself");

        let mut script = timeline();
        script.push(enter());
        script.extend(
            buttons
                .iter()
                .filter(|cell| covered.contains(cell))
                .map(|&(col, row)| click_at(col, row)),
        );
        script.push(interrupt());
        run(script);
        assert_eq!(
            opened_urls(),
            Vec::<String>::new(),
            "a click on the dialog opened a link of the card behind it"
        );
        assert!(
            !cells_answering(|target| matches!(target, HitTarget::Dialog)).is_empty(),
            "the dialog was dismissed by a click that landed on it"
        );
    }

    /// Renders a list row inside the width the pane really gives it: the screen
    /// less the pane's border and padding. The width has to be *definite* —
    /// under a bare `max_width` every `100pct` collapses to its content and
    /// nothing wraps, which is the same reason `Root` anchors the tree to the
    /// terminal size rather than to a percentage.
    fn pane_width(cols: u16, row: AnyElement<'static>) -> Canvas {
        element_to_any(element! {
            View(width: cols - 4, flex_direction: FlexDirection::Column) {
                #(Some(row))
            }
        })
        .render(Some(usize::from(cols)))
    }

    /// Width of a header plan as it will be laid out on one row.
    fn plan_width(plan: &HeaderPlan) -> usize {
        plan.title.map_or(0, |title| title.chars().count())
            + plan
                .labels
                .iter()
                .map(|label| label.chars().count())
                .sum::<usize>()
            + plan.tag.map_or(0, |tag| tag.chars().count())
    }

    #[test]
    fn header_never_outgrows_the_terminal() {
        // Down to a phone-sized terminal, on every tab.
        for cols in 20..=120 {
            for tab in 0..TAB_NAMES.len() {
                let plan = header_plan(tab, cols);
                assert!(
                    plan_width(&plan) <= cols,
                    "header of {} wide overflows {cols} on tab {tab}",
                    plan_width(&plan)
                );
                assert_eq!(
                    plan.labels.len(),
                    TAB_NAMES.len(),
                    "every tab stays clickable"
                );
            }
        }
    }

    #[test]
    fn header_names_the_current_tab_once_names_are_dropped() {
        // 32 columns is too narrow for every name but wide enough for one.
        let plan = header_plan(TIMELINE_TAB, 32);
        assert!(plan.labels[TIMELINE_TAB].contains("Timeline"));
        assert!(!plan.labels[ABOUT_TAB].contains("About"));
    }

    #[test]
    fn status_hints_fit_their_budget() {
        let hints = [
            ("←/→", "tabs"),
            ("q", "quit"),
            ("↑/↓", "select"),
            ("Enter", "details"),
            ("?", "help"),
        ];
        for budget in 0..60 {
            let kept = fitted_hints(&hints, budget);
            let width: usize = kept
                .iter()
                .enumerate()
                .map(|(index, (keys, description))| hint_width(keys, description, index == 0))
                .sum();
            assert!(width <= budget, "{width} hint columns overflow {budget}");
        }
        // The highest-priority hint survives as soon as there is room for it.
        assert_eq!(fitted_hints(&hints, 9), vec![("←/→", "tabs")]);
        assert!(fitted_hints(&hints, 0).is_empty());
    }

    #[test]
    fn truncate_marks_what_it_cuts() {
        assert_eq!(truncate("samlang", 20), "samlang");
        assert_eq!(truncate("samlang", 7), "samlang");
        assert_eq!(truncate("samlang", 4), "sam…");
        // Counts characters, not bytes, so multi-byte titles stay intact.
        assert_eq!(truncate("héllo wörld", 11).chars().count(), 11);
        assert_eq!(truncate("héllo wörld", 6).chars().count(), 6);
    }

    /// The invariant the timeline scroll rests on: `card_height` is what a
    /// card actually draws. It counts items, not lines, so a card that renders
    /// taller than it claims drifts the selection off screen with nothing else
    /// to catch it.
    #[test]
    fn every_card_draws_the_height_it_claims() {
        // From 60 columns up — the widths that carry artwork. Below that
        // `wrapped_rows` divides a character count by the width where the
        // renderer breaks on words, and the two can disagree by a row; that
        // predates artwork and is untouched by it.
        for cols in [60u16, 80, 120] {
            let inner = crate::content_width(cols);
            for (index, event) in data::TIMELINE.iter().enumerate() {
                let canvas = pane_width(cols, card_element(event, index, false, inner, cols));
                assert_eq!(
                    canvas.height(),
                    crate::card_height(event, cols),
                    "{} at {cols} cols draws {} rows but claims {}",
                    event.title,
                    canvas.height(),
                    crate::card_height(event, cols),
                );
            }
        }
    }

    /// An illustrated card really does paint half-blocks where the artwork goes.
    #[test]
    fn an_illustrated_card_paints_its_artwork() {
        let (index, event) = data::TIMELINE
            .iter()
            .enumerate()
            .find(|(_, event)| event.image.is_some())
            .expect("an illustrated card");
        let canvas = pane_width(
            120,
            card_element(event, index, false, crate::content_width(120), 120),
        );
        let painted = (0..canvas.height())
            .filter(|&y| canvas.get_text(0, y, canvas.width(), 1).contains('▀'))
            .count();
        let (_, rows) = image::size(event.image.unwrap(), image::THUMBNAIL).expect("baked");
        assert_eq!(
            painted,
            usize::from(rows),
            "artwork rows are not all painted"
        );
    }

    /// The rail is drawn per row, not stretched, so a body taller than a line
    /// has to repeat it. An eight-row thumbnail beside a single `│` left the
    /// timeline's spine broken on every illustrated card.
    #[test]
    fn the_rail_runs_unbroken_past_the_artwork() {
        let (index, event) = data::TIMELINE
            .iter()
            .enumerate()
            .find(|(_, event)| event.image.is_some())
            .expect("an illustrated card");
        let canvas = pane_width(
            120,
            card_element(event, index, false, crate::content_width(120), 120),
        );
        // Every row between the title, which carries the dot, and the trailing
        // blank separator.
        let broken: Vec<usize> = (1..canvas.height() - 1)
            .filter(|&y| !canvas.get_text(0, y, 1, 1).starts_with('│'))
            .collect();
        assert!(broken.is_empty(), "the rail is missing on rows {broken:?}");
    }

    /// The row count the scroll math assumes and the row the card actually
    /// draws come from one decision. If these ever disagree, the timeline
    /// scrolls out of step with the selection and nothing else catches it.
    #[test]
    fn artwork_rows_and_the_drawn_row_agree() {
        for event in data::TIMELINE {
            for cols in [0, 20, 40, 59, 60, 80, 200] {
                let counted = image::rows(event.image, cols, image::THUMBNAIL);
                let drawn = image_row(event.image, "│  ", false, cols).is_some();
                assert_eq!(
                    counted > 0,
                    drawn,
                    "{} at {cols} cols: counted {counted} rows, drawn = {drawn}",
                    event.title,
                );
            }
        }
    }

    /// The dialog's artwork has to land on the dialog layer, or the overlay
    /// hides it along with the cards it covers.
    #[test]
    fn the_dialog_hero_draws_on_the_dialog_layer() {
        let modal = Modal::Timeline {
            event: 0,
            scroll: 0,
        };
        let render = || {
            // A definite size, as `Root` gives it: the dialog is `100pct` of
            // its parent and collapses to nothing under a bare max width.
            element_to_any(element! {
                View(width: 140u16, height: 45u16) { #(Some(modal_element(&modal, 140, 45))) }
            })
            .render(Some(140))
        };

        image::begin_frame(image::LAYER_DIALOG);
        let _ = render();
        let placed = image::regions();
        assert_eq!(
            placed.len(),
            1,
            "the hero is reported while a dialog is open"
        );
        assert_eq!(
            placed[0].url,
            data::TIMELINE[0].image.expect("card 0 has artwork")
        );

        // Nothing in a dialog belongs to the pane layer.
        image::begin_frame(image::LAYER_PANE);
        let _ = render();
        assert!(image::regions().is_empty());
    }

    #[test]
    fn the_hero_yields_to_a_short_dialog() {
        // Tall enough for artwork and the fields beneath it.
        assert!(hero_bounds(100, 40).is_some());
        // A short screen keeps the links rather than the picture.
        assert_eq!(hero_bounds(100, 20), None);
        // The classic 80x24 still has room for a modest one.
        assert!(hero_bounds(80, 24).is_some_and(|(_, rows)| rows >= 6));
        // A narrow one drops it for the same reason cards do.
        assert_eq!(hero_bounds(40, 40), None);
        // It never asks for more rows than are baked.
        let (_, rows) = hero_bounds(100, 200).expect("a tall screen has room");
        assert!(rows <= image::HERO.1);
    }
}
