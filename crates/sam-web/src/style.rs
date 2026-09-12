//! Styled text: the one span type the whole site speaks, from a shell line to
//! a paragraph of a post. Color encodes structure and meaning, never
//! decoration.
//!
//! A span is also as small as a click target gets: a span that carries a `link`
//! is the thing a pointer opens, so a link inside a sentence takes exactly the
//! words it is written on and nothing else.

use crate::theme::Color;

/// The style of a run of text.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct TextStyle {
    /// The text color, or `None` for the page's default foreground.
    pub color: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }
}

/// A run of uniformly styled text, and where it leads if it is a link.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Span {
    pub text: String,
    pub style: TextStyle,
    /// The URL this run opens, for the runs that are links. Wrapping and
    /// merging both carry it, so a link that breaks across two rows is
    /// clickable on both of them and never merges into the prose beside it.
    pub link: Option<String>,
}

impl Span {
    pub fn new(text: impl Into<String>) -> Self {
        Span {
            text: text.into(),
            style: TextStyle::default(),
            link: None,
        }
    }

    pub fn styled(text: impl Into<String>, style: TextStyle) -> Self {
        Span {
            text: text.into(),
            style,
            link: None,
        }
    }

    /// The same run, opening `url` when it is clicked.
    pub fn linked(mut self, url: impl Into<String>) -> Self {
        self.link = Some(url.into());
        self
    }
}

/// A line of styled text: what every view, and the shell's scrollback, is made
/// of.
pub type Line = Vec<Span>;

pub fn colored(text: impl Into<String>, color: Color) -> Span {
    Span::styled(text, TextStyle::new().color(color))
}

pub fn bold_colored(text: impl Into<String>, color: Color) -> Span {
    Span::styled(text, TextStyle::new().color(color).bold())
}

pub fn underlined_colored(text: impl Into<String>, color: Color) -> Span {
    Span::styled(text, TextStyle::new().color(color).underline())
}

/// A link, drawn the way the site draws one and carrying where it goes.
pub fn link(text: impl Into<String>, color: Color, url: &str) -> Span {
    underlined_colored(text, color).linked(url)
}
