//! Interaction regions collected while painting the view tree.

use ratatui_core::layout::Rect;

/// A clickable URL rendered somewhere on screen.
#[derive(Clone, PartialEq, Eq)]
pub struct LinkRegion {
    pub rect: Rect,
    pub url: String,
}

/// What an interactive region points at.
#[derive(Clone, PartialEq, Eq)]
pub enum HitRef {
    Tab(usize),
    Item(usize),
    Link(String),
}

/// Mouse hit areas recorded during the last draw.
#[derive(Default)]
pub struct HitAreas {
    pub tabs: Vec<Rect>,
    /// Clickable list rows, each paired with its item index (timeline cards
    /// occupy several rows; all of them map to the same event).
    pub rows: Vec<(Rect, usize)>,
    pub links: Vec<LinkRegion>,
    pub modal: Option<Rect>,
    pub content: Option<Rect>,
}
