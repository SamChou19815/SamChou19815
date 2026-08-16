//! A small markdown subset with one consistent, semantic style system —
//! color encodes structure and meaning, never decoration:
//!
//! - `##`/`###` headings — bold; top-level headings carry the accent color
//! - `-` bullets — accent marker + content (a bullet holding a link is
//!   rendered as *link text* plus the raw URL in dim, since terminals cannot
//!   always follow links)
//! - `>` quotes — dim, italic, behind a vertical bar
//! - `[text](url)` — underlined link text; the whole row becomes a click
//!   region
//! - `**bold**`, `*italic*`, `` `code` `` — inline emphasis
//!
//! Blocks map one-to-one to screen rows, so scrolling is row-based.

use crate::theme;
use ratatui_core::style::{Color, Modifier, Style};
use ratatui_core::text::Span;

/// The accent as text (blue-600): headings, markers, buttons, keycaps —
/// everything accent-colored that must read clearly on the light background.
pub const ACCENT: Color = theme::ACCENT_TEXT;
/// Interactive affordances: keycaps and inline code.
pub const KEY: Color = theme::ACCENT_TEXT;
/// Secondary text: subheaders, raw URLs, hints.
pub const DIM: Color = theme::MUTED;

pub struct ContentLine {
    pub spans: Vec<Span<'static>>,
    /// When set, the whole row is a clickable link region for this URL.
    pub link: Option<String>,
}

pub fn accent() -> Style {
    Style::new().fg(ACCENT)
}

pub fn key_style() -> Style {
    Style::new().fg(KEY)
}

pub fn dim() -> Style {
    Style::new().fg(DIM)
}

/// The homepage's `ButtonLink`: uppercase, bold, blue.
pub fn link_style() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Parses a markdown document into styled rows.
pub fn parse(source: &str) -> Vec<ContentLine> {
    source.lines().map(parse_line).collect()
}

fn parse_line(raw: &str) -> ContentLine {
    if let Some(text) = raw.strip_prefix("### ") {
        return heading(
            text,
            Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
        );
    }
    if let Some(text) = raw.strip_prefix("## ") {
        return heading(text, accent().add_modifier(Modifier::BOLD));
    }
    if let Some(text) = raw.strip_prefix("# ") {
        return heading(text, accent().add_modifier(Modifier::BOLD));
    }
    if let Some(text) = raw.strip_prefix("- ") {
        return bullet(text);
    }
    if let Some(text) = raw.strip_prefix("> ") {
        return quote(text);
    }
    if raw.trim() == "---" {
        return ContentLine {
            spans: vec![Span::styled("─".repeat(64), dim())],
            link: None,
        };
    }
    if raw.trim().is_empty() {
        return ContentLine {
            spans: Vec::new(),
            link: None,
        };
    }
    let (spans, link) = inline(raw, Style::new());
    ContentLine { spans, link }
}

fn heading(text: &str, style: Style) -> ContentLine {
    let (spans, link) = inline(text, style);
    ContentLine { spans, link }
}

fn bullet(text: &str) -> ContentLine {
    let (content_spans, link) = inline(text, Style::new());
    let mut spans = vec![Span::styled("• ", accent().add_modifier(Modifier::BOLD))];
    spans.extend(content_spans);
    ContentLine { spans, link }
}

fn quote(text: &str) -> ContentLine {
    let (content_spans, link) = inline(text, dim().add_modifier(Modifier::ITALIC));
    let mut spans = vec![Span::styled("│ ", dim())];
    spans.extend(content_spans);
    ContentLine { spans, link }
}

/// A bullet whose entire purpose is one link: an uppercase button + raw URL.
pub fn bullet_link(name: &str, url: &str) -> ContentLine {
    ContentLine {
        spans: vec![
            Span::styled("• ", accent().add_modifier(Modifier::BOLD)),
            Span::styled(name.to_uppercase(), link_style()),
            Span::styled(format!(" {url}"), dim()),
        ],
        link: Some(url.to_string()),
    }
}

/// Renders inline markdown: `**bold**`, `*italic*`, `` `code` `` and links.
/// Returns the first link URL encountered, if any.
fn inline(text: &str, base: Style) -> (Vec<Span<'static>>, Option<String>) {
    let mut spans = Vec::new();
    let mut first_link: Option<String> = None;
    let mut rest = text;
    let mut bold = false;
    let mut italic = false;
    while !rest.is_empty() {
        let cut = rest.find(['*', '`', '[']).unwrap_or(rest.len());
        if cut > 0 {
            spans.push(Span::styled(
                rest[..cut].to_string(),
                effective(base, bold, italic),
            ));
        }
        rest = &rest[cut..];
        if rest.is_empty() {
            break;
        }
        if let Some(tail) = rest.strip_prefix("**") {
            bold = !bold;
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix('`') {
            match tail.find('`') {
                Some(end) => {
                    spans.push(Span::styled(tail[..end].to_string(), key_style()));
                    rest = &tail[end + 1..];
                }
                None => {
                    spans.push(Span::styled(tail.to_string(), key_style()));
                    rest = "";
                }
            }
        } else if let Some(tail) = rest.strip_prefix('[') {
            if let Some(close) = tail.find("](") {
                if let Some(end) = tail[close + 2..].find(')').map(|index| close + 2 + index) {
                    let name = &tail[..close];
                    let url = &tail[close + 2..end];
                    if first_link.is_none() {
                        first_link = Some(url.to_string());
                    }
                    // Uppercase, exactly like the homepage's buttons.
                    spans.push(Span::styled(name.to_uppercase(), link_style()));
                    spans.push(Span::styled(format!(" {url}"), dim()));
                    rest = &tail[end + 1..];
                    continue;
                }
            }
            spans.push(Span::styled("[".to_string(), effective(base, bold, italic)));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix('*') {
            italic = !italic;
            rest = tail;
        } else {
            // A lone special character with nothing to close: literal.
            spans.push(Span::styled(
                rest[..1].to_string(),
                effective(base, bold, italic),
            ));
            rest = &rest[1..];
        }
    }
    (spans, first_link)
}

fn effective(base: Style, bold: bool, italic: bool) -> Style {
    let mut style = base;
    if bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structure() {
        let doc = parse("# Title\n\n## Sub\n- [one](https://a)\n> note\nplain");
        assert_eq!(doc.len(), 6);
        assert_eq!(doc[0].spans[0].content, "Title"); // heading keeps text
        let bullet = &doc[3];
        assert_eq!(bullet.link.as_deref(), Some("https://a"));
        let url_span = bullet
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect::<Vec<_>>();
        assert!(url_span.join(" ").contains("https://a"));
        assert!(doc[4].spans.iter().any(|s| s.content == "│ "));
    }

    #[test]
    fn inline_emphasis() {
        let (spans, _) = inline("a **b** `c` *d*", Style::new());
        let text: String = spans.iter().map(|s| s.content.to_string()).collect();
        assert_eq!(text, "a b c d");
        assert!(spans
            .iter()
            .any(|s| s.content == "b" && s.style.add_modifier.contains(Modifier::BOLD)));
        assert!(spans
            .iter()
            .any(|s| s.content == "c" && s.style.fg == Some(KEY)));
        assert!(spans
            .iter()
            .any(|s| s.content == "d" && s.style.add_modifier.contains(Modifier::ITALIC)));
    }

    #[test]
    fn link_styling_is_semantic() {
        let (spans, link) = inline("see [docs](https://x.y) now", Style::new());
        assert_eq!(link.as_deref(), Some("https://x.y"));
        assert!(spans.iter().any(|s| s.style == link_style()));
    }
}
