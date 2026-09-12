//! Syntax highlighting for the samlang program on the About tab, using the
//! homepage's Prism light token colors.

use crate::crypt::EncryptedString;
use crate::data;
use crate::encrypted_str;
use crate::style::{link, Line, Span, TextStyle};
use crate::theme;

/// The language's keywords, encrypted like the program they color so the binary
/// spells out none of them; matched by decrypting.
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

pub fn doc_comment_lines() -> Vec<Line> {
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

pub fn program_lines() -> Vec<Line> {
    let mut result = Vec::new();
    let mut in_comment = false;
    let program = data::ABOUT_PROGRAM.decrypt();
    for source_line in program.lines() {
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
            match rest.find("*/") {
                Some(end) => {
                    spans.push(styled_italic(&rest[..end + 2], comment_color()));
                    rest = &rest[end + 2..];
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
        } else if rest.starts_with('"') {
            let end = rest[1..].find('"').map_or(rest.len(), |index| index + 2);
            spans.push(styled(&rest[..end], string_color()));
            rest = &rest[end..];
        } else {
            let word_length = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .count();
            if word_length == 0 {
                let first = rest.chars().next().unwrap_or_default();
                spans.push(styled(&rest[..first.len_utf8()], plain_color()));
                rest = &rest[first.len_utf8()..];
            } else {
                let word = &rest[..word_length];
                let is_call = rest[word_length..].trim_start().starts_with('(');
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
                rest = &rest[word_length..];
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
