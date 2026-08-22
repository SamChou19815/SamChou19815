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

/// Links beat cards beat tabs, so a button on a card opens instead of selecting
/// it. The dialog's frame ranks below all of them, because everything it
/// carries sits on top of it.
fn priority(target: &HitTarget) -> u8 {
    match target {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: u16, y: u16, width: u16, height: u16) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn link(url: &str) -> HitTarget {
        HitTarget::Link(url.to_string())
    }

    /// The bug behind a click on the About portrait opening another tab's link:
    /// regions used to outlive the frame that drew them.
    #[test]
    fn a_frame_forgets_the_regions_of_the_last_one() {
        begin_frame(false);
        register(PANE, rect(0, 3, 40, 1), link("https://example.com"));
        assert_eq!(hit_test(1, 3), Some(link("https://example.com")));
        // The next frame draws a pane with nothing clickable on it.
        begin_frame(false);
        assert_eq!(hit_test(1, 3), None);
    }

    #[test]
    fn links_beat_cards_beat_tabs() {
        begin_frame(false);
        register(CHROME, rect(0, 0, 40, 10), HitTarget::Tab(2));
        register(PANE, rect(0, 0, 40, 10), HitTarget::Item(5));
        register(PANE, rect(4, 4, 10, 1), link("https://flow.org"));
        assert_eq!(hit_test(5, 4), Some(link("https://flow.org")));
        assert_eq!(hit_test(5, 6), Some(HitTarget::Item(5)));
    }

    /// Two cards can never overlap, but a region redrawn in place should not
    /// lose to the one drawn under it.
    #[test]
    fn a_tie_goes_to_whatever_was_drawn_last() {
        begin_frame(false);
        register(PANE, rect(0, 0, 40, 10), HitTarget::Item(1));
        register(PANE, rect(0, 0, 40, 10), HitTarget::Item(2));
        assert_eq!(hit_test(1, 1), Some(HitTarget::Item(2)));
    }

    /// A card scrolled past the pane's bottom edge is laid out all the same,
    /// with a rect that reaches onto the status bar below it.
    #[test]
    fn a_region_clipped_away_by_its_surface_does_not_answer() {
        begin_frame(false);
        clip(PANE, rect(1, 3, 40, 5));
        register(PANE, rect(1, 6, 40, 4), HitTarget::Item(9));
        assert_eq!(hit_test(2, 7), Some(HitTarget::Item(9)));
        // Row 8 is past the pane's body: the status bar, not the card.
        assert_eq!(hit_test(2, 8), None);
    }

    /// An unclipped surface answers everywhere it was laid out — the header
    /// sits outside every pane.
    #[test]
    fn an_unclipped_surface_is_not_narrowed_by_another_surfaces_clip() {
        begin_frame(false);
        clip(PANE, rect(0, 3, 40, 5));
        register(CHROME, rect(0, 0, 8, 1), HitTarget::Tab(1));
        assert_eq!(hit_test(1, 0), Some(HitTarget::Tab(1)));
    }

    #[test]
    fn a_dialog_covers_everything_behind_it() {
        begin_frame(true);
        register(PANE, rect(0, 0, 80, 20), HitTarget::Item(3));
        register(CHROME, rect(0, 0, 8, 1), HitTarget::Tab(1));
        register(DIALOG, rect(8, 4, 60, 12), HitTarget::Dialog);
        register(DIALOG_BODY, rect(10, 8, 20, 1), link("https://samlang.io"));
        // The card and the tab label are behind the dialog.
        assert_eq!(hit_test(2, 2), None);
        assert_eq!(hit_test(1, 0), None);
        // The dialog itself swallows the click; its links still open.
        assert_eq!(hit_test(40, 5), Some(HitTarget::Dialog));
        assert_eq!(hit_test(12, 8), Some(link("https://samlang.io")));
    }

    /// The dialog's body scrolls, so a line past its bottom edge is laid out
    /// below the dialog — over the pane, where a click means "dismiss".
    #[test]
    fn a_dialog_line_below_the_dialog_does_not_answer() {
        begin_frame(true);
        clip(DIALOG_BODY, rect(9, 5, 58, 10));
        register(DIALOG_BODY, rect(10, 16, 20, 1), link("https://samlang.io"));
        assert_eq!(hit_test(12, 16), None);
    }
}
