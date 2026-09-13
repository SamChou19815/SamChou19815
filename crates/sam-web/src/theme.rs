//! The developersam.com homepage design language, translated to the terminal
//! palette the site's light mode was built from. Every color is lifted from
//! the site itself (`common.css` + `dev-sam-theme` + tailwind usage in
//! `page.tsx`).

/// A 24-bit truecolor, the only color form the palette uses.
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

    /// The CSS form, e.g. `#2563eb`.
    pub(crate) fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// The accent as text (blue-600) — readable on the light background.
pub(crate) const ACCENT_TEXT: Color = Color::rgb(37, 99, 235);

/// Site body text `#1c1e21`.
pub(crate) const TEXT: Color = Color::rgb(28, 30, 33);
/// gray-600 — secondary text: subheaders, raw URLs, hints.
pub(crate) const MUTED: Color = Color::rgb(75, 85, 99);
/// gray-700 — detail and tagline text on white cards.
pub(crate) const SUBTLE: Color = Color::rgb(55, 65, 81);
/// The shell prompt's `sam@developersam`: the code palette's string green at
/// the same hue, darkened until it clears 4.5:1 on the surface. `STRING`
/// itself only reaches 3.9:1, which is a syntax token on a code block but not
/// a word the first screen is read from.
pub(crate) const PROMPT_USER: Color = Color::rgb(21, 122, 69);
/// gray-500 — chrome drawn as text: tags and quote bars.
pub(crate) const BORDER: Color = Color::rgb(107, 114, 128);

// Prism light tokens from common.css.
pub(crate) const PLAIN: Color = Color::rgb(56, 72, 79);
pub(crate) const KEYWORD: Color = Color::rgb(62, 122, 226);
pub(crate) const PROPERTY: Color = Color::rgb(154, 48, 173);
pub(crate) const NUMBER: Color = Color::rgb(195, 59, 48);
pub(crate) const STRING: Color = Color::rgb(26, 143, 82);
pub(crate) const FUNCTION: Color = Color::rgb(213, 34, 98);
pub(crate) const COMMENT: Color = Color::rgb(100, 100, 100);
