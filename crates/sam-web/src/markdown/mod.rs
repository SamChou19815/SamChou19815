//! Minimal markdown subset to styled spans. No wrapping; the browser handles that.

mod scan;

use crate::posts::{CodeBlock, CodeSpan, Post, SpanColor};
use crate::site_path::SitePath;
use crate::style::{link, Line, Span, TextStyle};
use crate::theme;

use scan::{scan_line, LineKind};

pub(crate) enum Block {
    Line(Line),
    Heading {
        level: u8,
        line: Line,
    },
    /// `marker` is `•` or `N.`.
    Bullet {
        marker: String,
        line: Line,
    },
    Quote(Line),
    /// `closed` is false when the body ends inside the block.
    Code {
        lang: String,
        lines: Vec<Line>,
        closed: bool,
    },
    Image {
        url: SitePath,
    },
}

pub(crate) fn post_blocks(post: &Post) -> Vec<Block> {
    let body = post.body().decrypt();
    let compiled = post.code_blocks();
    let mut blocks: Vec<Block> = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    // Open list item: marker and its lines so far. Continuation lines fold in.
    let mut list_item: Option<(String, Vec<&str>)> = None;
    let mut code: Option<(String, Vec<&str>)> = None;
    let mut in_code = false;
    let mut next_compiled = compiled.iter();

    fn flush(
        blocks: &mut Vec<Block>,
        paragraph: &mut Vec<&str>,
        item: &mut Option<(String, Vec<&str>)>,
    ) {
        if !paragraph.is_empty() {
            let text = paragraph.join(" ");
            paragraph.clear();
            blocks.push(Block::Line(inline(&text, None)));
        }
        if let Some((marker, lines)) = item.take() {
            blocks.push(bullet(&marker, &lines));
        }
    }

    let mut lines = body.lines();
    while let Some(line) = lines.next() {
        match scan_line(line, &mut in_code) {
            LineKind::FenceOpen(lang) => {
                flush(&mut blocks, &mut paragraph, &mut list_item);
                code = Some((lang.to_string(), Vec::new()));
            }
            LineKind::FenceClose => match code.take() {
                Some((lang, source)) => {
                    blocks.push(code_block(lang, source, next_compiled.next(), true));
                }
                // Unreachable: scan only reports a close while a block is open.
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
                    // Raw HTML (only `<br />` and `<img>` in practice): skip it, keep the img src.
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
                    flush(&mut blocks, &mut paragraph, &mut list_item);
                    blocks.push(Block::Image {
                        url: SitePath::new(url),
                    });
                    let rest = rest.trim();
                    if !rest.is_empty() {
                        paragraph.push(rest);
                    }
                } else if line.trim().is_empty() {
                    flush(&mut blocks, &mut paragraph, &mut list_item);
                    blocks.push(Block::Line(Line::new()));
                } else if let Some((text, color, level)) = trimmed
                    .strip_prefix("### ")
                    .map(|text| (text, theme::TEXT, 3u8))
                    .or_else(|| {
                        trimmed
                            .strip_prefix("## ")
                            .map(|text| (text, theme::ACCENT_TEXT, 2))
                    })
                    .or_else(|| {
                        trimmed
                            .strip_prefix("# ")
                            .map(|text| (text, theme::ACCENT_TEXT, 1))
                    })
                {
                    flush(&mut blocks, &mut paragraph, &mut list_item);
                    blocks.push(Block::Heading {
                        level,
                        line: heading(text, color),
                    });
                } else if let Some((marker, text)) = trimmed
                    .strip_prefix("- ")
                    .map(|text| ("•", text))
                    .or_else(|| numbered_item(trimmed))
                {
                    flush(&mut blocks, &mut paragraph, &mut list_item);
                    list_item = Some((marker.to_string(), vec![text]));
                } else if let Some(text) = trimmed.strip_prefix("> ") {
                    flush(&mut blocks, &mut paragraph, &mut list_item);
                    blocks.push(Block::Quote(quote(text)));
                } else if let Some((_, lines)) = list_item.as_mut() {
                    lines.push(trimmed);
                } else {
                    paragraph.push(trimmed);
                }
            }
        }
    }
    flush(&mut blocks, &mut paragraph, &mut list_item);
    if let Some((lang, source)) = code {
        blocks.push(code_block(lang, source, next_compiled.next(), false));
    }
    blocks
}

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

/// `"3. foo"` -> `("3.", "foo")`
fn numbered_item(line: &str) -> Option<(&str, &str)> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let item = line.get(digits..)?.strip_prefix(". ")?;
    Some((line.get(..=digits)?, item))
}

/// `"![alt](url) rest"` -> `("url", " rest")`
fn leading_image(line: &str) -> Option<(&str, &str)> {
    let (_, after_url_open) = line.strip_prefix("![")?.split_once("](")?;
    after_url_open.split_once(')')
}

fn html_src(html: &str) -> Option<&str> {
    let (_, rest) = html.split_once("src=\"")?;
    rest.split_once('"').map(|(src, _)| src)
}

fn heading(text: &str, color: theme::Color) -> Line {
    inline(text, Some(color))
}

fn bullet(marker: &str, lines: &[&str]) -> Block {
    Block::Bullet {
        marker: marker.to_string(),
        line: inline(&lines.join(" "), None),
    }
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

/// `**bold**`, `*italic*`, `_italic_`, `` `code` ``, `[text](url)`. `_` only toggles italic at a
/// word boundary so snake_case and URLs survive.
fn inline(text: &str, base: Option<theme::Color>) -> Line {
    let base_color = base.unwrap_or(theme::TEXT);
    let mut contents: Line = Vec::new();
    let mut rest = text;
    let mut bold = false;
    let mut italic = false;
    // Last char emitted, for the word boundary check.
    let mut last: Option<char> = None;
    while !rest.is_empty() {
        let cut = rest.find(['*', '`', '[', '_']).unwrap_or(rest.len());
        let (run, marked) = rest.split_at_checked(cut).unwrap_or((rest, ""));
        if !run.is_empty() {
            last = run.chars().next_back();
            contents.push(styled(run.to_string(), base_color, bold, italic));
        }
        rest = marked;
        if rest.is_empty() {
            break;
        }
        if let Some(tail) = rest.strip_prefix("**") {
            bold = !bold;
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix('`') {
            match tail.split_once('`') {
                Some((code, after)) => {
                    contents.push(Span::styled(
                        code.to_string(),
                        TextStyle::new().color(theme::ACCENT_TEXT),
                    ));
                    rest = after;
                }
                None => {
                    contents.push(Span::styled(
                        tail.to_string(),
                        TextStyle::new().color(theme::ACCENT_TEXT),
                    ));
                    rest = "";
                }
            }
            last = None;
        } else if let Some(tail) = rest.strip_prefix('[') {
            if let Some((name, after_name)) = tail.split_once("](") {
                if let Some((url, after_url)) = after_name.split_once(')') {
                    contents.push(link(name, theme::ACCENT_TEXT, url));
                    rest = after_url;
                    last = None;
                    continue;
                }
            }
            contents.push(styled("[".to_string(), base_color, bold, italic));
            last = Some('[');
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix('_') {
            let next = tail.chars().next();
            let at_word_start = last.is_none_or(|c| !c.is_alphanumeric());
            let at_word_end = next.is_none_or(|c| !c.is_alphanumeric());
            if at_word_start && !italic && next.is_some_and(|c| !c.is_whitespace())
                || at_word_end && italic && last.is_some_and(|c| !c.is_whitespace())
            {
                italic = !italic;
            } else {
                contents.push(styled("_".to_string(), base_color, bold, italic));
                last = Some('_');
            }
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix('*') {
            italic = !italic;
            rest = tail;
        } else {
            let mut chars = rest.chars();
            let first = chars.next().unwrap_or_default();
            contents.push(styled(first.to_string(), base_color, bold, italic));
            last = Some(first);
            rest = chars.as_str();
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
