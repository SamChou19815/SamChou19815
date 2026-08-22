//! Syntax highlighting for the samlang program on the About tab, using the
//! homepage's Prism light token colors.

use crate::data;
use crate::markdown::ContentLine;
use crate::theme;
use iocraft::components::MixedTextContent;
use iocraft::prelude::TextDecoration;

const KEYWORDS: &[&str] = &["import", "from", "class", "function", "let", "val"];

// Token colors mirror the homepage's Prism dark theme (see `theme.rs`).
fn keyword_color() -> crossterm::style::Color {
    theme::KEYWORD
}

/// Class and type names — `.token.property` on the homepage.
fn type_color() -> crossterm::style::Color {
    theme::PROPERTY
}

fn string_color() -> crossterm::style::Color {
    theme::STRING
}

fn number_color() -> crossterm::style::Color {
    theme::NUMBER
}

/// Identifiers directly followed by `(`: function and method calls.
fn function_color() -> crossterm::style::Color {
    theme::FUNCTION
}

/// Comments are italic on the homepage.
fn comment_color() -> crossterm::style::Color {
    theme::COMMENT
}

/// Plain code text.
fn plain_color() -> crossterm::style::Color {
    theme::PLAIN
}

fn styled(text: &str, color: crossterm::style::Color) -> MixedTextContent {
    MixedTextContent::new(text).color(color)
}

fn styled_italic(text: &str, color: crossterm::style::Color) -> MixedTextContent {
    styled(text, color).italic()
}

/// Renders the license header as comment-styled code lines. Its `@` tags carry
/// their URL, so the reader can open one the way the homepage's docblock lets
/// them click it — underlined in the comment color, exactly as it is there.
pub fn doc_comment_lines() -> Vec<ContentLine> {
    let plain = |text: &str| ContentLine {
        contents: vec![styled(text, comment_color())],
        link: None,
    };
    let mut lines = vec![plain("/**"), plain(&format!(" * {}", data::COPYRIGHT))];
    for link in data::ABOUT_DOC_LINKS {
        lines.push(ContentLine {
            contents: vec![
                styled(&format!(" * @{} ", link.name), comment_color()),
                styled(link.url, comment_color()).decoration(TextDecoration::Underline),
            ],
            link: Some(link.url.to_string()),
        });
    }
    lines.push(plain(" */"));
    lines
}

/// Renders the about program as one styled line per source line.
pub fn program_lines() -> Vec<Vec<MixedTextContent>> {
    let mut result = Vec::new();
    let mut in_comment = false;
    for source_line in data::ABOUT_PROGRAM.lines() {
        let (spans, comment_continues) = highlight_line(source_line, in_comment);
        in_comment = comment_continues;
        result.push(spans);
    }
    result
}

fn highlight_line(source: &str, mut in_comment: bool) -> (Vec<MixedTextContent>, bool) {
    let mut spans: Vec<MixedTextContent> = Vec::new();
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
                let color = if KEYWORDS.contains(&word) {
                    keyword_color()
                } else if word.chars().all(|c| c.is_ascii_digit()) {
                    number_color()
                } else if is_call {
                    function_color()
                } else if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                    type_color()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(spans: &[MixedTextContent]) -> String {
        spans.iter().map(|span| span.text.as_str()).collect()
    }

    #[test]
    fn highlights_the_program() {
        let lines = program_lines();
        assert_eq!(lines.len(), data::ABOUT_PROGRAM.lines().count());
        let text = plain(&highlight_line(r#"let github = "SamChou19815";"#, false).0);
        assert_eq!(text, r#"let github = "SamChou19815";"#);
    }

    #[test]
    fn numbers_and_calls_get_styled() {
        let (spans, _) = highlight_line("Developer.init(github, 42)", false);
        assert!(spans.iter().any(|s| s.color == Some(function_color())));
        assert!(spans.iter().any(|s| s.color == Some(number_color())));
        // A type followed by '.' stays a type, not a call.
        let (spans, _) = highlight_line("Developer.sam()", false);
        assert!(spans.iter().any(|s| s.color == Some(type_color())));
        assert!(spans.iter().any(|s| s.color == Some(function_color())));
    }
}
