//! Turning laid-out graph lines into styled ratatui lines.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};

use crate::dag::{Dag, Node, Preview};
use crate::github::{CheckSignal, PrInfo, PrState, Review};
use crate::graph::{self, Kind};

/// Extra emphasis for specific nodes in a pane.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// The keyboard cursor.
    Cursor,
    /// The commit an operation acts on.
    Source,
    /// The target being picked.
    Target,
}

/// The few colors that must adapt to the terminal's background — everything
/// else is plain ANSI colors the terminal already maps to its theme. Dark
/// rows highlighted with a dark background are unreadable with a light
/// terminal's dark text, and vice versa.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Theme {
    /// Cursor / selected row background.
    pub cursor: Color,
    /// The commit an operation acts on, while picking a destination.
    pub source: Color,
    /// The destination being picked.
    pub target: Color,
    /// Toolbar pill background, with its label and icon colors.
    pub pill: Color,
    pub pill_text: Color,
    pub pill_accent: Color,
}

impl Theme {
    /// Asks the terminal for its background color (OSC 11) before the TUI
    /// takes over the screen. `JJ_HELPER_THEME=light|dark` overrides, and
    /// terminals that can't answer fall back to dark.
    pub fn detect() -> Theme {
        if let Some(t) = std::env::var("JJ_HELPER_THEME")
            .ok()
            .as_deref()
            .and_then(parse_override)
        {
            return t;
        }
        match terminal_colorsaurus::theme_mode(terminal_colorsaurus::QueryOptions::default()) {
            Ok(terminal_colorsaurus::ThemeMode::Light) => Theme::light(),
            _ => Theme::dark(),
        }
    }

    /// Current look: navy cursor, olive source, forest target, slate pills.
    fn dark() -> Theme {
        Theme {
            cursor: Color::Rgb(40, 52, 80),
            source: Color::Rgb(70, 55, 20),
            target: Color::Rgb(25, 70, 45),
            pill: Color::Rgb(64, 72, 90),
            pill_text: Color::White,
            pill_accent: Color::Yellow,
        }
    }

    /// Pastel twins of the dark colors: same hues, light enough for a light
    /// terminal's dark text to stay readable.
    fn light() -> Theme {
        Theme {
            cursor: Color::Rgb(215, 227, 248),
            source: Color::Rgb(248, 234, 192),
            target: Color::Rgb(214, 240, 224),
            pill: Color::Rgb(225, 229, 236),
            pill_text: Color::Black,
            pill_accent: Color::Rgb(146, 100, 12),
        }
    }
}

fn parse_override(s: &str) -> Option<Theme> {
    match s.trim().to_ascii_lowercase().as_str() {
        "light" => Some(Theme::light()),
        "dark" => Some(Theme::dark()),
        _ => None,
    }
}

pub struct Rendered {
    pub lines: Vec<Line<'static>>,
    /// First line index of each node.
    pub node_line: HashMap<String, usize>,
    /// Change rows by line index, for mouse hit-testing.
    pub nodes: HashMap<u16, String>,
}

pub fn relative_time(ts: &str) -> String {
    let Ok(t) = DateTime::parse_from_rfc3339(ts) else {
        return String::new();
    };
    let secs = (Utc::now() - t.with_timezone(&Utc)).num_seconds().max(0);
    match secs {
        s if s < 60 => "just now".into(),
        s if s < 3600 => format!("{}m ago", s / 60),
        s if s < 86_400 => format!("{}h ago", s / 3600),
        s if s < 86_400 * 14 => format!("{}d ago", s / 86_400),
        s if s < 86_400 * 365 => format!("{}w ago", s / (86_400 * 7)),
        s => format!("{}y ago", s / (86_400 * 365)),
    }
}

pub fn preview_style(p: Option<Preview>) -> Style {
    match p {
        None => Style::default(),
        Some(Preview::Moved) => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        Some(Preview::MovedDescendant) => Style::default().fg(Color::Cyan),
        Some(Preview::Ghost) => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC),
        Some(Preview::Hidden) | Some(Preview::SquashSource) => Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::CROSSED_OUT | Modifier::DIM),
        Some(Preview::SquashTarget) | Some(Preview::WcDestination) | Some(Preview::BookmarkNew) => {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        }
        Some(Preview::New) => Style::default().fg(Color::Green),
        Some(Preview::WcPrevious) | Some(Preview::BookmarkOld) => {
            Style::default().add_modifier(Modifier::DIM)
        }
        Some(Preview::Rewritten) => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        Some(Preview::Submit) => Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
        Some(Preview::Ship) => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    }
}

fn is_faded(p: Option<Preview>) -> bool {
    matches!(
        p,
        Some(Preview::Ghost | Preview::Hidden | Preview::SquashSource)
    )
}

/// PR for a node: the first of its bookmarks that has one (open wins).
pub fn pr_for<'a>(n: &Node, prs: &'a HashMap<String, PrInfo>) -> Option<&'a PrInfo> {
    let found: Vec<&PrInfo> = n
        .commit
        .bookmarks
        .iter()
        .filter_map(|b| prs.get(b))
        .collect();
    found
        .iter()
        .find(|p| p.is_open())
        .or(found.first())
        .copied()
}

pub fn pr_badge(pr: &PrInfo) -> Vec<Span<'static>> {
    let mut spans = vec![];
    let (label, color) = match (pr.state, pr.draft) {
        (PrState::Merged, _) => (format!("#{} shipped", pr.number), Color::Magenta),
        (PrState::Closed, _) => (format!("#{} closed", pr.number), Color::Red),
        (PrState::Open, true) => (format!("#{} draft", pr.number), Color::Gray),
        (PrState::Open, false) => (format!("#{}", pr.number), Color::Green),
    };
    spans.push(Span::styled(
        label,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    if pr.is_open() {
        match pr.review_decision.as_deref() {
            Some("APPROVED") => spans.push(" ✓approved".green()),
            Some("CHANGES_REQUESTED") => spans.push(" ✗changes".red()),
            _ if !pr.reviews.is_empty() => spans.push(" ◑reviewed".yellow()),
            _ => {}
        }
        match pr.checks.as_deref() {
            Some("SUCCESS") => spans.push(" ●".green()),
            Some("FAILURE") | Some("ERROR") => spans.push(" ●".red()),
            Some(_) => spans.push(" ●".yellow()),
            None => {}
        }
        if let Some(q) = &pr.merge_queue {
            spans.push(format!(" ⇢queue:{}", q.to_lowercase()).cyan());
        }
        if let Some(s) = &pr.stack {
            spans.push(format!(" ⧉{}/{}", s.position, s.size).blue());
        }
    }
    spans
}

/// One reviewer line for the details pane: who, and their latest verdict.
/// Reviews made on an older head render dimmed with a note.
pub fn review_line(r: &Review, head_oid: &str) -> Line<'static> {
    let (glyph, label, color) = match r.state.as_str() {
        "APPROVED" => ("✓", "approved", Color::Green),
        "CHANGES_REQUESTED" => ("✗", "changes requested", Color::Red),
        "DISMISSED" => ("○", "dismissed", Color::DarkGray),
        _ => ("◦", "commented", Color::Yellow),
    };
    let mut spans = vec![
        Span::raw("  "),
        Span::styled(glyph.to_string(), Style::default().fg(color)),
        Span::raw(format!(" {} ", r.login)),
        Span::styled(label.to_string(), Style::default().fg(color)),
    ];
    if r.oid != head_oid {
        spans.push(Span::styled(
            "  (older commit)",
            Style::default().fg(Color::DarkGray),
        ));
    }
    Line::from(spans)
}

/// One status-check line for the details pane: the check's name and its
/// outcome, colored green / red / yellow like the row badge's CI dot.
pub fn check_line(s: &CheckSignal) -> Line<'static> {
    let (glyph, color) = match s.state.as_str() {
        "SUCCESS" => ("✓", Color::Green),
        "FAILURE" | "ERROR" | "TIMED_OUT" | "ACTION_REQUIRED" => ("✗", Color::Red),
        "NEUTRAL" | "SKIPPED" | "CANCELLED" => ("○", Color::DarkGray),
        // PENDING, QUEUED, IN_PROGRESS, WAITING, EXPECTED, …
        _ => ("●", Color::Yellow),
    };
    Line::from(vec![
        Span::raw("  "),
        Span::styled(glyph.to_string(), Style::default().fg(color)),
        Span::raw(format!(" {}", s.name)),
        Span::styled(
            format!("  {}", s.state.to_lowercase().replace('_', " ")),
            Style::default().fg(color),
        ),
    ])
}

/// One VisualJJ-style row: pills (You are here, branches, PR) first, the
/// description as the primary text, then the time and the change-id chip
/// right-aligned at the pane's right edge when `avail` leaves room for it.
/// `landed` grays the whole row out — history that already landed on trunk.
fn node_line_spans(
    n: &Node,
    prs: &HashMap<String, PrInfo>,
    avail: usize,
    landed: bool,
) -> Vec<Span<'static>> {
    let c = &n.commit;
    let faded = is_faded(n.preview) || landed;
    let mut spans = vec![];
    if c.is_wc {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            "You are here · editing",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }
    // Branch pills (VisualJJ renders them blue).
    for b in &c.bookmarks {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            b.clone(),
            Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        ));
    }
    // Remote bookmarks without a same-named local one (others' branches).
    for rb in &c.remote_bookmarks {
        let name = rb.split('@').next().unwrap_or(rb);
        if !c.bookmarks.iter().any(|b| b == name) {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                rb.clone(),
                Style::default().fg(Color::Blue).add_modifier(Modifier::DIM),
            ));
        }
    }
    for t in &c.tags {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(t.clone(), Style::default().fg(Color::Yellow)));
    }
    if !faded {
        if let Some(pr) = pr_for(n, prs) {
            spans.push(Span::raw(" "));
            spans.extend(pr_badge(pr));
        }
    }
    // The description is the primary text; an undescribed change gets
    // VisualJJ's inline "Describe" affordance.
    spans.push(Span::raw("  "));
    if c.title().is_empty() {
        spans.push(Span::styled(
            "Describe…",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::DIM),
        ));
    } else {
        let mut title = String::new();
        if c.empty && n.preview != Some(Preview::New) {
            title.push_str("(empty) ");
        }
        title.push_str(c.title());
        spans.push(Span::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        ));
    }
    if c.conflict {
        spans.push(" conflict".red().bold());
    }
    if c.divergent {
        spans.push(" divergent".red());
    }
    if let Some(note) = &n.note {
        spans.push(Span::raw("  "));
        let st = preview_style(n.preview).remove_modifier(Modifier::CROSSED_OUT | Modifier::DIM);
        spans.push(Span::styled(format!("◀ {note}"), st));
    }
    // Time and the change-id chip live at the end of the row; author and
    // commit hash stay in the details pane, like VisualJJ.
    let mut right = vec![];
    let when = relative_time(&c.timestamp);
    if !when.is_empty() {
        right.push(Span::styled(when, Style::default().fg(Color::Cyan)));
        right.push(Span::raw("  "));
    }
    if c.short_change != "(new)" {
        let prefix_len = c.change_prefix_len.min(c.short_change.len());
        right.push(Span::styled(
            c.short_change[..prefix_len].to_string(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        right.push(Span::styled(
            c.short_change[prefix_len..].to_string(),
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        right.push(Span::styled(
            "(new)",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if c.is_wc {
        right.push(Span::styled("@", Style::default().fg(Color::Magenta)));
    }
    if !right.is_empty() {
        let left_w: usize = spans.iter().map(|s| s.width()).sum();
        let right_w: usize = right.iter().map(|s| s.width()).sum();
        // Right-align against the pane edge minus one cell of extra air;
        // at least two spaces before the block, and rows that overflow
        // simply clip at the pane edge.
        let pad = avail
            .saturating_sub(1)
            .saturating_sub(left_w + right_w)
            .max(2);
        spans.push(Span::raw(" ".repeat(pad)));
        spans.extend(right);
    }
    if faded {
        for s in &mut spans {
            s.style = s.style.add_modifier(Modifier::DIM);
        }
    }
    spans
}

fn glyph_style(n: &Node, landed: bool) -> Style {
    if n.preview.is_some() {
        return preview_style(n.preview).remove_modifier(Modifier::CROSSED_OUT);
    }
    if landed {
        // Landed trunk history: a gray diamond for context, not action.
        return Style::default().fg(Color::DarkGray);
    }
    let c = &n.commit;
    if c.is_wc {
        // VisualJJ's working-copy node is a filled blue circle.
        Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD)
    } else if c.conflict {
        Style::default().fg(Color::Red)
    } else if c.immutable {
        // VisualJJ's trunk diamond is green.
        Style::default().fg(Color::Green)
    } else {
        Style::default()
    }
}

/// `width` is the inner width the rows will be rendered into (0 when
/// unknown); the time and change-id chip are right-aligned against it.
pub fn render(
    dag: &Dag,
    prs: &HashMap<String, PrInfo>,
    marks: &HashMap<String, Mark>,
    width: usize,
    theme: &Theme,
) -> Rendered {
    let laid = graph::layout(dag);
    let landed = dag.landed();
    let mut lines = Vec::with_capacity(laid.len());
    let mut node_line = HashMap::new();
    for (i, l) in laid.iter().enumerate() {
        let n = &dag.nodes[&l.owner];
        let is_landed = n.preview.is_none() && landed.contains(&l.owner);
        let mut spans: Vec<Span<'static>> = vec![];
        match (l.kind, l.node_at) {
            (Kind::Node, Some(at)) => {
                node_line.entry(l.owner.clone()).or_insert(i);
                let chars: Vec<char> = l.graph.chars().collect();
                spans.push(Span::styled(
                    chars[..at].iter().collect::<String>(),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(
                    chars[at].to_string(),
                    glyph_style(n, is_landed),
                ));
                spans.push(Span::styled(
                    chars[at + 1..].iter().collect::<String>(),
                    Style::default().fg(Color::DarkGray),
                ));
                let avail = width.saturating_sub(l.graph.chars().count());
                spans.extend(node_line_spans(n, prs, avail, is_landed));
            }
            _ => {
                node_line.entry(l.owner.clone()).or_insert(i);
                spans.push(Span::styled(
                    l.graph.clone(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }
        let mut line = Line::from(spans);
        if l.kind != Kind::Connector {
            if let Some(mark) = marks.get(&l.owner) {
                line = line.style(match mark {
                    Mark::Cursor => Style::default().bg(theme.cursor),
                    Mark::Source => Style::default().bg(theme.source),
                    Mark::Target => Style::default().bg(theme.target),
                });
            }
        }
        lines.push(line);
    }
    // Node line index should point at the node line itself, not a merge
    // connector preceding it.
    let mut nodes = HashMap::new();
    for (i, l) in laid.iter().enumerate() {
        if l.kind == Kind::Node {
            node_line.insert(l.owner.clone(), i);
            nodes.insert(i as u16, l.owner.clone());
        }
    }
    Rendered {
        lines,
        node_line,
        nodes,
    }
}

/// Diff text with git-style coloring.
pub fn diff_lines(text: &str) -> Vec<Line<'static>> {
    text.lines()
        .map(|l| {
            let style =
                if l.starts_with("+++") || l.starts_with("---") || l.starts_with("diff --git") {
                    Style::default().add_modifier(Modifier::BOLD)
                } else if l.starts_with('+') {
                    Style::default().fg(Color::Green)
                } else if l.starts_with('-') {
                    Style::default().fg(Color::Red)
                } else if l.starts_with("@@") {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
            Line::styled(l.to_string(), style)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_override_parses() {
        assert_eq!(parse_override("light"), Some(Theme::light()));
        assert_eq!(parse_override(" DARK "), Some(Theme::dark()));
        assert_eq!(parse_override("nope"), None);
        // Light and dark must never collapse into one palette.
        assert_ne!(Theme::light().cursor, Theme::dark().cursor);
    }

    #[test]
    fn signal_lines_carry_names() {
        let review = Review {
            login: "sam".into(),
            oid: "c0".into(),
            state: "APPROVED".into(),
        };
        let line = review_line(&review, "c0").spans;
        let text: String = line
            .iter()
            .map(|s| s.content.to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("sam") && text.contains("approved"), "{text}");
        // A review of an older commit says so.
        let stale = review_line(&review, "c1").spans;
        let text: String = stale
            .iter()
            .map(|s| s.content.to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("older commit"), "{text}");

        let check = CheckSignal {
            name: "build / CI".into(),
            state: "TIMED_OUT".into(),
        };
        let text: String = check_line(&check)
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect::<Vec<_>>()
            .join("");
        assert!(
            text.contains("build / CI") && text.contains("timed out"),
            "{text}"
        );
    }
}
