//! The developersam.com homepage design language, translated to the terminal
//! palette the site's light mode was built from. Every color is lifted from
//! the site itself (`common.css` + `dev-sam-theme` + tailwind usage in
//! `page.tsx`).

/// A 24-bit truecolor, the only color form the palette uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// The CSS form, e.g. `#2563eb`.
    pub fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// blue-500 — the site's accent (timeline line, dots, buttons, links).
pub const ACCENT: Color = Color::rgb(59, 130, 246);
/// The accent as text (blue-600) — readable on the light background.
pub const ACCENT_TEXT: Color = Color::rgb(37, 99, 235);
/// Selected-row tint: blue-100, the site's `bg-blue-500 bg-opacity-10` hover.
pub const SELECT_BG: Color = Color::rgb(219, 234, 254);
/// Selected-row text: blue-900 on the blue-100 tint (≈8:1).
pub const SELECT_FG: Color = Color::rgb(30, 58, 138);
/// Every surface the app paints — the screen behind it, the pane, a card — is
/// the homepage's code block, `#f7f7f7`. It is the site's body colour and the
/// terminal's own background, so the app and the page it is served on are one
/// colour, edge to edge.
pub const SURFACE: Color = Color::rgb(247, 247, 247);

/// Site body text `#1c1e21`.
pub const TEXT: Color = Color::rgb(28, 30, 33);
/// gray-600 — secondary text: subheaders, raw URLs, hints.
pub const MUTED: Color = Color::rgb(75, 85, 99);
/// gray-700 — detail and tagline text on white cards.
pub const SUBTLE: Color = Color::rgb(55, 65, 81);
/// The shell prompt's `sam@developersam`: the code palette's string green at
/// the same hue, darkened until it clears 4.5:1 on the surface. `STRING`
/// itself only reaches 3.9:1, which is a syntax token on a code block but not
/// a word the first screen is read from.
pub const PROMPT_USER: Color = Color::rgb(21, 122, 69);
/// gray-500 — chrome drawn as text: tags and quote bars.
pub const BORDER: Color = Color::rgb(107, 114, 128);
/// gray-300 — the boxes' borders, the site's own `border-gray-300` on cards.
/// A cell is as thin as a box line gets, so the hairline look is all color:
/// this sits far enough back that the card reads as an edge, not a frame.
pub const BORDER_SUBTLE: Color = Color::rgb(209, 213, 219);
/// amber-600 — the completion star.
pub const STAR: Color = Color::rgb(217, 119, 6);

// Prism light tokens from common.css.
pub const PLAIN: Color = Color::rgb(56, 72, 79);
pub const KEYWORD: Color = Color::rgb(62, 122, 226);
pub const PROPERTY: Color = Color::rgb(154, 48, 173);
pub const NUMBER: Color = Color::rgb(195, 59, 48);
pub const STRING: Color = Color::rgb(26, 143, 82);
pub const FUNCTION: Color = Color::rgb(213, 34, 98);
pub const COMMENT: Color = Color::rgb(100, 100, 100);
