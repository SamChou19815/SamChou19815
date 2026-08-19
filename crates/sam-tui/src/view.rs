//! The view layer: one component function per screen region, rendering
//! through ratatui's widget system (`Layout`, `Block`, `Paragraph`, `List`).
//! Interaction regions are collected next to the widgets that own them
//! ([`crate::hit`]); mouse hit-testing itself stays in [`crate::App`].

use crate::highlight;
use crate::hit::HitAreas;
use crate::markdown::{self, accent, dim, key_style, ContentLine};
use crate::theme;
use crate::{
    data, App, LinkRegion, Modal, ABOUT_TAB, CONTACT_TAB, EDUCATION_TAB, MIN_COLS, MIN_ROWS,
    PROJECTS_TAB, TAB_NAMES, TIMELINE_TAB, WORK_TAB,
};
use ratatui_core::layout::{Alignment, Constraint, Layout, Rect};
use ratatui_core::style::{Modifier, Style};
use ratatui_core::terminal::Frame;
use ratatui_core::text::{Line, Span};
use ratatui_widgets::block::{Block, Padding};
use ratatui_widgets::clear::Clear;
use ratatui_widgets::list::{List, ListItem, ListState};
use ratatui_widgets::paragraph::Paragraph;

fn text_bold() -> Style {
    Style::new().fg(theme::TEXT).add_modifier(Modifier::BOLD)
}

fn subtle() -> Style {
    Style::new().fg(theme::SUBTLE)
}

fn border_style() -> Style {
    Style::new().fg(theme::BORDER)
}

fn select_style() -> Style {
    // The homepage's own hover treatment (`bg-blue-500 bg-opacity-10`),
    // scaled up to a selection: a soft blue tint with deep blue text.
    Style::new().bg(theme::SELECT_BG).fg(theme::SELECT_FG)
}

/// Total number of lines a scrolling tab pane can show.
pub fn tab_line_count(tab: usize) -> usize {
    match tab {
        TIMELINE_TAB => data::TIMELINE.iter().map(card_height).sum(),
        PROJECTS_TAB => data::PROJECTS.len(),
        ABOUT_TAB => about_doc_len(),
        WORK_TAB => markdown::parse(data::WORK_MARKDOWN).len(),
        EDUCATION_TAB => markdown::parse(data::EDUCATION_MARKDOWN).len(),
        CONTACT_TAB => markdown::parse(data::CONTACT_MARKDOWN).len(),
        _ => 0,
    }
}

/// Total number of lines a modal's scrollable body can show.
pub fn modal_line_count(modal: &Modal) -> usize {
    modal_body(modal).1.len()
}

pub fn draw(app: &mut App, frame: &mut Frame) {
    app.hit = HitAreas::default();
    let area = frame.area();
    if area.width < MIN_COLS || area.height < MIN_ROWS {
        let message = if area.width >= 34 {
            " please resize to at least 40×12 "
        } else {
            " 40×12+ "
        };
        frame.render_widget(Paragraph::new(message).alignment(Alignment::Center), area);
        return;
    }
    let [header, content, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    let modal_open = app.modal.is_some();
    draw_header(app, frame, header);
    draw_content(app, frame, content, !modal_open);
    if modal_open {
        draw_modal(app, frame, content);
    }
    draw_status(app, frame, status);
}

// --- Header: title, tabs, tag -------------------------------------------------

const TITLE: &str = " DEVELOPER SAM ";
const TAG: &str = " rust · wasm ";

fn draw_header(app: &mut App, frame: &mut Frame, area: Rect) {
    let mut constraints = Vec::with_capacity(TAB_NAMES.len() + 3);
    let mut labels: Vec<(usize, String, bool)> = Vec::with_capacity(TAB_NAMES.len());
    constraints.push(Constraint::Length(TITLE.len() as u16));
    for (index, name) in TAB_NAMES.iter().enumerate() {
        let label = format!(" {} {} ", index + 1, name);
        constraints.push(Constraint::Length(label.len() as u16));
        labels.push((index, label, index == app.tab));
    }
    constraints.push(Constraint::Min(0));
    constraints.push(Constraint::Length(TAG.len() as u16));
    let areas = Layout::horizontal(&constraints).split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(TITLE, accent().add_modifier(Modifier::BOLD))),
        areas[0],
    );
    for (position, (index, label, selected)) in labels.iter().enumerate() {
        let style = if *selected {
            select_style().add_modifier(Modifier::BOLD)
        } else {
            subtle()
        };
        let separator = if index + 1 < TAB_NAMES.len() {
            "│"
        } else {
            ""
        };
        let line = Line::from(vec![
            Span::styled(label.clone(), style),
            Span::styled(separator, dim()),
        ]);
        frame.render_widget(Paragraph::new(line), areas[position + 1]);
        app.hit.tabs.push(*areas.get(position + 1).unwrap_or(&area));
    }
    frame.render_widget(
        Paragraph::new(Span::styled(TAG, dim())),
        *areas.last().unwrap(),
    );
}

// --- Content pane: one white card per tab ---------------------------------------

fn draw_content(app: &mut App, frame: &mut Frame, area: Rect, interactive: bool) {
    let total = tab_line_count(app.tab);
    let position = match app.tab {
        TIMELINE_TAB | PROJECTS_TAB => app.selected[app.tab] + 1,
        _ => app.scroll[app.tab] + 1,
    };
    let counter = Line::from(Span::styled(
        format!(" {}/{} ", position, total.max(1)),
        dim(),
    ))
    .right_aligned();
    let block = Block::bordered()
        .border_style(border_style())
        .style(Style::new().bg(theme::CARD_BG))
        .title(Span::styled(
            format!(" {} ", TAB_NAMES[app.tab]),
            accent().add_modifier(Modifier::BOLD),
        ))
        .title(counter);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    match app.tab {
        TIMELINE_TAB => draw_timeline(app, frame, inner, interactive),
        PROJECTS_TAB => draw_projects(app, frame, inner, interactive),
        ABOUT_TAB => draw_about(app, frame, inner, interactive),
        WORK_TAB | EDUCATION_TAB | CONTACT_TAB => draw_markdown(app, frame, inner, interactive),
        _ => {}
    }
}

/// Timeline rows per card: title, optional detail, optional buttons, gap.
fn card_height(event: &data::TimelineEvent) -> usize {
    1 + usize::from(event.detail.is_some()) + usize::from(!event.links.is_empty()) + 1
}

fn card_item(event: &data::TimelineEvent, selected: bool) -> ListItem<'static> {
    let mut lines = vec![Line::from(vec![
        Span::styled(
            if selected { "▸" } else { "●" },
            accent().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(event.title, text_bold()),
        Span::raw("  "),
        Span::styled(event.time, dim()),
        Span::raw("  "),
        Span::styled(
            format!("[{}]", event.category.label()),
            Style::new().fg(event.category.color()),
        ),
    ])];
    if let Some(detail) = event.detail {
        lines.push(Line::from(vec![
            Span::styled("│", accent()),
            Span::raw(" "),
            Span::styled(detail, subtle()),
        ]));
    }
    if !event.links.is_empty() {
        let mut spans = vec![Span::styled("│", accent()), Span::raw(" ")];
        for (index, link) in event.links.iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(
                format!(" {} ", link.name.to_uppercase()),
                markdown::link_style(),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::default());
    ListItem::new(lines)
}

fn draw_timeline(app: &mut App, frame: &mut Frame, inner: Rect, interactive: bool) {
    let selected = app.selected[TIMELINE_TAB].min(data::TIMELINE.len() - 1);
    let items: Vec<ListItem<'static>> = data::TIMELINE
        .iter()
        .enumerate()
        .map(|(index, event)| card_item(event, index == selected))
        .collect();
    let list = List::new(items).highlight_style(select_style().add_modifier(Modifier::BOLD));
    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(list, inner, &mut state);
    if interactive {
        collect_timeline_hits(&mut app.hit, inner, state.offset());
    }
}

fn collect_timeline_hits(hits: &mut HitAreas, inner: Rect, offset: usize) {
    let mut y = inner.y;
    for (index, event) in data::TIMELINE.iter().enumerate().skip(offset) {
        if y >= inner.bottom() {
            break;
        }
        let height = card_height(event) as u16;
        // The trailing gap row is not part of the card's click target.
        let rect = Rect::new(
            inner.x,
            y,
            inner.width,
            height.saturating_sub(1).min(inner.bottom() - y),
        );
        hits.rows.push((rect, index));
        if !event.links.is_empty() {
            let buttons_y = y + 1 + u16::from(event.detail.is_some());
            if buttons_y < inner.bottom() {
                let mut x = inner.x + 2;
                for link in event.links {
                    let width = (link.name.chars().count() + 2) as u16;
                    if x + width > inner.right() {
                        break;
                    }
                    hits.links.push(LinkRegion {
                        rect: Rect::new(x, buttons_y, width, 1),
                        url: link.url.to_string(),
                    });
                    x += width + 2;
                }
            }
        }
        y = y.saturating_add(height);
    }
}

fn draw_projects(app: &mut App, frame: &mut Frame, inner: Rect, interactive: bool) {
    let selected = app.selected[PROJECTS_TAB].min(data::PROJECTS.len() - 1);
    let items: Vec<ListItem<'static>> = data::PROJECTS
        .iter()
        .enumerate()
        .map(|(index, project)| {
            let line = Line::from(vec![
                Span::raw(if index == selected { "▸ " } else { "  " }),
                Span::styled(format!("{:>2} ", index + 1), dim()),
                Span::styled(
                    format!("{:<17}", project.id),
                    Style::new()
                        .fg(theme::ACCENT_TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(project.tagline, subtle()),
            ]);
            ListItem::new(line)
        })
        .collect();
    let list = List::new(items).highlight_style(select_style().add_modifier(Modifier::BOLD));
    let mut state = ListState::default();
    state.select(Some(selected));
    frame.render_stateful_widget(list, inner, &mut state);
    if interactive {
        for (y, index) in (inner.y..inner.bottom()).zip(state.offset()..data::PROJECTS.len()) {
            app.hit
                .rows
                .push((Rect::new(inner.x, y, inner.width, 1), index));
        }
    }
}

// --- Markdown panes (work, education, contact) -----------------------------------

fn draw_markdown(app: &mut App, frame: &mut Frame, inner: Rect, interactive: bool) {
    let source = match app.tab {
        WORK_TAB => data::WORK_MARKDOWN,
        EDUCATION_TAB => data::EDUCATION_MARKDOWN,
        _ => data::CONTACT_MARKDOWN,
    };
    let lines = markdown::parse(source);
    let scroll = app.scroll[app.tab].min(lines.len().saturating_sub(1));
    app.scroll[app.tab] = scroll;
    let rendered: Vec<Line<'static>> = lines
        .iter()
        .map(|line| Line::from(line.spans.clone()))
        .collect();
    frame.render_widget(Paragraph::new(rendered).scroll((scroll as u16, 0)), inner);
    if interactive {
        collect_paragraph_links(&lines, scroll, inner, &mut app.hit);
    }
}

/// Records link regions for the visible rows of a scrolled paragraph.
fn collect_paragraph_links(lines: &[ContentLine], scroll: usize, area: Rect, hits: &mut HitAreas) {
    for (index, y) in (scroll..lines.len()).zip(area.y..area.bottom()) {
        if let Some(url) = &lines[index].link {
            let width = spans_width(&lines[index].spans).min(area.width as usize) as u16;
            hits.links.push(LinkRegion {
                rect: Rect::new(area.x, y, width, 1),
                url: url.clone(),
            });
        }
    }
}

fn spans_width(spans: &[Span<'static>]) -> usize {
    spans.iter().map(|span| span.content.chars().count()).sum()
}

// --- About: the code block plus links ---------------------------------------------

fn about_code_lines() -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = highlight::doc_comment_lines()
        .into_iter()
        .map(Line::from)
        .collect();
    // A blank line separates the doc block from the program, as on the site.
    lines.push(Line::default());
    lines.extend(highlight::program_lines().into_iter().map(Line::from));
    lines
}

fn about_link_lines() -> Vec<ContentLine> {
    let mut lines = markdown::parse("### links");
    for link in data::ABOUT_DOC_LINKS {
        lines.push(markdown::bullet_link(link.name, link.url));
    }
    lines
}

fn about_doc_len() -> usize {
    // code lines + one padding row top and bottom, plus the links section
    about_code_lines().len() + 2 + about_link_lines().len()
}

fn draw_about(app: &mut App, frame: &mut Frame, inner: Rect, interactive: bool) {
    let code_lines = about_code_lines();
    let link_lines = about_link_lines();
    let code_len = code_lines.len() + 2;
    let scroll = app.scroll[ABOUT_TAB].min(about_doc_len().saturating_sub(1));
    app.scroll[ABOUT_TAB] = scroll;
    let [code_area, rest_area] =
        Layout::vertical([Constraint::Length(code_len as u16), Constraint::Min(0)]).areas(inner);
    // The homepage code block: its own surface with one cell of padding.
    let code_block = Block::default()
        .style(Style::new().bg(theme::CODE_BG))
        .padding(Padding::uniform(1));
    frame.render_widget(
        Paragraph::new(code_lines)
            .block(code_block)
            .scroll((scroll.min(code_len) as u16, 0)),
        code_area,
    );
    let rest_scroll = scroll.saturating_sub(code_len);
    let rendered: Vec<Line<'static>> = link_lines
        .iter()
        .map(|line| Line::from(line.spans.clone()))
        .collect();
    frame.render_widget(
        Paragraph::new(rendered).scroll((rest_scroll as u16, 0)),
        rest_area,
    );
    if interactive {
        collect_paragraph_links(&link_lines, rest_scroll, rest_area, &mut app.hit);
    }
}

// --- Modals ----------------------------------------------------------------------

/// The classic centered-rect helper from the ratatui popup guide.
fn centered_rect(area: Rect) -> Rect {
    let width = (area.width * 4 / 5).clamp(40, area.width.saturating_sub(2));
    let height = (area.height * 4 / 5).clamp(10, area.height.saturating_sub(2));
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn field(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), accent()),
        Span::raw(value.to_string()),
    ])
}

/// Word-wraps `text` to `width` columns, one span per line.
fn wrapped(text: &str, width: usize, style: Style) -> Vec<Span<'static>> {
    if width < 8 {
        return vec![Span::styled(text.to_string(), style)];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let candidate_len = current.len() + word.len() + usize::from(!current.is_empty());
        if !current.is_empty() && candidate_len > width {
            lines.push(Span::styled(current.trim_end().to_string(), style));
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        while current.len() > width {
            lines.push(Span::styled(current[..width].to_string(), style));
            current = current[width..].trim_start().to_string();
        }
    }
    lines.push(Span::styled(current, style));
    lines
}

fn modal_body(modal: &Modal) -> (String, Vec<Line<'static>>, Vec<Option<String>>) {
    match modal {
        Modal::Timeline { event, .. } => {
            let event = &data::TIMELINE[*event];
            let mut lines = vec![
                field("time", event.time),
                field("category", event.category.label()),
            ];
            let mut links = vec![None, None];
            if let Some(detail) = event.detail {
                lines.push(Line::default());
                links.push(None);
                for span in wrapped(detail, 60, subtle()) {
                    lines.push(Line::from(vec![span]));
                    links.push(None);
                }
            }
            if !event.links.is_empty() {
                lines.push(Line::default());
                links.push(None);
                lines.push(Line::from(Span::styled(
                    "links",
                    key_style().add_modifier(Modifier::BOLD),
                )));
                links.push(None);
                for link in event.links {
                    let line = markdown::bullet_link(link.name, link.url);
                    lines.push(Line::from(line.spans.clone()));
                    links.push(line.link.clone());
                }
            }
            (event.title.to_string(), lines, links)
        }
        Modal::Project { project, .. } => {
            let project = &data::PROJECTS[*project];
            let mut lines = Vec::new();
            let mut links = Vec::new();
            for span in wrapped(project.tagline, 60, subtle()) {
                lines.push(Line::from(vec![span]));
                links.push(None);
            }
            lines.push(Line::default());
            links.push(None);
            lines.push(Line::from(Span::styled(
                "links",
                key_style().add_modifier(Modifier::BOLD),
            )));
            links.push(None);
            for link in project.links {
                let line = markdown::bullet_link(link.name, link.url);
                lines.push(Line::from(line.spans.clone()));
                links.push(line.link.clone());
            }
            (format!("project — {}", project.id), lines, links)
        }
        Modal::Help { .. } => {
            let bindings = [
                ("←/→ or h/l", "switch between tabs"),
                ("1 … 6", "jump to a tab"),
                ("↑/↓ or j/k", "move selection / scroll"),
                ("Enter", "open details (timeline, projects)"),
                ("g / G", "jump to top / bottom"),
                ("Esc", "close this dialog"),
                ("?", "toggle this help"),
                ("q / Ctrl+C", "quit"),
                ("mouse", "click tabs, cards and buttons · wheel scrolls"),
            ];
            let lines = bindings
                .into_iter()
                .map(|(keys, description)| {
                    Line::from(vec![
                        Span::styled(
                            format!("  {keys:<14}"),
                            key_style().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(description, subtle()),
                    ])
                })
                .collect();
            ("help".to_string(), lines, vec![None; bindings_len()])
        }
    }
}

fn bindings_len() -> usize {
    9
}

fn draw_modal(app: &mut App, frame: &mut Frame, content: Rect) {
    let modal = app
        .modal
        .clone()
        .expect("draw_modal called without a modal");
    let rect = centered_rect(content);
    app.hit.modal = Some(rect);
    frame.render_widget(Clear, rect);
    let (title, body, links) = modal_body(&modal);
    let scroll = *match &modal {
        Modal::Timeline { scroll, .. } | Modal::Project { scroll, .. } | Modal::Help { scroll } => {
            scroll
        }
    };
    let block = Block::bordered()
        .border_style(border_style())
        .style(Style::new().bg(theme::CARD_BG))
        .title(Span::styled(
            format!(" {title} "),
            select_style().add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(rect);
    frame.render_widget(
        Paragraph::new(body).block(block).scroll((scroll as u16, 0)),
        rect,
    );
    for (index, y) in (scroll..links.len()).zip(inner.y..inner.bottom()) {
        if let Some(url) = &links[index] {
            app.hit.links.push(LinkRegion {
                rect: Rect::new(inner.x, y, inner.width, 1),
                url: url.clone(),
            });
        }
    }
}

// --- Status bar -------------------------------------------------------------------

fn draw_status(app: &mut App, frame: &mut Frame, area: Rect) {
    let key = |text: &str| Span::styled(text.to_string(), key_style().add_modifier(Modifier::BOLD));
    let spans: Vec<Span<'static>> = if app.modal.is_some() {
        vec![
            key("↑/↓"),
            Span::styled(" scroll · ", dim()),
            key("Esc"),
            Span::styled(" close · ", dim()),
            key("click"),
            Span::styled(" a button to open", dim()),
        ]
    } else if matches!(app.tab, TIMELINE_TAB | PROJECTS_TAB) {
        vec![
            key("↑/↓"),
            Span::styled(" select · ", dim()),
            key("Enter"),
            Span::styled(" details · ", dim()),
            key("←/→"),
            Span::styled(" tabs · ", dim()),
            key("?"),
            Span::styled(" help · ", dim()),
            key("q"),
            Span::styled(" quit", dim()),
        ]
    } else {
        vec![
            key("↑/↓"),
            Span::styled(" scroll · ", dim()),
            key("←/→"),
            Span::styled(" tabs · ", dim()),
            key("?"),
            Span::styled(" help · ", dim()),
            key("q"),
            Span::styled(" quit", dim()),
        ]
    };
    frame.render_widget(Paragraph::new(Line::from(spans)), area);

    let visited = app.visited_count();
    let complete = visited == 6;
    let progress_style = if complete {
        Style::new().fg(theme::STAR).add_modifier(Modifier::BOLD)
    } else {
        subtle()
    };
    let progress = Line::from(vec![
        Span::styled(if complete { "★" } else { "◆" }, progress_style),
        Span::styled(format!(" {visited}/6"), progress_style),
    ]);
    frame.render_widget(Paragraph::new(progress).alignment(Alignment::Right), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Input, Key, Mods};
    use ratatui_core::backend::TestBackend;
    use ratatui_core::terminal::Terminal;

    fn app(cols: u16, rows: u16) -> App {
        let mut app = App::new();
        app.resize(cols, rows);
        app
    }

    fn frame_text(app: &mut App) -> String {
        let mut terminal =
            Terminal::new(TestBackend::new(app.cols, app.rows)).expect("TestBackend cannot fail");
        terminal
            .draw(|frame| app.draw(frame))
            .expect("TestBackend cannot fail");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer.content[(y * buffer.area.width + x) as usize].symbol());
            }
            text.push('\n');
        }
        text
    }

    fn press(app: &mut App, key: Key) {
        app.handle(Input::Key {
            key,
            mods: Mods::default(),
        });
    }

    #[test]
    fn renders_header_tabs_and_timeline_cards() {
        let mut app = app(110, 30);
        press(&mut app, Key::Char('2'));
        let text = frame_text(&mut app);
        assert!(text.contains("DEVELOPER SAM"));
        assert!(text.contains("Timeline"));
        assert!(text.contains("● Facebook SWE"));
        assert!(text.contains("│ Working on Flow's type system"));
        assert!(text.contains("[work]"));
        assert!(!text.contains("please resize"));
    }

    #[test]
    fn timeline_cards_have_homepage_buttons() {
        let mut app = app(110, 30);
        press(&mut app, Key::Char('2'));
        frame_text(&mut app);
        assert!(app
            .hit
            .links
            .iter()
            .any(|link| link.url == "https://flow.org"));
    }

    #[test]
    fn opens_timeline_detail_modal_with_links() {
        let mut app = app(110, 30);
        press(&mut app, Key::Char('2'));
        press(&mut app, Key::Enter);
        let text = frame_text(&mut app);
        assert!(text.contains("time:"));
        assert!(text.contains("category:"));
        assert!(text.contains("https://flow.org"));
        assert!(app.hit.modal.is_some());
        assert!(app
            .hit
            .links
            .iter()
            .any(|link| link.url.contains("flow.org")));
        assert!(app.hit.rows.is_empty());
    }

    #[test]
    fn records_clickable_links_on_contact_tab() {
        let mut app = app(110, 30);
        press(&mut app, Key::Char('6'));
        let text = frame_text(&mut app);
        assert!(text.contains("Ways to reach Developer Sam"));
        assert!(text.contains("GITHUB"));
        assert!(app
            .hit
            .links
            .iter()
            .any(|link| link.url == "https://github.com/SamChou19815"));
    }

    #[test]
    fn renders_about_program_on_code_panel() {
        let mut app = app(110, 40);
        let text = frame_text(&mut app);
        assert!(text.contains("import {List} from std.list;"));
        assert!(text.contains("Developer.init(github, projects)"));
        assert!(text.contains("@demo https://samlang.io/demo"));
        assert!(app
            .hit
            .links
            .iter()
            .any(|link| link.url.contains("samlang.io/demo")));
    }

    #[test]
    fn code_panel_paints_the_code_block_background() {
        let mut app = app(110, 40);
        let mut terminal =
            Terminal::new(TestBackend::new(app.cols, app.rows)).expect("TestBackend cannot fail");
        terminal
            .draw(|frame| app.draw(frame))
            .expect("TestBackend cannot fail");
        let buffer = terminal.backend().buffer();
        let width = buffer.area.width as usize;
        let mut found = false;
        for (index, cell) in buffer.content.iter().enumerate() {
            if cell.symbol() == "i"
                && index % width != 0
                && buffer
                    .content
                    .get(index - 1)
                    .map(|c| c.symbol() == " ")
                    .unwrap_or(false)
            {
                let row: String = buffer.content
                    [(index / width) * width..(index / width + 1) * width]
                    .iter()
                    .map(|c| c.symbol())
                    .collect();
                if row.contains("import {List} from std.list") {
                    assert_eq!(
                        cell.style().bg,
                        Some(theme::CODE_BG),
                        "code rows must sit on the code-block background"
                    );
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "the program's import line must be rendered");
    }

    #[test]
    fn renders_markdown_structure_on_work_tab() {
        let mut app = app(110, 40);
        press(&mut app, Key::Char('4'));
        let text = frame_text(&mut app);
        assert!(text.contains("Software Engineer, Flow — Meta"));
        assert!(text.contains("February 2022 — present"));
        assert!(text.contains("https://flow.org"));
        assert!(app
            .hit
            .links
            .iter()
            .any(|link| link.url == "https://flow.org"));
    }

    #[test]
    fn tiny_terminal_shows_resize_hint_instead_of_panicking() {
        let mut tiny = app(20, 8);
        assert!(frame_text(&mut tiny).contains("40×12"));
        let mut narrow = app(80, 10);
        assert!(frame_text(&mut narrow).contains("please resize"));
    }

    #[test]
    fn serializes_to_ansi_with_link_table() {
        let mut app = app(110, 30);
        press(&mut app, Key::Char('6'));
        let mut terminal =
            Terminal::new(TestBackend::new(app.cols, app.rows)).expect("TestBackend cannot fail");
        terminal
            .draw(|frame| app.draw(frame))
            .expect("TestBackend cannot fail");
        let buffer = terminal.backend().buffer().clone();
        let bytes = crate::ansi::serialize_frame(&buffer, &app.hit.links);
        let frame =
            String::from_utf8(bytes.split(|byte| *byte == 0).next().unwrap().to_vec()).unwrap();
        assert!(frame.contains("\x1b[1;1H"));
        assert!(frame.contains("developersam.com"));
        let separator = bytes.iter().position(|byte| *byte == 0).unwrap();
        let count = u32::from_le_bytes(bytes[separator + 1..separator + 5].try_into().unwrap());
        assert_eq!(count as usize, app.hit.links.len());
    }
}
