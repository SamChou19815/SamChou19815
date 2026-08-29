//! Click regions recorded by components during rendering, consumed by the
//! app's mouse dispatch. `use_component_rect` reports the previous frame's
//! rect, so registrations are refreshed every render.
//!
//! A frame's regions belong to that frame alone: [`begin_frame`] drops the
//! previous ones, exactly as [`crate::image::begin_frame`] drops the previous
//! frame's artwork. Without that, a region outlives what drew it — the cards of
//! a tab that has since been switched away answer clicks on the pane that
//! replaced them, which is how a click on the About portrait used to open a
//! link belonging to another tab.
//!
//! Regions belong to a surface, which settles two things a rect alone cannot:
//!
//! - what covers what. An open dialog is painted over the pane and the header,
//!   so nothing behind it may answer a click — not the card it covers, not the
//!   tab label above it.
//! - what clips what. The pane lays out every card and paints only the ones
//!   inside its body, so a card scrolled past the bottom edge still has a rect,
//!   and that rect reaches down onto the status bar. A surface that clips its
//!   contents records where it painted ([`clip`]), and regions on it answer
//!   only inside that rectangle.

use std::cell::{Cell, RefCell};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.x && col < self.x + self.width && row >= self.y && row < self.y + self.height
    }
}

/// What a click on a region does.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HitTarget {
    Tab(usize),
    Item(usize),
    Link(String),
    /// The reader's close button, in the pane's title row. Keys close the
    /// reader too, but a pointer had no way out of a post before this.
    Close,
    /// The open dialog's own frame. It swallows the click instead of acting on
    /// it: a click that lands on the dialog is not a click on the pane behind
    /// it, and so must neither dismiss the dialog nor reach what it covers.
    Dialog,
}

/// The surfaces a region can belong to.
///
/// [`CHROME`] is the header, which no pane clips. [`PANE`] is the content pane,
/// clipped to its body. [`DIALOG`] and [`DIALOG_BODY`] are the open dialog's
/// frame and its scrolling body, which cover everything else; only the body
/// clips.
pub const CHROME: u8 = 0;
pub const PANE: u8 = 1;
pub const DIALOG: u8 = 2;
pub const DIALOG_BODY: u8 = 3;
const SURFACES: usize = 4;

/// Whether a surface belongs to the open dialog rather than to what it covers.
fn is_dialog(surface: u8) -> bool {
    matches!(surface, DIALOG | DIALOG_BODY)
}

/// The way out ranks above everything, then links beat cards beat tabs, so a
/// button on a card opens instead of selecting it. The dialog's frame ranks
/// below all of them, because everything it carries sits on top of it.
fn priority(target: &HitTarget) -> u8 {
    match target {
        HitTarget::Close => 4,
        HitTarget::Link(_) => 3,
        HitTarget::Item(_) => 2,
        HitTarget::Tab(_) => 1,
        HitTarget::Dialog => 0,
    }
}

struct Region {
    surface: u8,
    rect: Rect,
    target: HitTarget,
}

thread_local! {
    static REGIONS: RefCell<Vec<Region>> = const { RefCell::new(Vec::new()) };
    /// Where each clipping surface actually painted this frame.
    static CLIPS: RefCell<[Option<Rect>; SURFACES]> = const { RefCell::new([None; SURFACES]) };
    /// Whether a dialog covers the frame, which decides which surfaces answer.
    static DIALOG_OPEN: Cell<bool> = const { Cell::new(false) };
}

/// Drops the previous frame's regions. Called from `Root` at the top of the
/// update pass — which iocraft always follows with a draw pass before handing
/// control back — so a click is only ever tested against a whole frame's worth
/// of regions, all of them belonging to what is on screen now.
pub fn begin_frame(dialog_open: bool) {
    REGIONS.with(|regions| regions.borrow_mut().clear());
    CLIPS.with(|clips| *clips.borrow_mut() = [None; SURFACES]);
    DIALOG_OPEN.with(|open| open.set(dialog_open));
}

pub fn register(surface: u8, rect: Rect, target: HitTarget) {
    REGIONS.with(|regions| {
        regions.borrow_mut().push(Region {
            surface,
            rect,
            target,
        });
    });
}

/// Records where a clipping surface painted, so the regions on it that were
/// laid out past its edge cannot answer a click landing outside.
pub fn clip(surface: u8, rect: Rect) {
    CLIPS.with(|clips| clips.borrow_mut()[usize::from(surface)] = Some(rect));
}

/// Finds the target under a cell, on the surfaces the frame left visible.
pub fn hit_test(col: u16, row: u16) -> Option<HitTarget> {
    let dialog_open = DIALOG_OPEN.with(Cell::get);
    let clips = CLIPS.with(|clips| *clips.borrow());
    REGIONS.with(|regions| {
        let mut best: Option<(u8, HitTarget)> = None;
        for region in regions.borrow().iter() {
            // An open dialog answers for the whole screen; without one, the
            // dialog's surfaces are not on it at all.
            if is_dialog(region.surface) != dialog_open {
                continue;
            }
            if !region.rect.contains(col, row) {
                continue;
            }
            if clips[usize::from(region.surface)].is_some_and(|clip| !clip.contains(col, row)) {
                continue;
            }
            let priority = priority(&region.target);
            // Regions are recorded in render order, so a tie goes to whichever
            // was drawn last and is therefore on top.
            if best
                .as_ref()
                .is_none_or(|(best_priority, _)| priority >= *best_priority)
            {
                best = Some((priority, region.target.clone()));
            }
        }
        best.map(|(_, target)| target)
    })
}
