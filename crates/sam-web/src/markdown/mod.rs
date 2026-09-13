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
//! already site-root-relative, and every fenced block a tree-sitter grammar
//! covers arrives with highlighting compiled in — byte ranges into each line,
//! resolved against the palette here once the body is decrypted into text
//! again. The samlang blocks keep the runtime highlighter in
//! [`crate::highlight`], the same system the About tab's program is painted
//! with.

/// How a post body's lines read, fence-aware — shared with build.rs's
/// tree-sitter highlighter, so the blocks it highlights are exactly the
/// blocks the renderer draws.
mod scan;

use crate::posts::{CodeBlock, CodeSpan, Post, SpanColor};
use crate::site_path::SitePath;
use crate::style::{link, Line, Span, TextStyle};
use crate::theme;

use scan::{scan_line, LineKind};

/// One unit of a rendered post. A [`Block::Line`] is a paragraph's worth of
/// styled runs; how many rows it takes is decided where it is drawn.
pub(crate) enum Block {
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

/// Renders a post into blocks.
pub(crate) fn post_blocks(post: &Post) -> Vec<Block> {
    let body = post.body().decrypt();
    let compiled = post.code_blocks();
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut code: Option<(String, Vec<&str>)> = None;
    let mut in_code = false;
    // The compiled highlighting of the next fenced block, in body order.
    let mut next_compiled = compiled.iter();

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
        match scan_line(line, &mut in_code) {
            LineKind::FenceOpen(lang) => {
                flush(&mut blocks, &mut paragraph);
                code = Some((lang.to_string(), Vec::new()));
            }
            LineKind::FenceClose => match code.take() {
                Some((lang, source)) => {
                    blocks.push(code_block(lang, source, next_compiled.next(), true));
                }
                // A closing fence cannot arrive with no block open: the scan
                // only reports one while a fence the scan itself opened holds
                // it. Read it as the prose it would otherwise have been.
                None => paragraph.push(line.trim()),
            },
            LineKind::Code(source) => {
                if let Some((_, lines)) = code.as_mut() {
                    lines.push(source);
                }
            }
            LineKind::Text(line) => {
                let trimmed = line.trim_start();
                if trimmed.starts_with('<') {
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
        }
    }
    flush(&mut blocks, &mut paragraph);
    if let Some((lang, source)) = code {
        blocks.push(code_block(lang, source, next_compiled.next(), false));
    }
    blocks
}

/// Assembles one fenced block from its label, its raw lines, and the
/// highlighting build.rs compiled for it. Samlang blocks have none compiled:
/// the runtime highlighter paints them instead.
fn code_block(
    lang: String,
    source: Vec<&str>,
    compiled: Option<&'static CodeBlock>,
    closed: bool,
) -> Block {
    let lines = if lang.eq_ignore_ascii_case("samlang") {
        crate::highlight::samlang_lines(&source)
    } else {
        let unstyled = compiled.map_or(&[] as _, |block| block.lines);
        source
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let spans = unstyled.get(index).copied().unwrap_or(&[]);
                styled_code_line(line, spans)
            })
            .collect()
    };
    Block::Code {
        lang,
        lines,
        closed,
    }
}

/// One code line as its compiled spans paint it: a styled run per span, plain
/// runs between them, and a single plain run when none was compiled.
fn styled_code_line(line: &str, spans: &'static [CodeSpan]) -> Line {
    let plain = |text: &str| Span::styled(text, TextStyle::new().color(theme::PLAIN));
    if spans.is_empty() {
        return vec![plain(line)];
    }
    let mut styled: Line = Vec::new();
    let mut plain_from = 0;
    for span in spans {
        let start = span.start as usize;
        let end = start + span.len as usize;
        // The spans are compiled against this very line, so their ranges
        // land on char boundaries; degrade to plain rather than panic if one
        // ever does not.
        let Some(painted) = line.get(start..end) else {
            continue;
        };
        if start > plain_from {
            if let Some(run) = line.get(plain_from..start) {
                if !run.is_empty() {
                    styled.push(plain(run));
                }
            }
        }
        styled.push(Span::styled(painted, span_color(span.color)));
        plain_from = end;
    }
    if plain_from < line.len() {
        if let Some(run) = line.get(plain_from..) {
            styled.push(plain(run));
        }
    }
    styled
}

/// The style a compiled span color resolves to, from the same palette the
/// samlang highlighter paints with.
fn span_color(color: SpanColor) -> TextStyle {
    match color {
        SpanColor::Keyword => TextStyle::new().color(theme::KEYWORD),
        SpanColor::String => TextStyle::new().color(theme::STRING),
        SpanColor::Number => TextStyle::new().color(theme::NUMBER),
        SpanColor::Function => TextStyle::new().color(theme::FUNCTION),
        SpanColor::Type => TextStyle::new().color(theme::PROPERTY),
        SpanColor::Comment => TextStyle::new().color(theme::COMMENT).italic(),
    }
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
