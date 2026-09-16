use crate::theme::Color;

#[derive(Clone, Default)]
pub(crate) struct TextStyle {
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

#[derive(Clone)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) style: TextStyle,
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

    pub(crate) fn linked(mut self, url: impl Into<String>) -> Self {
        self.link = Some(url.into());
        self
    }
}

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

pub(crate) fn link(text: impl Into<String>, color: Color, url: &str) -> Span {
    underlined_colored(text, color).linked(url)
}
