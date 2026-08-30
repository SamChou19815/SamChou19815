//! A small markdown subset rendered through iocraft's `MixedText`, with one
//! consistent, semantic style system — color encodes structure and meaning,
//! never decoration. Links are returned alongside the line for click handling
//! in the view.
//!
//! Blog bodies arrive pre-normalized by `build.rs`: every image URL in one is
//! already site-root-relative, so [`post_blocks`] can hand each one to
//! [`image`] as it finds it.

use crate::{image, theme};
use iocraft::components::MixedTextContent;
use iocraft::prelude::*;

/// One rendered line: styled segments plus an optional whole-line link.
pub struct ContentLine {
    pub contents: Vec<MixedTextContent>,
    pub link: Option<String>,
}

/// One unit of a rendered post. `Line` is always exactly one terminal row —
/// text is wrapped here rather than by `MixedText`, so the reader's scroll
/// math and what is painted can never disagree.
pub enum Block {
    Line(ContentLine),
    /// `url` is owned rather than borrowed from the body: a body is decrypted
    /// on demand ([`crate::crypt`]), so there is no `'static` text to point at.
    Image {
        url: String,
        alt: String,
    },
}

/// Renders a post body into blocks, wrapped to `width` columns.
pub fn post_blocks(body: &str, width: usize) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut in_code = false;

    fn flush(blocks: &mut Vec<Block>, paragraph: &mut Vec<&str>, width: usize) {
        if paragraph.is_empty() {
            return;
        }
        let text = paragraph.join(" ");
        paragraph.clear();
        let (contents, link) = inline(&text, None);
        push_wrapped(blocks, contents, width, 0, link);
    }

    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            flush(&mut blocks, &mut paragraph, width);
            in_code = !in_code;
            let lang = trimmed.trim_start_matches('`').trim();
            let mut rule = format!("── {lang} ");
            let pad = width.saturating_sub(rule.chars().count());
            if pad > 0 {
                rule.push_str(&"─".repeat(pad));
            } else {
                rule = truncate(&rule, width);
            }
            blocks.push(Block::Line(ContentLine {
                contents: vec![content(rule, theme::BORDER)],
                link: None,
            }));
        } else if in_code {
            blocks.push(Block::Line(ContentLine {
                contents: vec![content(truncate(line, width), theme::PLAIN)],
                link: None,
            }));
        } else if trimmed.starts_with('<') {
            // A lone HTML element: swallow it whole, keeping its `src` if it
            // is an image. The corpus uses these for `<br />`s and one
            // centered picture.
            let mut src = html_src(trimmed);
            if !trimmed.trim_end().ends_with('>') {
                for further in lines.by_ref() {
                    if src.is_none() {
                        src = html_src(further.trim());
                    }
                    if further.trim_end().ends_with('>') {
                        break;
                    }
                }
            }
            if let Some(url) = src.filter(|url| image::size(url, image::HERO).is_some()) {
                blocks.push(Block::Image {
                    url: url.to_string(),
                    alt: String::new(),
                });
            }
        } else if let Some((alt, url, rest)) = leading_image(trimmed) {
            flush(&mut blocks, &mut paragraph, width);
            if image::size(url, image::HERO).is_some() {
                blocks.push(Block::Image {
                    url: url.to_string(),
                    alt: alt.to_string(),
                });
            }
            // A caption trailing the image starts a new paragraph.
            let rest = rest.trim();
            if !rest.is_empty() {
                paragraph.push(rest);
            }
        } else if line.trim().is_empty() {
            flush(&mut blocks, &mut paragraph, width);
            blocks.push(Block::Line(ContentLine {
                contents: Vec::new(),
                link: None,
            }));
        } else if let Some((text, color)) = trimmed
            .strip_prefix("### ")
            .map(|text| (text, theme::TEXT))
            .or_else(|| {
                trimmed
                    .strip_prefix("## ")
                    .map(|text| (text, theme::ACCENT_TEXT))
            })
            .or_else(|| {
                trimmed
                    .strip_prefix("# ")
                    .map(|text| (text, theme::ACCENT_TEXT))
            })
        {
            flush(&mut blocks, &mut paragraph, width);
            push_wrapped(&mut blocks, heading(text, color), width, 0, None);
        } else if let Some(text) = trimmed
            .strip_prefix("- ")
            .or_else(|| numbered_item(trimmed))
        {
            flush(&mut blocks, &mut paragraph, width);
            let (contents, link) = bullet(text);
            push_wrapped(&mut blocks, contents, width, 2, link);
        } else if let Some(text) = trimmed.strip_prefix("> ") {
            flush(&mut blocks, &mut paragraph, width);
            // The rail prefixes every row, so the text wraps in its own
            // column and the `│ ` is stitched on afterwards.
            for mut row in wrap(quote(text), width.saturating_sub(2).max(1), 0) {
                row.insert(0, content("│ ", theme::BORDER));
                blocks.push(Block::Line(ContentLine {
                    contents: row,
                    link: None,
                }));
            }
        } else {
            paragraph.push(trimmed);
        }
    }
    flush(&mut blocks, &mut paragraph, width);
    blocks
}

/// The text after a numbered list item's `N. ` marker.
fn numbered_item(line: &str) -> Option<&str> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || !line[digits..].starts_with(". ") {
        return None;
    }
    Some(&line[digits + 2..])
}

/// A `![alt](url)` reference leading a line: its alt, url and trailing text.
fn leading_image(line: &str) -> Option<(&str, &str, &str)> {
    let after_marker = line.strip_prefix("![")?;
    let close = after_marker.find("](")?;
    let after_url_open = &after_marker[close + 2..];
    let end = after_url_open.find(')')?;
    Some((
        &after_marker[..close],
        &after_url_open[..end],
        &after_url_open[end + 1..],
    ))
}

/// The URL in an HTML fragment's `src="…"`, if it carries one.
fn html_src(html: &str) -> Option<&str> {
    let start = html.find("src=\"")? + 5;
    let rest = &html[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Truncates to `width` columns, marking the cut with an ellipsis.
pub fn truncate(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let keep = width.saturating_sub(1);
    text.chars()
        .take(keep)
        .chain(std::iter::once('…'))
        .collect()
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

/// A bullet whose entire purpose is one link: its name, then the raw URL.
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

fn heading(text: &str, color: crossterm::style::Color) -> Vec<MixedTextContent> {
    inline(text, Some(color)).0
}

fn bullet(text: &str) -> (Vec<MixedTextContent>, Option<String>) {
    let (contents, link) = inline(text, None);
    let mut all = vec![content_bold("• ", theme::ACCENT_TEXT)];
    all.extend(contents);
    (all, link)
}

fn quote(text: &str) -> Vec<MixedTextContent> {
    let (contents, _) = inline(text, Some(theme::BORDER));
    contents
        .iter()
        .map(|piece| piece.clone().italic())
        .collect()
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
                    // Running prose keeps just the link text; the raw URL
                    // would double every link's length in a blog body.
                    contents.push(content_underlined(name, theme::ACCENT_TEXT));
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

/// Wraps styled spans and emits one [`Block::Line`] per row; the first row
/// alone carries the link, so a click opens what the paragraph introduced.
fn push_wrapped(
    blocks: &mut Vec<Block>,
    spans: Vec<MixedTextContent>,
    width: usize,
    indent: usize,
    link: Option<String>,
) {
    for (index, row) in wrap(spans, width, indent).into_iter().enumerate() {
        blocks.push(Block::Line(ContentLine {
            contents: row,
            link: if index == 0 { link.clone() } else { None },
        }));
    }
}

// --- Word wrap -----------------------------------------------------------------

#[derive(Default)]
struct WrapRow {
    spans: Vec<MixedTextContent>,
    /// Columns used so far, including any leading indent.
    used: usize,
}

impl WrapRow {
    /// A continuation row, seeded with `indent` columns of space.
    fn indented(indent: usize) -> Self {
        let mut row = WrapRow {
            spans: Vec::new(),
            used: 0,
        };
        if indent > 0 {
            append_text(&mut row, &" ".repeat(indent), &MixedTextContent::new(""));
        }
        row
    }

    /// Whether no word has landed on the row yet.
    fn fresh(&self) -> bool {
        self.spans
            .iter()
            .all(|span| span.text.chars().all(char::is_whitespace))
    }
}

/// Greedy word wrap over a styled span sequence. Splits spans at word
/// boundaries, carrying each span's color, weight, italic and decoration onto
/// every row it lands on. `indent` columns of leading space on rows after the
/// first. Never returns empty: an empty input yields one empty row.
fn wrap(spans: Vec<MixedTextContent>, width: usize, indent: usize) -> Vec<Vec<MixedTextContent>> {
    let width = width.max(1);
    let indent = indent.min(width - 1);
    let mut rows = vec![WrapRow::default()];
    let mut pending_space = false;
    for span in &spans {
        let mut word = String::new();
        for character in span.text.chars() {
            if character.is_whitespace() {
                if !word.is_empty() {
                    place(&mut rows, &mut pending_space, &word, span, width, indent);
                    word.clear();
                }
                pending_space = true;
            } else {
                word.push(character);
            }
        }
        if !word.is_empty() {
            place(&mut rows, &mut pending_space, &word, span, width, indent);
        }
    }
    rows.into_iter().map(|row| row.spans).collect()
}

/// Appends `text` to the row, merging into the last span when its style
/// matches — this keeps the span count near the source's.
fn append_text(row: &mut WrapRow, text: &str, style: &MixedTextContent) {
    if let Some(last) = row.spans.last_mut() {
        if last.color == style.color
            && last.weight == style.weight
            && last.decoration == style.decoration
            && last.italic == style.italic
            && last.invert == style.invert
        {
            last.text.push_str(text);
            row.used += text.chars().count();
            return;
        }
    }
    let mut piece = style.clone();
    piece.text = text.to_string();
    row.spans.push(piece);
    row.used += text.chars().count();
}

/// Places one word: on the current row if it fits, on a fresh row if that is
/// enough, or hard-split at the row edge when the word alone is wider than a
/// whole row — long URLs appear in these posts.
fn place(
    rows: &mut Vec<WrapRow>,
    pending_space: &mut bool,
    word: &str,
    style: &MixedTextContent,
    width: usize,
    indent: usize,
) {
    let count = word.chars().count();
    {
        let row = rows.last_mut().expect("wrap seeds one row");
        let separator = usize::from(*pending_space && !row.fresh());
        if row.used + separator + count <= width {
            if separator == 1 {
                append_text(row, " ", style);
            }
            append_text(row, word, style);
            *pending_space = false;
            return;
        }
    }
    *pending_space = false;
    if count <= width.saturating_sub(indent) {
        let mut row = WrapRow::indented(indent);
        append_text(&mut row, word, style);
        rows.push(row);
        return;
    }
    let chars: Vec<char> = word.chars().collect();
    let mut start = 0;
    while start < chars.len() {
        let row = rows.last_mut().expect("wrap seeds one row");
        let room = width.saturating_sub(row.used);
        if room == 0 {
            rows.push(WrapRow::indented(indent));
            continue;
        }
        let end = (start + room).min(chars.len());
        let piece: String = chars[start..end].iter().collect();
        append_text(row, &piece, style);
        start = end;
        if start < chars.len() {
            rows.push(WrapRow::indented(indent));
        }
    }
}
