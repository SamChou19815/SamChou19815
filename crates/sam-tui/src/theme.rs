//! The developersam.com homepage design language, translated to the TUI
//! (`crossterm` colors). Every color is lifted from the site itself
//! (`common.css` + `dev-sam-theme` + tailwind usage in `page.tsx`), in the
//! site's light mode.

use crossterm::style::Color;

/// blue-500 — the site's accent (timeline line, dots, buttons, links).
pub const ACCENT: Color = Color::Rgb {
    r: 59,
    g: 130,
    b: 246,
};
/// The accent as text (blue-600) — readable on the light background.
pub const ACCENT_TEXT: Color = Color::Rgb {
    r: 37,
    g: 99,
    b: 235,
};
/// Selected-row tint: blue-100, the site's `bg-blue-500 bg-opacity-10` hover.
pub const SELECT_BG: Color = Color::Rgb {
    r: 219,
    g: 234,
    b: 254,
};
/// Selected-row text: blue-900 on the blue-100 tint (≈8:1).
pub const SELECT_FG: Color = Color::Rgb {
    r: 30,
    g: 58,
    b: 138,
};
/// Card surface: white, like the homepage cards on the `#f7f7f7` body.
pub const CARD_BG: Color = Color::Rgb {
    r: 255,
    g: 255,
    b: 255,
};
/// Code block background `#f7f7f7`, as on the homepage.
pub const CODE_BG: Color = Color::Rgb {
    r: 247,
    g: 247,
    b: 247,
};

/// Site body text `#1c1e21`.
pub const TEXT: Color = Color::Rgb {
    r: 28,
    g: 30,
    b: 33,
};
/// gray-600 — secondary text: subheaders, raw URLs, hints.
pub const MUTED: Color = Color::Rgb {
    r: 75,
    g: 85,
    b: 99,
};
/// gray-700 — detail and tagline text on white cards.
pub const SUBTLE: Color = Color::Rgb {
    r: 55,
    g: 65,
    b: 81,
};
/// gray-500 — borders and chrome.
pub const BORDER: Color = Color::Rgb {
    r: 107,
    g: 114,
    b: 128,
};
/// amber-600 — the completion star.
pub const STAR: Color = Color::Rgb {
    r: 217,
    g: 119,
    b: 6,
};

// Prism light tokens from common.css.
pub const PLAIN: Color = Color::Rgb {
    r: 56,
    g: 72,
    b: 79,
};
pub const KEYWORD: Color = Color::Rgb {
    r: 62,
    g: 122,
    b: 226,
};
pub const PROPERTY: Color = Color::Rgb {
    r: 154,
    g: 48,
    b: 173,
};
pub const NUMBER: Color = Color::Rgb {
    r: 195,
    g: 59,
    b: 48,
};
pub const STRING: Color = Color::Rgb {
    r: 26,
    g: 143,
    b: 82,
};
pub const FUNCTION: Color = Color::Rgb {
    r: 213,
    g: 34,
    b: 98,
};
pub const COMMENT: Color = Color::Rgb {
    r: 100,
    g: 100,
    b: 100,
};
