//! Styled text: the one span type the whole site speaks, from a shell line to
//! a paragraph of a post. Color encodes structure and meaning, never
//! decoration.
//!
//! A span is also as small as a click target gets: a span that carries a `link`
//! is the thing a pointer opens, so a link inside a sentence takes exactly the
//! words it is written on and nothing else.

use crate::theme::Color;

/// The style of a run of text.
#[derive(Clone, Default)]
pub(crate) struct TextStyle {
    /// The text color, or `None` for the page's default foreground.
    pub(crate) color: Option<Color>,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
}

impl TextStyle {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub(crate) fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub(crate) fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub(crate) fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

/// A run of uniformly styled text, and where it leads if it is a link.
#[derive(Clone)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) style: TextStyle,
    /// The URL this run opens, for the runs that are links. Wrapping and
    /// merging both carry it, so a link that breaks across two rows is
    /// clickable on both of them and never merges into the prose beside it.
    pub(crate) link: Option<String>,
}

impl Span {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Span {
            text: text.into(),
            style: TextStyle::default(),
            link: None,
        }
    }

    pub(crate) fn styled(text: impl Into<String>, style: TextStyle) -> Self {
        Span {
            text: text.into(),
            style,
            link: None,
        }
    }

    /// The same run, opening `url` when it is clicked.
    pub(crate) fn linked(mut self, url: impl Into<String>) -> Self {
        self.link = Some(url.into());
        self
    }
}

/// A line of styled text: what every view, and the shell's scrollback, is made
/// of.
pub(crate) type Line = Vec<Span>;

pub(crate) fn colored(text: impl Into<String>, color: Color) -> Span {
    Span::styled(text, TextStyle::new().color(color))
}

pub(crate) fn bold_colored(text: impl Into<String>, color: Color) -> Span {
    Span::styled(text, TextStyle::new().color(color).bold())
}

fn underlined_colored(text: impl Into<String>, color: Color) -> Span {
    Span::styled(text, TextStyle::new().color(color).underline())
}

/// A link, drawn the way the site draws one and carrying where it goes.
pub(crate) fn link(text: impl Into<String>, color: Color, url: &str) -> Span {
    underlined_colored(text, color).linked(url)
}
