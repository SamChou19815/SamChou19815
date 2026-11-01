//! Turning laid-out graph lines into styled ratatui lines.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};

use crate::dag::{Dag, Node, Preview};
use crate::github::{PrInfo, PrState};
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

pub struct Rendered {
    pub lines: Vec<Line<'static>>,
    /// First line index of each node.
    pub node_line: HashMap<String, usize>,
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
        (PrState::Merged, _) => (format!("#{} merged", pr.number), Color::Magenta),
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

fn node_line_spans(n: &Node, prs: &HashMap<String, PrInfo>) -> Vec<Span<'static>> {
    let c = &n.commit;
    let faded = is_faded(n.preview);
    let mut spans = vec![];
    let prefix_len = c.change_prefix_len.min(c.short_change.len());
    if c.short_change == "(new)" {
        spans.push(Span::styled(
            "(new)",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::styled(
            c.short_change[..prefix_len].to_string(),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            c.short_change[prefix_len..].to_string(),
            Style::default().fg(Color::DarkGray),
        ));
    }
    if !c.author_email.is_empty() {
        let who = c.author_email.split('@').next().unwrap_or("").to_string();
        spans.push(Span::raw(" "));
        spans.push(Span::styled(who, Style::default().fg(Color::Yellow)));
    }
    let when = relative_time(&c.timestamp);
    if !when.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(when, Style::default().fg(Color::Cyan)));
    }
    for b in &c.bookmarks {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(b.clone(), Style::default().fg(Color::Magenta)));
    }
    // Remote bookmarks without a same-named local one (others' branches).
    for rb in &c.remote_bookmarks {
        let name = rb.split('@').next().unwrap_or(rb);
        if !c.bookmarks.iter().any(|b| b == name) {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                rb.clone(),
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::DIM),
            ));
        }
    }
    for t in &c.tags {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(t.clone(), Style::default().fg(Color::Yellow)));
    }
    for w in &c.working_copies {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("{w}@"),
            Style::default().fg(Color::Green),
        ));
    }
    if !faded {
        if let Some(pr) = pr_for(n, prs) {
            spans.push(Span::raw(" "));
            spans.extend(pr_badge(pr));
        }
    }
    if !c.short_commit.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            c.short_commit.clone(),
            Style::default().fg(Color::Blue),
        ));
    }
    if c.conflict {
        spans.push(" conflict".red().bold());
    }
    if c.divergent {
        spans.push(" divergent".red());
    }
    if faded {
        for s in &mut spans {
            s.style = s.style.add_modifier(Modifier::DIM);
        }
    }
    spans
}

fn detail_spans(n: &Node) -> Vec<Span<'static>> {
    let c = &n.commit;
    let mut spans = vec![];
    if c.empty && n.preview != Some(Preview::New) {
        spans.push(Span::styled("(empty) ", Style::default().fg(Color::Green)));
    }
    if c.title().is_empty() {
        spans.push(Span::styled(
            "(no description set)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::DIM),
        ));
    } else {
        spans.push(Span::raw(c.title().to_string()));
    }
    let style = preview_style(n.preview);
    if is_faded(n.preview) {
        for s in &mut spans {
            s.style = s.style.patch(style);
        }
    }
    spans
}

fn glyph_style(n: &Node) -> Style {
    if n.preview.is_some() {
        return preview_style(n.preview).remove_modifier(Modifier::CROSSED_OUT);
    }
    let c = &n.commit;
    if c.is_wc {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if c.conflict {
        Style::default().fg(Color::Red)
    } else if c.immutable {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}

pub fn render(dag: &Dag, prs: &HashMap<String, PrInfo>, marks: &HashMap<String, Mark>) -> Rendered {
    let laid = graph::layout(dag);
    let mut lines = Vec::with_capacity(laid.len());
    let mut node_line = HashMap::new();
    for (i, l) in laid.iter().enumerate() {
        let n = &dag.nodes[&l.owner];
        let mut spans: Vec<Span<'static>> = vec![];
        match (l.kind, l.node_at) {
            (Kind::Node, Some(at)) => {
                node_line.entry(l.owner.clone()).or_insert(i);
                let chars: Vec<char> = l.graph.chars().collect();
                spans.push(Span::styled(
                    chars[..at].iter().collect::<String>(),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.push(Span::styled(chars[at].to_string(), glyph_style(n)));
                spans.push(Span::styled(
                    chars[at + 1..].iter().collect::<String>(),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.extend(node_line_spans(n, prs));
                if let Some(note) = &n.note {
                    spans.push(Span::raw("  "));
                    let st = preview_style(n.preview)
                        .remove_modifier(Modifier::CROSSED_OUT | Modifier::DIM);
                    spans.push(Span::styled(format!("◀ {note}"), st));
                }
            }
            (Kind::Detail, _) => {
                spans.push(Span::styled(
                    l.graph.clone(),
                    Style::default().fg(Color::DarkGray),
                ));
                spans.extend(detail_spans(n));
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
                    Mark::Cursor => Style::default().bg(Color::Rgb(40, 52, 80)),
                    Mark::Source => Style::default().bg(Color::Rgb(70, 55, 20)),
                    Mark::Target => Style::default().bg(Color::Rgb(25, 70, 45)),
                });
            }
        }
        lines.push(line);
    }
    // Node line index should point at the node line itself, not a merge
    // connector preceding it.
    for (i, l) in laid.iter().enumerate() {
        if l.kind == Kind::Node {
            node_line.insert(l.owner.clone(), i);
        }
    }
    Rendered { lines, node_line }
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
