//! The design tokens: a neutral light palette — near-black text on white and
//! light gray, with blue strictly as the interactive accent (buttons,
//! selection, links, the active tab) — plus the iced widget styles derived
//! from it. The view only ever refers to names in this module.

use iced::border::Radius;
use iced::{color, theme, Color, Theme};

pub const CHROME: Color = color!(0xf6f7f9);
pub const SURFACE: Color = color!(0xffffff);
pub const FIELD: Color = color!(0xffffff);
pub const HOVER: Color = color!(0xeceef1);
pub const TEXT: Color = color!(0x3b3f45);
pub const STRONG: Color = color!(0x22252a);
pub const MUTED: Color = color!(0x878d96);
pub const LINE: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.08);
pub const ACCENT: Color = color!(0x2563eb);
pub const ACCENT_SOFT: Color = Color::from_rgba(0.145, 0.388, 0.922, 0.10);
pub const DANGER: Color = color!(0xc4514a);
pub const SUCCESS: Color = color!(0x6f7f45);
pub const EDGE: Color = color!(0xc3c8cf);
pub const GRID: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.06);
pub const HALO: Color = Color::from_rgba(0.145, 0.388, 0.922, 0.18);

/// The app theme: light, neutral, blue-accented.
pub fn theme() -> Theme {
    Theme::custom(
        "warm-light",
        theme::Palette {
            background: SURFACE,
            text: TEXT,
            primary: ACCENT,
            success: SUCCESS,
            danger: DANGER,
            warning: ACCENT,
        },
    )
}

pub fn rounded(radius: f32) -> Radius {
    Radius {
        top_left: radius,
        top_right: radius,
        bottom_right: radius,
        bottom_left: radius,
    }
}

// --- container styles -------------------------------------------------------
// --- container styles -------------------------------------------------------

pub fn chrome() -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(CHROME.into()),
        ..Default::default()
    }
}

pub fn vertical_rule() -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(LINE.into()),
        ..Default::default()
    }
}

/// Sidebar note row: quiet hover wash, soft amber tint when selected.
pub fn note_row(selected: bool, hovered: bool) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(
            if selected {
                ACCENT_SOFT
            } else if hovered {
                HOVER
            } else {
                Color::TRANSPARENT
            }
            .into(),
        ),
        border: iced::border::Border {
            radius: rounded(6.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn dimmed() -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(Color::from_rgba(0.16, 0.17, 0.19, 0.40).into()),
        ..Default::default()
    }
}

pub fn card() -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(SURFACE.into()),
        border: iced::border::Border {
            color: LINE,
            width: 1.0,
            radius: rounded(12.0),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba(0.16, 0.17, 0.19, 0.30),
            offset: iced::Vector::new(0.0, 14.0),
            blur_radius: 44.0,
        },
        ..Default::default()
    }
}

// --- button styles ----------------------------------------------------------

pub fn primary_button() -> iced::widget::button::Style {
    iced::widget::button::Style {
        text_color: color!(0x171310),
        background: Some(ACCENT.into()),
        border: iced::border::Border {
            radius: rounded(6.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn danger_button() -> iced::widget::button::Style {
    iced::widget::button::Style {
        text_color: color!(0x1c1210),
        background: Some(DANGER.into()),
        border: iced::border::Border {
            radius: rounded(6.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Pill-shaped tag chip.
pub fn chip() -> iced::widget::button::Style {
    iced::widget::button::Style {
        text_color: TEXT,
        background: Some(HOVER.into()),
        border: iced::border::Border {
            radius: rounded(999.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

// --- scrollable style -------------------------------------------------------

/// A slim, borderless scrollbar: a thin rounded thumb on an invisible rail,
/// flush to the pane edge.
pub fn scrollable() -> iced::widget::scrollable::Style {
    let rail = iced::widget::scrollable::Rail {
        background: None,
        border: iced::border::Border::default(),
        scroller: iced::widget::scrollable::Scroller {
            background: Color::from_rgba(0.0, 0.0, 0.0, 0.18).into(),
            border: iced::border::Border::default(),
        },
    };
    iced::widget::scrollable::Style {
        container: Default::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: SURFACE.into(),
            border: iced::border::Border::default(),
            shadow: Default::default(),
            icon: MUTED,
        },
    }
}

// --- input styles -----------------------------------------------------------

pub fn text_input() -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: FIELD.into(),
        border: iced::border::Border {
            color: LINE,
            width: 1.0,
            radius: rounded(6.0),
        },
        icon: MUTED,
        placeholder: MUTED,
        value: TEXT,
        selection: ACCENT_SOFT,
    }
}
