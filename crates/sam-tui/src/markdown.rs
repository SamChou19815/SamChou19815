//! A small markdown subset rendered through iocraft's `MixedText`, with one
//! consistent, semantic style system — color encodes structure and meaning,
//! never decoration. Links are returned alongside the line for click
//! handling in the view.

use crate::theme;
use iocraft::components::MixedTextContent;
use iocraft::prelude::*;

/// One rendered line: styled segments plus an optional whole-line link.
pub struct ContentLine {
    pub contents: Vec<MixedTextContent>,
    pub link: Option<String>,
}

fn content(text: impl Into<String>, color: crossterm::style::Color) -> MixedTextContent {
    MixedTextContent::new(text.into()).color(color)
}

fn content_bold(text: impl Into<String>, color: crossterm::style::Color) -> MixedTextContent {
    content(text, color).weight(Weight::Bold)
}

fn content_underlined(text: impl Into<String>, color: crossterm::style::Color) -> MixedTextContent {
    content(text, color).decoration(TextDecoration::Underline)
}

/// Parses a markdown document into styled rows.
pub fn parse(source: &str) -> Vec<ContentLine> {
    source.lines().map(parse_line).collect()
}

fn parse_line(raw: &str) -> ContentLine {
    if let Some(text) = raw.strip_prefix("### ") {
        return heading(text, theme::TEXT);
    }
    if let Some(text) = raw.strip_prefix("## ") {
        return heading(text, theme::ACCENT_TEXT);
    }
    if let Some(text) = raw.strip_prefix("# ") {
        return heading(text, theme::ACCENT_TEXT);
    }
    if let Some(text) = raw.strip_prefix("- ") {
        return bullet(text);
    }
    if let Some(text) = raw.strip_prefix("> ") {
        return quote(text);
    }
    if raw.trim() == "---" {
        return ContentLine {
            contents: vec![content("─".repeat(64), theme::BORDER)],
            link: None,
        };
    }
    if raw.trim().is_empty() {
        return ContentLine {
            contents: Vec::new(),
            link: None,
        };
    }
    let (contents, link) = inline(raw, None);
    ContentLine { contents, link }
}

fn heading(text: &str, color: crossterm::style::Color) -> ContentLine {
    let (contents, link) = inline(text, Some(color));
    let _ = link;
    ContentLine {
        contents,
        link: None,
    }
}

fn bullet(text: &str) -> ContentLine {
    let (contents, link) = inline(text, None);
    let mut all = vec![content_bold("• ", theme::ACCENT_TEXT)];
    all.extend(contents);
    ContentLine {
        contents: all,
        link,
    }
}

fn quote(text: &str) -> ContentLine {
    let (contents, _) = inline(text, Some(theme::BORDER));
    let mut all = vec![content("│ ", theme::BORDER)];
    all.extend(contents.iter().map(|piece| piece.clone().italic()));
    ContentLine {
        contents: all,
        link: None,
    }
}

/// A bullet whose entire purpose is one link: `[name](url)` + raw URL.
pub fn bullet_link(name: &str, url: &str) -> ContentLine {
    ContentLine {
        contents: vec![
            content_bold("• ", theme::ACCENT_TEXT),
            content_underlined(name.to_uppercase(), theme::ACCENT_TEXT),
            content(format!(" {url}"), theme::MUTED),
        ],
        link: Some(url.to_string()),
    }
}

/// Renders inline markdown: `**bold**`, `*italic*`, `` `code` `` and links.
/// Returns the first link URL encountered, if any.
fn inline(
    text: &str,
    base: Option<crossterm::style::Color>,
) -> (Vec<MixedTextContent>, Option<String>) {
    let base_color = base.unwrap_or(theme::TEXT);
    let mut contents: Vec<MixedTextContent> = Vec::new();
    let mut first_link: Option<String> = None;
    let mut rest = text;
    let mut bold = false;
    let mut italic = false;
    while !rest.is_empty() {
        let cut = rest.find(['*', '`', '[']).unwrap_or(rest.len());
        if cut > 0 {
            contents.push(styled(rest[..cut].to_string(), base_color, bold, italic));
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
                    contents.push(content(tail[..end].to_string(), theme::ACCENT_TEXT));
                    rest = &tail[end + 1..];
                }
                None => {
                    contents.push(content(tail.to_string(), theme::ACCENT_TEXT));
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
                    contents.push(content_underlined(name.to_uppercase(), theme::ACCENT_TEXT));
                    contents.push(content(format!(" {url}"), theme::MUTED));
                    rest = &tail[end + 1..];
                    continue;
                }
            }
            contents.push(styled("[".to_string(), base_color, bold, italic));
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix('*') {
            italic = !italic;
            rest = tail;
        } else {
            contents.push(styled(rest[..1].to_string(), base_color, bold, italic));
            rest = &rest[1..];
        }
    }
    (contents, first_link)
}

fn styled(
    text: String,
    color: crossterm::style::Color,
    bold: bool,
    italic: bool,
) -> MixedTextContent {
    let mut piece = content(text, color);
    if bold {
        piece = piece.weight(Weight::Bold);
    }
    if italic {
        piece = piece.italic();
    }
    piece
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structure() {
        let doc = parse("# Title\n\n## Sub\n- [one](https://a)\n> note\nplain");
        assert_eq!(doc.len(), 6);
        assert_eq!(doc[0].contents[0].text, "Title");
        let bullet = &doc[3];
        assert_eq!(bullet.link.as_deref(), Some("https://a"));
        let text: String = bullet.contents.iter().map(|c| c.text.as_str()).collect();
        assert!(text.contains("https://a"));
        assert_eq!(doc[4].contents[0].text, "│ ");
    }

    #[test]
    fn inline_emphasis() {
        let (contents, _) = inline("a **b** `c` *d*", None);
        let text: String = contents.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(text, "a b c d");
        assert_eq!(contents[1].weight, Weight::Bold);
        assert!(contents
            .iter()
            .any(|piece| piece.text == "d" && piece.italic));
    }

    #[test]
    fn link_styling_is_semantic() {
        let (contents, link) = inline("see [docs](https://x.y) now", None);
        assert_eq!(link.as_deref(), Some("https://x.y"));
        assert!(contents
            .iter()
            .any(|c| c.decoration == TextDecoration::Underline));
    }
}
