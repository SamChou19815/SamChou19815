//! Syntax highlighting for the samlang program on the About tab.

use crate::theme;
use ratatui_core::style::{Modifier, Style};
use ratatui_core::text::Span;

const KEYWORDS: &[&str] = &["import", "from", "class", "function", "let", "val"];

// Token colors mirror the homepage's Prism dark theme (see `theme.rs`).

fn keyword_style() -> Style {
    Style::new().fg(theme::KEYWORD)
}

/// Class and type names — `.token.property` on the homepage.
fn type_style() -> Style {
    Style::new().fg(theme::PROPERTY)
}

fn string_style() -> Style {
    Style::new().fg(theme::STRING)
}

fn number_style() -> Style {
    Style::new().fg(theme::NUMBER)
}

/// Identifiers directly followed by `(`: function and method calls.
fn function_style() -> Style {
    Style::new().fg(theme::FUNCTION)
}

/// Comments are italic on the homepage.
fn comment_style() -> Style {
    Style::new()
        .fg(theme::COMMENT)
        .add_modifier(Modifier::ITALIC)
}

/// Plain code text.
fn plain_style() -> Style {
    Style::new().fg(theme::PLAIN)
}

/// Renders the license header as comment-styled code lines.
/// A clickable URL embedded in a doc-comment line: its character offset
/// within the line and its target.
pub struct DocLink {
    pub offset: usize,
    pub url: String,
}

/// One doc-comment line: styled spans plus an optional embedded link.
pub struct DocLine {
    pub spans: Vec<Span<'static>>,
    pub link: Option<DocLink>,
}

/// The license header as comment-styled code lines. The `@doc` URLs render
/// like the homepage's doc links: comment-colored but underlined, and their
/// positions are reported so the view can make them clickable.
pub fn doc_comment_lines() -> Vec<DocLine> {
    let plain = |text: &str| DocLine {
        spans: vec![styled(text, comment_style())],
        link: None,
    };
    let mut lines = vec![
        plain("/**"),
        plain(&format!(" * {}", crate::data::COPYRIGHT)),
    ];
    for link in crate::data::ABOUT_DOC_LINKS {
        let prefix = format!(" * @{} ", link.name);
        lines.push(DocLine {
            spans: vec![
                styled(&prefix, comment_style()),
                styled(link.url, comment_style().add_modifier(Modifier::UNDERLINED)),
            ],
            link: Some(DocLink {
                offset: prefix.chars().count(),
                url: link.url.to_string(),
            }),
        });
    }
    lines.push(plain(" */"));
    lines
}

/// Renders the about program as one styled line per source line.
pub fn program_lines() -> Vec<Vec<Span<'static>>> {
    let mut result = Vec::new();
    let mut in_comment = false;
    for source_line in crate::data::ABOUT_PROGRAM.lines() {
        let (spans, comment_continues) = highlight_line(source_line, in_comment);
        in_comment = comment_continues;
        result.push(spans);
    }
    result
}

fn styled(text: &str, style: Style) -> Span<'static> {
    Span::styled(text.to_string(), style)
}

fn highlight_line(source: &str, mut in_comment: bool) -> (Vec<Span<'static>>, bool) {
    let mut spans = Vec::new();
    let mut rest = source;
    while !rest.is_empty() {
        if in_comment {
            match rest.find("*/") {
                Some(end) => {
                    spans.push(styled(&rest[..end + 2], comment_style()));
                    rest = &rest[end + 2..];
                    in_comment = false;
                }
                None => {
                    spans.push(styled(rest, comment_style()));
                    rest = "";
                }
            }
        } else if rest.starts_with("/*") {
            in_comment = true;
        } else if rest.starts_with("//") {
            spans.push(styled(rest, comment_style()));
            rest = "";
        } else if rest.starts_with('"') {
            let end = rest[1..].find('"').map_or(rest.len(), |index| index + 2);
            spans.push(styled(&rest[..end], string_style()));
            rest = &rest[end..];
        } else {
            let word_length = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .count();
            if word_length == 0 {
                let first = rest.chars().next().unwrap_or_default();
                spans.push(styled(&rest[..first.len_utf8()], plain_style()));
                rest = &rest[first.len_utf8()..];
            } else {
                let word = &rest[..word_length];
                let is_call = rest[word_length..].trim_start().starts_with('(');
                let style = if KEYWORDS.contains(&word) {
                    keyword_style()
                } else if word.chars().all(|c| c.is_ascii_digit()) {
                    number_style()
                } else if is_call {
                    function_style()
                } else if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                    type_style()
                } else {
                    plain_style()
                };
                spans.push(styled(word, style));
                rest = &rest[word_length..];
            }
        }
    }
    (spans, in_comment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_the_program() {
        let lines = program_lines();
        assert_eq!(lines.len(), crate::data::ABOUT_PROGRAM.lines().count());
        let source = std::concat!(
            "class Main {\n",
            "  function main(): Developer = Developer.sam()\n",
            "}"
        );
        let mut in_comment = false;
        for source_line in source.lines() {
            let _ = highlight_line(source_line, in_comment);
            in_comment = false;
        }
        let (spans, _) = highlight_line(r#"let github = "SamChou19815";"#, false);
        let text: String = spans.iter().flat_map(|s| s.content.chars()).collect();
        assert_eq!(text, r#"let github = "SamChou19815";"#);
    }

    #[test]
    fn strings_and_keywords_get_styled() {
        let (spans, _) = highlight_line("let x = \"hi\";", false);
        assert!(spans.iter().any(|s| s.style == keyword_style()));
        assert!(spans.iter().any(|s| s.style == string_style()));
    }

    #[test]
    fn numbers_and_calls_get_styled() {
        let (spans, _) = highlight_line("Developer.init(github, 42)", false);
        assert!(spans.iter().any(|s| s.style == function_style()));
        assert!(spans.iter().any(|s| s.style == number_style()));
        // A type followed by '.' stays a type, not a call.
        let (spans, _) = highlight_line("Developer.sam()", false);
        assert!(spans.iter().any(|s| s.style == type_style()));
        assert!(spans.iter().any(|s| s.style == function_style()));
    }
}
