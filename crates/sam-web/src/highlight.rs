//! samlang syntax highlighting. Other languages are highlighted by tree-sitter in build.rs.

use crate::crypt::{encrypted_str, EncryptedString};
use crate::data;
use crate::style::{link, Line, Span, TextStyle};
use crate::theme;

const KEYWORDS: [EncryptedString; 6] = [
    encrypted_str!("import"),
    encrypted_str!("from"),
    encrypted_str!("class"),
    encrypted_str!("function"),
    encrypted_str!("let"),
    encrypted_str!("val"),
];

fn styled(text: &str, color: theme::Color) -> Span {
    Span::styled(text, TextStyle::new().color(color))
}

fn styled_italic(text: &str, color: theme::Color) -> Span {
    Span::styled(text, TextStyle::new().color(color).italic())
}

pub(crate) fn doc_comment_lines() -> Vec<Line> {
    let plain = |text: &str| vec![styled(text, comment_color())];
    let mut lines = vec![plain("/**"), plain(&format!(" * {}", data::COPYRIGHT))];
    for entry in data::ABOUT_DOC_LINKS {
        let url = entry.url.decrypt();
        lines.push(vec![
            styled(&format!(" * @{} ", entry.name), comment_color()),
            link(url.clone(), comment_color(), &url),
        ]);
    }
    lines.push(plain(" */"));
    lines
}

pub(crate) fn program_lines() -> Vec<Line> {
    let program = data::ABOUT_PROGRAM.decrypt();
    let lines: Vec<&str> = program.lines().collect();
    samlang_lines(&lines)
}

pub(crate) fn samlang_lines(source_lines: &[&str]) -> Vec<Line> {
    let mut result = Vec::new();
    let mut in_comment = false;
    for source_line in source_lines {
        let (spans, comment_continues) = highlight_line(source_line, in_comment);
        in_comment = comment_continues;
        result.push(spans);
    }
    result
}

fn highlight_line(source: &str, mut in_comment: bool) -> (Vec<Span>, bool) {
    let mut spans: Vec<Span> = Vec::new();
    let mut rest = source;
    while !rest.is_empty() {
        if in_comment {
            match rest.split_once("*/") {
                Some((comment, after)) => {
                    spans.push(styled_italic(&format!("{comment}*/"), comment_color()));
                    rest = after;
                    in_comment = false;
                }
                None => {
                    spans.push(styled_italic(rest, comment_color()));
                    rest = "";
                }
            }
        } else if rest.starts_with("/*") {
            in_comment = true;
        } else if rest.starts_with("//") {
            spans.push(styled_italic(rest, comment_color()));
            rest = "";
        } else if let Some(open) = rest.strip_prefix('"') {
            let (literal, after) = match open.split_once('"') {
                Some((inside, after)) => (format!("\"{inside}\""), after),
                None => (rest.to_string(), ""),
            };
            spans.push(styled(&literal, string_color()));
            rest = after;
        } else {
            let word_length = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .count();
            if word_length == 0 {
                let mut chars = rest.chars();
                let first = chars.next().unwrap_or_default();
                spans.push(styled(&first.to_string(), plain_color()));
                rest = chars.as_str();
            } else {
                // ASCII, so chars are bytes.
                let (word, after) = rest.split_at_checked(word_length).unwrap_or((rest, ""));
                let is_call = after.trim_start().starts_with('(');
                let color = if KEYWORDS.iter().any(|keyword| keyword.decrypt() == word) {
                    keyword_color()
                } else if word.chars().all(|c| c.is_ascii_digit()) {
                    number_color()
                } else if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                    type_color()
                } else if is_call {
                    function_color()
                } else {
                    plain_color()
                };
                spans.push(styled(word, color));
                rest = after;
            }
        }
    }
    (spans, in_comment)
}

fn keyword_color() -> theme::Color {
    theme::KEYWORD
}

fn type_color() -> theme::Color {
    theme::PROPERTY
}

fn string_color() -> theme::Color {
    theme::STRING
}

fn number_color() -> theme::Color {
    theme::NUMBER
}

fn function_color() -> theme::Color {
    theme::FUNCTION
}

fn comment_color() -> theme::Color {
    theme::COMMENT
}

fn plain_color() -> theme::Color {
    theme::PLAIN
}
