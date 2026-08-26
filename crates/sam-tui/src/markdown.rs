//! The styled line the panes and dialogs are built out of, with one
//! consistent, semantic style system — color encodes structure and meaning,
//! never decoration. Links are carried alongside the line for click handling
//! in the view.

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
