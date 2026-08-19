//! The developersam.com homepage design language, translated to the TUI.
//!
//! Every color here is lifted from the site itself (`common.css` +
//! `dev-sam-theme` + tailwind usage in `page.tsx`), so the terminal edition
//! reads as the same product — in the site's light mode:
//!
//! - body `#f7f7f7`, cards white, code blocks `#f7f7f7` (light mode)
//! - `blue-500 #3b82f6` is THE accent: nav, timeline line and dots, buttons,
//!   selection
//! - buttons are uppercase bold blue (`ButtonLink`), like `WEBSITE`
//! - the timeline is a vertical blue line with a dot per card
//! - code uses the site's Prism light token colors (comments italic)

use ratatui_core::style::{Color, Modifier, Style};

/// blue-500 — the site's accent, used for selection backgrounds and other
/// filled surfaces (black text on it scores 5.7:1).
pub const ACCENT: Color = Color::Rgb(59, 130, 246);
/// blue-600 — the accent as TEXT on the light background (≈5:1); blue-500
/// text measured too washed out.
pub const ACCENT_TEXT: Color = Color::Rgb(37, 99, 235);
/// Selected-row tint: blue-100, the site's `bg-blue-500 bg-opacity-10` hover.
pub const SELECT_BG: Color = Color::Rgb(219, 234, 254);
/// Selected-row text: blue-900 on the blue-100 tint (≈8:1).
pub const SELECT_FG: Color = Color::Rgb(30, 58, 138);
/// Card surface: white, like the homepage cards on the `#f7f7f7` body.
pub const CARD_BG: Color = Color::Rgb(255, 255, 255);
/// Code block background `#f7f7f7`, as on the homepage.
pub const CODE_BG: Color = Color::Rgb(247, 247, 247);
/// Site body background `#f7f7f7` (the web terminal's default background).
pub const BODY_BG: Color = Color::Rgb(247, 247, 247);

/// Site body text `#1c1e21`.
pub const TEXT: Color = Color::Rgb(28, 30, 33);
/// gray-600 — secondary text: subheaders, raw URLs, hints (readable on white).
pub const MUTED: Color = Color::Rgb(75, 85, 99);
/// gray-700 — detail and tagline text on white cards.
pub const SUBTLE: Color = Color::Rgb(55, 65, 81);
/// gray-500 — borders and chrome.
pub const BORDER: Color = Color::Rgb(107, 114, 128);
/// amber-600 — the completion star.
pub const STAR: Color = Color::Rgb(217, 119, 6);

// Prism light tokens from common.css.
pub const PLAIN: Color = Color::Rgb(56, 72, 79);
pub const KEYWORD: Color = Color::Rgb(62, 122, 226);
pub const PROPERTY: Color = Color::Rgb(154, 48, 173);
pub const NUMBER: Color = Color::Rgb(195, 59, 48);
pub const STRING: Color = Color::Rgb(26, 143, 82);
pub const FUNCTION: Color = Color::Rgb(213, 34, 98);
pub const COMMENT: Color = Color::Rgb(100, 100, 100);

/// The homepage button style: uppercase, bold, blue.
pub fn button_style() -> Style {
    Style::new().fg(ACCENT).add_modifier(Modifier::BOLD)
}
