//! The iocraft component tree: one component per screen region, mirroring
//! the homepage's design ([`crate::theme`]). Interactive regions register
//! themselves with [`crate::hit`] each frame via `use_component_rect`.

use crate::hit::{self, HitTarget, Rect};
use crate::image::{self, Image};
use crate::{
    data, markdown, theme, App, Modal, ABOUT_TAB, CONTACT_TAB, EDUCATION_TAB, PROJECTS_TAB,
    TAB_NAMES, TIMELINE_TAB, WORK_TAB,
};
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
        PROJECTS_TAB => data::PROJECTS
            .iter()
            .map(|project| crate::project_height(project, cols))
            .sum(),
        ABOUT_TAB => about_lines().len(),
        WORK_TAB => markdown::parse(data::WORK_MARKDOWN).len(),
        EDUCATION_TAB => markdown::parse(data::EDUCATION_MARKDOWN).len(),
        CONTACT_TAB => markdown::parse(data::CONTACT_MARKDOWN).len(),
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

fn register_rect(
    rect: Option<crate::view::ComponentRect>,
    kind: u8,
    index: usize,
    target: HitTarget,
) {
    if let Some(rect) = rect {
        hit::register(
            kind,
            index,
            Rect {
                x: rect.left.max(0) as u16,
                y: rect.top.max(0) as u16,
                width: (rect.right - rect.left).max(0) as u16,
                height: (rect.bottom - rect.top).max(0) as u16,
            },
            target,
        );
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
    register_rect(rect, hit::TAB, props.index, HitTarget::Tab(props.index));
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
    register_rect(rect, hit::ITEM, props.index, HitTarget::Item(props.index));
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
    index: usize,
}

#[component]
fn Button(props: &ButtonProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    register_rect(
        rect,
        hit::LINK,
        props.index,
        HitTarget::Link(props.url.clone()),
    );
    element! {
        Text(content: props.label.clone(), color: theme::ACCENT_TEXT, weight: Weight::Bold)
    }
}

#[derive(Props, Default)]
struct ProjectRowProps {
    index: usize,
    id: String,
    selected: bool,
}

#[component]
fn ProjectRow(props: &ProjectRowProps) -> impl Into<AnyElement<'static>> {
    let selected = props.selected;
    element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            background_color: if selected { Some(theme::SELECT_BG) } else { None },
        ) {
            Text(
                content: if selected { "▸  " } else { "   " },
                color: theme::SELECT_FG,
                weight: Weight::Bold,
                wrap: TextWrap::NoWrap,
            )
            Text(content: format!("{} ", props.index + 1), color: theme::MUTED, wrap: TextWrap::NoWrap)
            Text(content: props.id.clone(), color: theme::ACCENT_TEXT, weight: Weight::Bold, wrap: TextWrap::NoWrap)
        }
    }
}

#[derive(Props, Default)]
struct LineProps {
    contents: Vec<MixedTextContent>,
    url: Option<String>,
    index: usize,
}

#[component]
fn Line(props: &LineProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    if let Some(url) = props.url.as_deref() {
        register_rect(
            rect,
            hit::LINK,
            props.index,
            HitTarget::Link(url.to_string()),
        )
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
    // before handing control back. Every image redraws itself into an empty
    // registry, so what the host reads is exactly this frame — and an open
    // dialog raises the top layer, hiding the card artwork it covers.
    image::begin_frame(if app.modal.is_some() {
        image::LAYER_DIALOG
    } else {
        image::LAYER_PANE
    });
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
        PROJECTS_TAB => (app.selected(app.tab) + 1, data::PROJECTS.len()),
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
            View(flex_direction: FlexDirection::Column, width: 100pct, flex_grow: 1.0_f32, overflow: Overflow::Hidden, padding_left: 1, padding_right: 1) {
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
        PROJECTS_TAB => projects_element(app),
        ABOUT_TAB => about_element(app.scroll(ABOUT_TAB), app.cols),
        WORK_TAB => markdown_element(data::WORK_MARKDOWN, app.scroll(WORK_TAB)),
        EDUCATION_TAB => markdown_element(data::EDUCATION_MARKDOWN, app.scroll(EDUCATION_TAB)),
        CONTACT_TAB => markdown_element(data::CONTACT_MARKDOWN, app.scroll(CONTACT_TAB)),
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
    image::size(url, image::THUMBNAIL)?;
    Some(gutter_row(
        gutter,
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
    element_to_any(element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            background_color: if selected { Some(theme::SELECT_BG) } else { None },
        ) {
            Text(content: gutter, color: theme::ACCENT_TEXT, wrap: TextWrap::NoWrap)
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
                    #(event.links.iter().enumerate().map(|(link_index, link)| element! {
                        Button(label: format!("{} ", link.name.to_uppercase()), url: link.url.to_string(), index: link_index)
                    }))
                }
            }))) })
            // Blank separator, so cards read as separate blocks.
            #(gutter_row("   ", false, element_to_any(element! { Text(content: "") })))
        }
    }
}

fn projects_element(app: &App) -> AnyElement<'static> {
    element_to_any(projects_tree(app))
}

fn projects_tree(app: &App) -> impl Into<AnyElement<'static>> {
    let selected = app.selected(PROJECTS_TAB);
    let rows: Vec<AnyElement<'static>> = data::PROJECTS
        .iter()
        .enumerate()
        .skip(app.scroll(PROJECTS_TAB))
        .map(|(index, project)| {
            let is_selected = index == selected;
            let tagline_color = if is_selected {
                theme::SELECT_FG
            } else {
                theme::SUBTLE
            };
            // Name line, then the tagline beneath it in its own column, so it
            // wraps under itself the way the homepage project cards read.
            element_to_any(element! {
                HitBlock(index: index) {
                    ProjectRow(
                        index: index,
                        id: project.id.to_string(),
                        selected: is_selected,
                    )
                    #(image_row(project.image, "     ", is_selected, app.cols))
                    #(gutter_row("     ", is_selected, element_to_any(element! {
                        Text(content: project.tagline, color: tagline_color)
                    })))
                    #(gutter_row("   ", false, element_to_any(element! { Text(content: "") })))
                }
            })
        })
        .collect();
    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct, flex_grow: 1.0_f32, overflow: Overflow::Hidden) {
            #(rows)
        }
    }
}

fn markdown_element(source: &'static str, scroll: usize) -> AnyElement<'static> {
    element_to_any(markdown_tree(source, scroll))
}

fn markdown_tree(source: &'static str, scroll: usize) -> impl Into<AnyElement<'static>> {
    let lines = markdown::parse(source);
    let rows: Vec<AnyElement<'static>> = lines
        .iter()
        .enumerate()
        .skip(scroll)
        .map(|(index, line)| line_element(line, index))
        .collect();
    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct, flex_grow: 1.0_f32, overflow: Overflow::Hidden) {
            #(rows)
        }
    }
}

fn line_element(line: &markdown::ContentLine, index: usize) -> AnyElement<'static> {
    element_to_any(element! {
        Line(contents: line.contents.clone(), url: line.link.clone(), index: index)
    })
}

fn about_lines() -> Vec<markdown::ContentLine> {
    let mut lines = Vec::new();
    for code in crate::highlight::doc_comment_lines() {
        lines.push(markdown::ContentLine {
            contents: code,
            link: None,
        });
    }
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
        .enumerate()
        .skip(scroll)
        .map(|(index, line)| line_element(line, index))
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
        Modal::Project { project, .. } => {
            let project = &data::PROJECTS[*project];
            (vec![project.tagline.to_string()], project.links)
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
            ("1 … 6", "jump to a tab"),
            ("↑/↓ or j/k", "move selection / scroll"),
            ("Enter", "open details (timeline, projects)"),
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
        Modal::Project { project, .. } => data::PROJECTS[*project].image,
        Modal::Help { .. } => None,
    }
}

fn modal_element(modal: &Modal, cols: usize, rows: u16) -> AnyElement<'static> {
    element_to_any(modal_tree(modal, cols, rows))
}

fn modal_tree(modal: &Modal, cols: usize, rows: u16) -> impl Into<AnyElement<'static>> {
    let scroll = match modal {
        Modal::Timeline { scroll, .. } | Modal::Project { scroll, .. } | Modal::Help { scroll } => {
            *scroll
        }
    };
    let title = match modal {
        Modal::Timeline { event, .. } => format!(" {} ", data::TIMELINE[*event].title),
        Modal::Project { project, .. } => format!(" project — {} ", data::PROJECTS[*project].id),
        Modal::Help { .. } => " help ".to_string(),
    };
    // A narrow screen has no room to spare for a margin.
    let modal_width = Percent(if cols < 60 { 96.0 } else { 80.0 });
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
        .enumerate()
        .skip(scroll)
        .map(|(index, line)| line_element(line, index))
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
            View(
                // A narrow screen has no room to spare for a margin.
                width: modal_width,
                height: 80pct,
                border_style: BorderStyle::Round,
                border_color: theme::BORDER,
                background_color: theme::CARD_BG,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::Hidden,
            ) {
                View(width: 100pct, padding_left: 1, padding_right: 1) {
                    Text(content: title, color: theme::SELECT_FG, weight: Weight::Bold)
                }
                View(flex_direction: FlexDirection::Column, width: 100pct, padding: 1, overflow: Overflow::Hidden) {
                    #(hero)
                    #(rows)
                }
            }
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
    } else if matches!(tab, TIMELINE_TAB | PROJECTS_TAB) {
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
    let complete = visited == 6;
    let progress = format!(" {} {}/6 ", if complete { "★" } else { "◆" }, visited);
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
    pub scroll: [usize; 6],
    pub selected: [usize; 6],
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
        // 64 columns is too narrow for six names but wide enough for one.
        let plan = header_plan(TIMELINE_TAB, 64);
        assert!(plan.labels[TIMELINE_TAB].contains("Timeline"));
        assert!(!plan.labels[PROJECTS_TAB].contains("Projects"));
    }

    #[test]
    fn header_keeps_everything_when_it_fits() {
        let plan = header_plan(ABOUT_TAB, 200);
        assert_eq!(plan.title, Some(TITLE));
        assert_eq!(plan.tag, Some(TAG));
        assert!(plan.labels[CONTACT_TAB].contains("Contact"));
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

    #[test]
    fn card_height_matches_the_rows_a_card_renders() {
        // A card with a detail and links, and no artwork: title, time, detail,
        // buttons, blank.
        let plain = data::TIMELINE
            .iter()
            .find(|event| {
                event.detail.is_some() && !event.links.is_empty() && event.image.is_none()
            })
            .expect("a card with a detail and links but no artwork");
        // Wide enough that neither the detail nor the buttons wrap.
        assert_eq!(crate::card_height(plain, 400), 5);

        // The same card with artwork is exactly its thumbnail taller.
        let illustrated = data::TIMELINE
            .iter()
            .find(|event| {
                event.detail.is_some() && !event.links.is_empty() && event.image.is_some()
            })
            .expect("a card with a detail, links and artwork");
        let (_, rows) = image::size(illustrated.image.unwrap(), image::THUMBNAIL)
            .expect("the artwork is baked");
        assert_eq!(crate::card_height(illustrated, 400), 5 + usize::from(rows));
    }

    #[test]
    fn narrow_cards_grow_as_their_detail_wraps() {
        let event = data::TIMELINE
            .iter()
            .max_by_key(|event| event.detail.map_or(0, |detail| detail.chars().count()))
            .expect("the timeline is not empty");
        // Both widths keep their artwork, so this isolates the wrapping.
        assert!(crate::card_height(event, 60) > crate::card_height(event, 400));
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

    /// The same for the projects pane.
    #[test]
    fn every_project_draws_the_height_it_claims() {
        for cols in [60u16, 80, 120] {
            for (index, project) in data::PROJECTS.iter().enumerate() {
                let canvas = pane_width(
                    cols,
                    element_to_any(element! {
                        HitBlock(index: index) {
                            ProjectRow(index: index, id: project.id.to_string(), selected: false)
                            #(image_row(project.image, "     ", false, cols))
                            #(gutter_row("     ", false, element_to_any(element! {
                                Text(content: project.tagline, color: theme::SUBTLE)
                            })))
                            #(gutter_row("   ", false, element_to_any(element! { Text(content: "") })))
                        }
                    }),
                );
                assert_eq!(
                    canvas.height(),
                    crate::project_height(project, cols),
                    "{} at {cols} cols",
                    project.id,
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

    #[test]
    fn a_narrow_terminal_drops_the_artwork() {
        let event = data::TIMELINE
            .iter()
            .find(|event| event.image.is_some())
            .expect("an illustrated card");
        // Same content width either side of the threshold, so any difference is
        // the artwork alone.
        let text_only = crate::card_height(event, 59);
        assert!(crate::card_height(event, 60) > text_only);
        assert_eq!(
            text_only,
            crate::card_height(
                &data::TimelineEvent {
                    image: None,
                    ..*event
                },
                59
            )
        );
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
        for project in data::PROJECTS {
            for cols in [0, 40, 59, 60, 200] {
                let counted = image::rows(project.image, cols, image::THUMBNAIL);
                let drawn = image_row(project.image, "     ", false, cols).is_some();
                assert_eq!(counted > 0, drawn, "{} at {cols} cols", project.id);
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
