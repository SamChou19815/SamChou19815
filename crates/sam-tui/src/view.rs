//! The iocraft component tree: one component per screen region, mirroring
//! the homepage's design ([`crate::theme`]). Interactive regions register
//! themselves with [`crate::hit`] each frame via `use_component_rect`.

use crate::hit::{self, HitTarget, Rect};
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
pub fn tab_line_count(tab: usize) -> usize {
    match tab {
        TIMELINE_TAB => data::TIMELINE.iter().map(crate::card_height).sum(),
        PROJECTS_TAB => data::PROJECTS.len(),
        ABOUT_TAB => about_lines().len(),
        WORK_TAB => markdown::parse(data::WORK_MARKDOWN).len(),
        EDUCATION_TAB => markdown::parse(data::EDUCATION_MARKDOWN).len(),
        CONTACT_TAB => markdown::parse(data::CONTACT_MARKDOWN).len(),
        _ => 0,
    }
}

/// Total number of lines a modal's scrollable body can show.
pub fn modal_line_count(modal: &Modal) -> usize {
    modal_lines(modal).len()
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
        )
    }
}

#[derive(Props, Default)]
struct TimelineTitleProps {
    index: usize,
    contents: Vec<MixedTextContent>,
    selected: bool,
}

#[component]
fn TimelineTitle(props: &TimelineTitleProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    register_rect(rect, hit::ITEM, props.index, HitTarget::Item(props.index));
    element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            background_color: if props.selected { Some(theme::SELECT_BG) } else { None },
        ) {
            MixedText(contents: props.contents.clone())
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
    tagline: String,
    selected: bool,
}

#[component]
fn ProjectRow(props: &ProjectRowProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let rect = hooks.use_component_rect();
    register_rect(rect, hit::ITEM, props.index, HitTarget::Item(props.index));
    element! {
        View(
            flex_direction: FlexDirection::Row,
            width: 100pct,
            background_color: if props.selected { Some(theme::SELECT_BG) } else { None },
        ) {
            Text(content: if props.selected { "▸ " } else { "  " }, color: theme::SELECT_FG)
            Text(content: format!("{:>2} ", props.index + 1), color: theme::MUTED)
            Text(content: format!("{:<17}", props.id), color: theme::ACCENT_TEXT, weight: Weight::Bold)
            Text(content: props.tagline.clone(), color: theme::SUBTLE)
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

    let counter = pane_counter(&app);
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: terminal_width,
            height: terminal_height,
            background_color: theme::CARD_BG,
        ) {
            Header(tab: app.tab)
            Pane(title: TAB_NAMES[app.tab].to_string(), counter: counter) {
                #(content_element(&app))
            }
            StatusBar(tab: app.tab, modal_open: app.modal.is_some(), visited: app.visited_count())
            #(app.modal.map(|modal| modal_element(&modal)))
        }
    }
}

/// "position/total" for the pane's title row, matching the old border title.
fn pane_counter(app: &App) -> String {
    let total = tab_line_count(app.tab);
    let position = match app.tab {
        TIMELINE_TAB | PROJECTS_TAB => app.selected(app.tab) + 1,
        _ => app.scroll(app.tab) + 1,
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
}

#[component]
fn Header(props: &HeaderProps) -> impl Into<AnyElement<'static>> {
    let tab = props.tab;
    element! {
        View(flex_direction: FlexDirection::Row, width: 100pct) {
            Text(content: TITLE, color: theme::ACCENT_TEXT, weight: Weight::Bold)
            #(TAB_NAMES.iter().enumerate().map(|(index, name)| {
                element! {
                    TabLabel(
                        label: format!(" {} {} ", index + 1, name),
                        selected: index == tab,
                        index: index,
                    )
                }
            }))
            View(flex_grow: 1.0_f32)
            Text(content: TAG, color: theme::BORDER)
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
        ABOUT_TAB => about_element(app.scroll(ABOUT_TAB)),
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
    let cards: Vec<AnyElement<'static>> = data::TIMELINE
        .iter()
        .enumerate()
        .skip(app.scroll(TIMELINE_TAB))
        .map(|(index, event)| card_element(event, index, index == selected))
        .collect();
    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct, flex_grow: 1.0_f32, overflow: Overflow::Hidden) {
            View(flex_direction: FlexDirection::Column, width: 100pct) {
                #(cards)
            }
        }
    }
}

fn card_element(event: &data::TimelineEvent, index: usize, selected: bool) -> AnyElement<'static> {
    element_to_any(card_tree(event, index, selected))
}

fn card_tree(
    event: &data::TimelineEvent,
    index: usize,
    selected: bool,
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
    let marker = if selected { "▸" } else { "●" };
    let title_contents = vec![
        colored(marker, theme::ACCENT_TEXT).weight(Weight::Bold),
        colored(format!(" {}  ", event.title), selected_color),
        colored(event.time, time_color),
        colored(format!("  [{}]", event.category.label()), tag_color),
    ];
    element! {
        View(flex_direction: FlexDirection::Column, width: 100pct) {
            TimelineTitle(index: index, contents: title_contents, selected: selected)
            #(event.detail.map(|detail| element! {
                View(flex_direction: FlexDirection::Row, width: 100pct, background_color: if selected { Some(theme::SELECT_BG) } else { None }) {
                    Text(content: "│ ", color: theme::ACCENT_TEXT)
                    Text(content: detail, color: theme::SUBTLE)
                }
            }))
            #(if event.links.is_empty() { None } else { Some(element! {
                View(flex_direction: FlexDirection::Row, width: 100pct, background_color: if selected { Some(theme::SELECT_BG) } else { None }) {
                    Text(content: "│", color: theme::ACCENT_TEXT)
                    #(event.links.iter().enumerate().map(|(link_index, link)| element! {
                        Button(label: format!(" {} ", link.name.to_uppercase()), url: link.url.to_string(), index: link_index)
                    }))
                }
            }) })
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
            element_to_any(element! {
                ProjectRow(index: index, id: project.id.to_string(), tagline: project.tagline.to_string(), selected: index == selected)
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

fn about_element(scroll: usize) -> AnyElement<'static> {
    element_to_any(about_tree(scroll))
}

fn about_tree(scroll: usize) -> impl Into<AnyElement<'static>> {
    let lines = about_lines();
    let rows: Vec<AnyElement<'static>> = lines
        .iter()
        .enumerate()
        .skip(scroll)
        .map(|(index, line)| line_element(line, index))
        .collect();
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0_f32,
            overflow: Overflow::Hidden,
            background_color: theme::CODE_BG,
            padding: 1,
        ) {
            #(rows)
        }
    }
}

// --- Modal --------------------------------------------------------------------

fn modal_lines(modal: &Modal) -> Vec<markdown::ContentLine> {
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
    for field in fields {
        lines.push(markdown::ContentLine {
            contents: vec![colored(field, theme::TEXT)],
            link: None,
        });
    }
    lines.push(markdown::ContentLine {
        contents: Vec::new(),
        link: None,
    });
    if !matches!(modal, Modal::Help { .. }) {
        lines.push(markdown::ContentLine {
            contents: vec![bold_colored("links (press 1-9)", theme::ACCENT_TEXT)],
            link: None,
        });
        for link in links {
            lines.push(markdown::bullet_link(link.name, link.url));
        }
    } else {
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
            lines.push(markdown::ContentLine {
                contents: vec![
                    bold_colored(format!("  {keys:<14}"), theme::ACCENT_TEXT),
                    colored(description, theme::SUBTLE),
                ],
                link: None,
            });
        }
    }
    lines
}

fn modal_element(modal: &Modal) -> AnyElement<'static> {
    element_to_any(modal_tree(modal))
}

fn modal_tree(modal: &Modal) -> impl Into<AnyElement<'static>> {
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
    let lines = modal_lines(modal);
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
                width: 80pct,
                height: 80pct,
                border_style: BorderStyle::Round,
                border_color: theme::BORDER,
                background_color: theme::CARD_BG,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::Hidden,
            ) {
                Text(content: title, color: theme::SELECT_FG, weight: Weight::Bold)
                View(flex_direction: FlexDirection::Column, width: 100pct, padding: 1, overflow: Overflow::Hidden) {
                    #(rows)
                }
            }
        }
    }
}

// --- Status bar ----------------------------------------------------------------

#[derive(Props, Default)]
struct StatusBarProps {
    tab: usize,
    modal_open: bool,
    visited: usize,
}

#[component]
fn StatusBar(props: &StatusBarProps) -> impl Into<AnyElement<'static>> {
    let (tab, modal_open, visited) = (props.tab, props.modal_open, props.visited);
    let key = |text: String| bold_colored(text, theme::ACCENT_TEXT);
    let mut contents: Vec<MixedTextContent> = Vec::new();
    if modal_open {
        contents.push(key("↑/↓".into()));
        contents.push(muted(" scroll · "));
        contents.push(key("1-9".into()));
        contents.push(muted(" open button · "));
        contents.push(key("Esc".into()));
        contents.push(muted(" close"));
    } else if matches!(tab, TIMELINE_TAB | PROJECTS_TAB) {
        contents.push(key("↑/↓".into()));
        contents.push(muted(" select · "));
        contents.push(key("Enter".into()));
        contents.push(muted(" details · "));
        contents.push(key("←/→".into()));
        contents.push(muted(" tabs · "));
        contents.push(key("?".into()));
        contents.push(muted(" help · "));
        contents.push(key("q".into()));
        contents.push(muted(" quit"));
    } else {
        contents.push(key("↑/↓".into()));
        contents.push(muted(" scroll · "));
        contents.push(key("←/→".into()));
        contents.push(muted(" tabs · "));
        contents.push(key("?".into()));
        contents.push(muted(" help · "));
        contents.push(key("q".into()));
        contents.push(muted(" quit"));
    }
    let complete = visited == 6;
    let progress_color = if complete { theme::STAR } else { theme::MUTED };
    element! {
        View(flex_direction: FlexDirection::Row, width: 100pct, justify_content: JustifyContent::SpaceBetween) {
            MixedText(contents: contents)
            MixedText(contents: vec![
                colored(format!(" {} {}/6 ", if complete { "★" } else { "◆" }, visited), progress_color),
            ])
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
        TerminalEvent::Key(key) => Some(crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(key.code, key.modifiers),
        )),
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
