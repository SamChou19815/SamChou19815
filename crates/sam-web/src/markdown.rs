//! A small markdown subset rendered into styled spans, with one consistent,
//! semantic style system — color encodes structure and meaning, never
//! decoration. A link is carried by the run of text it was written on
//! ([`Span::link`]), so it is clicked where it is read.
//!
//! Text is never wrapped or cut here: a block is styled runs, and where its
//! lines break is the browser's call, made against the real width the block
//! lands at.
//!
//! Blog bodies arrive pre-normalized by `build.rs`: every image URL in one is
//! already site-root-relative, so [`post_blocks`] can hand each one to the
//! view as it finds it.

use crate::site_path::SitePath;
use crate::style::{link, Line, Span, TextStyle};
use crate::theme;

/// One unit of a rendered post. A [`Block::Line`] is a paragraph's worth of
/// styled runs; how many rows it takes is decided where it is drawn.
pub enum Block {
    Line(Line),
    /// A list item: `marker` hangs in the two characters the text indents by,
    /// which the view draws for it.
    Bullet(Line),
    /// A block quote: the line is already italic and in the quote color; the
    /// view draws the bar it hangs from.
    Quote(Line),
    /// A fenced code block: its label, its lines, and whether a closing fence
    /// ever came — the view draws a rule for each fence, and a block left
    /// open at the end of the body only gets the one it opened with.
    Code {
        lang: String,
        lines: Vec<Line>,
        closed: bool,
    },
    /// `url` is owned rather than borrowed from the body: a body is decrypted
    /// on demand ([`crate::crypt`]), so there is no `'static` text to point at.
    Image {
        url: SitePath,
    },
}

/// Renders a post body into blocks.
pub fn post_blocks(body: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut code: Option<(String, Vec<Line>)> = None;

    fn flush(blocks: &mut Vec<Block>, paragraph: &mut Vec<&str>) {
        if paragraph.is_empty() {
            return;
        }
        let text = paragraph.join(" ");
        paragraph.clear();
        blocks.push(Block::Line(inline(&text, None)));
    }

    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            flush(&mut blocks, &mut paragraph);
            match code.take() {
                None => {
                    let lang = trimmed.trim_start_matches('`').trim().to_string();
                    code = Some((lang, Vec::new()));
                }
                Some((lang, lines)) => blocks.push(Block::Code {
                    lang,
                    lines,
                    closed: true,
                }),
            }
        } else if let Some((_lang, code_lines)) = code.as_mut() {
            code_lines.push(vec![Span::styled(
                line,
                TextStyle::new().color(theme::PLAIN),
            )]);
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
            if let Some(url) = src.map(SitePath::new) {
                blocks.push(Block::Image { url });
            }
        } else if let Some((url, rest)) = leading_image(trimmed) {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block::Image {
                url: SitePath::new(url),
            });
            // A caption trailing the image starts a new paragraph.
            let rest = rest.trim();
            if !rest.is_empty() {
                paragraph.push(rest);
            }
        } else if line.trim().is_empty() {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block::Line(Line::new()));
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
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block::Line(heading(text, color)));
        } else if let Some(text) = trimmed
            .strip_prefix("- ")
            .or_else(|| numbered_item(trimmed))
        {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block::Bullet(bullet(text)));
        } else if let Some(text) = trimmed.strip_prefix("> ") {
            flush(&mut blocks, &mut paragraph);
            blocks.push(Block::Quote(quote(text)));
        } else {
            paragraph.push(trimmed);
        }
    }
    flush(&mut blocks, &mut paragraph);
    if let Some((lang, lines)) = code {
        blocks.push(Block::Code {
            lang,
            lines,
            closed: false,
        });
    }
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

/// An `![alt](url)` reference leading a line: its url and trailing text.
fn leading_image(line: &str) -> Option<(&str, &str)> {
    let after_marker = line.strip_prefix("![")?;
    let close = after_marker.find("](")?;
    let after_url_open = &after_marker[close + 2..];
    let end = after_url_open.find(')')?;
    Some((&after_url_open[..end], &after_url_open[end + 1..]))
}

/// The URL in an HTML fragment's `src="…"`, if it carries one.
fn html_src(html: &str) -> Option<&str> {
    let start = html.find("src=\"")? + 5;
    let rest = &html[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn heading(text: &str, color: theme::Color) -> Line {
    inline(text, Some(color))
}

fn bullet(text: &str) -> Line {
    let mut all = vec![Span::styled(
        "• ",
        TextStyle::new().color(theme::ACCENT_TEXT).bold(),
    )];
    all.extend(inline(text, None));
    all
}

fn quote(text: &str) -> Line {
    inline(text, Some(theme::BORDER))
        .into_iter()
        .map(|mut piece| {
            piece.style.italic = true;
            piece
        })
        .collect()
}

/// Renders inline markdown: `**bold**`, `*italic*`, `` `code` `` and links,
/// each link carried by the run of text it was written on.
fn inline(text: &str, base: Option<theme::Color>) -> Line {
    let base_color = base.unwrap_or(theme::TEXT);
    let mut contents: Line = Vec::new();
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
                    contents.push(Span::styled(
                        tail[..end].to_string(),
                        TextStyle::new().color(theme::ACCENT_TEXT),
                    ));
                    rest = &tail[end + 1..];
                }
                None => {
                    contents.push(Span::styled(
                        tail.to_string(),
                        TextStyle::new().color(theme::ACCENT_TEXT),
                    ));
                    rest = "";
                }
            }
        } else if let Some(tail) = rest.strip_prefix('[') {
            if let Some(close) = tail.find("](") {
                if let Some(end) = tail[close + 2..].find(')').map(|index| close + 2 + index) {
                    let name = &tail[..close];
                    let url = &tail[close + 2..end];
                    // Running prose keeps just the link text; the raw URL
                    // would double every link's length in a blog body. The
                    // words themselves are the click target.
                    contents.push(link(name, theme::ACCENT_TEXT, url));
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
    contents
}

fn styled(text: String, color: theme::Color, bold: bool, italic: bool) -> Span {
    let mut style = TextStyle::new().color(color);
    if bold {
        style.bold = true;
    }
    if italic {
        style.italic = true;
    }
    Span::styled(text, style)
}
