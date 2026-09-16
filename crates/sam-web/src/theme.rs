#[derive(Clone, Copy)]
pub(crate) struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub(crate) const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    pub(crate) fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// blue-600
pub(crate) const ACCENT_TEXT: Color = Color::rgb(37, 99, 235);

/// Site body text.
pub(crate) const TEXT: Color = Color::rgb(28, 30, 33);
/// gray-600
pub(crate) const MUTED: Color = Color::rgb(75, 85, 99);
/// gray-700
pub(crate) const SUBTLE: Color = Color::rgb(55, 65, 81);
/// `STRING` darkened to pass 4.5:1 contrast.
pub(crate) const PROMPT_USER: Color = Color::rgb(21, 122, 69);
/// gray-500
pub(crate) const BORDER: Color = Color::rgb(107, 114, 128);

// Prism light tokens from common.css.
pub(crate) const PLAIN: Color = Color::rgb(56, 72, 79);
pub(crate) const KEYWORD: Color = Color::rgb(62, 122, 226);
pub(crate) const PROPERTY: Color = Color::rgb(154, 48, 173);
pub(crate) const NUMBER: Color = Color::rgb(195, 59, 48);
pub(crate) const STRING: Color = Color::rgb(26, 143, 82);
pub(crate) const FUNCTION: Color = Color::rgb(213, 34, 98);
pub(crate) const COMMENT: Color = Color::rgb(100, 100, 100);
