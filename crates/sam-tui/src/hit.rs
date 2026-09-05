//! Click regions, recorded by the components that paint them and consumed by
//! the app's mouse dispatch.
//!
//! A component asks to be clickable with one line — [`UseHit::use_hit_region`]
//! — and everything else follows from where and when it paints.
//!
//! # Last painted wins
//!
//! Regions are recorded in paint order, and a click resolves to the last one
//! covering the cell: the painter's algorithm, run over rectangles instead of
//! pixels. Paint order is the component tree walked in order, parents before
//! their children, so it already carries every ranking a hand-kept table of
//! priorities would have had to state twice —
//!
//! - a card's link is a *descendant* of the card, so it paints after it and a
//!   click on the link opens it rather than selecting the card;
//! - the open dialog is the root's last child, so it paints over the pane, the
//!   header and the reader's close button, and none of them answer while it is
//!   up — including the screen around it, which is a region of its own
//!   ([`HitTarget::Dismiss`]) rather than the absence of one;
//! - the dialog's body lines paint after the dialog's own frame, so a link in
//!   an open dialog still opens.
//!
//! What that asks of the tree in return is that an interactive child paint
//! inside the box of whatever contains it. Flex layout gives exactly that: a
//! child is laid out inside its parent, so it cannot paint anywhere its parent
//! did not.
//!
//! # Clipped by what contained it
//!
//! A pane lays out every card of a tab and paints only the ones that fit, so
//! the card after the last visible one still has a rect — one that reaches down
//! over the status bar, close enough to click. [`UseHit::use_hit_clip`] brackets
//! a container's children with the box it painted, exactly as iocraft's own
//! `with_clip_rect_for_children` brackets their drawing, and a region is cut
//! down to the clip in force when it was recorded. Nothing pushes a clip around
//! the header, so the tabs and the close button answer wherever they are.
//!
//! # Recorded while painting, not while rendering
//!
//! Both of those hooks run during the *draw* pass rather than the update pass
//! that builds the tree, which is what makes a region describe the frame the
//! user is looking at: geometry only exists once layout has run, and by then
//! rendering is over. A hook that read a rect during the update pass — as
//! `use_component_rect` does — would be reporting the previous frame, so a click
//! arriving right after a scroll would land a frame behind the pointer. Neither
//! hook implements `poll_change`, so neither costs the extra render-and-layout
//! pass `use_component_rect` forces whenever a rect moves.
//!
//! [`crate::frame`] drops what the previous frame recorded, just before this one
//! paints.

use iocraft::prelude::*;
use std::cell::RefCell;

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

    /// The cells two rects have in common. `None` when they share none, which
    /// is how a region laid out entirely past its container's edge stops being
    /// a region at all.
    pub fn intersect(&self, other: Rect) -> Option<Rect> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (u32::from(self.x) + u32::from(self.width))
            .min(u32::from(other.x) + u32::from(other.width));
        let bottom = (u32::from(self.y) + u32::from(self.height))
            .min(u32::from(other.y) + u32::from(other.height));
        (right > u32::from(x) && bottom > u32::from(y)).then(|| Rect {
            x,
            y,
            width: (right - u32::from(x)) as u16,
            height: (bottom - u32::from(y)) as u16,
        })
    }

    /// A rect from canvas edges that may sit partly off the top or left of the
    /// screen — a block the reader has scrolled up out of sight does — cropped
    /// to what is on it.
    fn from_edges(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        let limit = i32::from(u16::MAX);
        let x = left.clamp(0, limit);
        let y = top.clamp(0, limit);
        Self {
            x: x as u16,
            y: y as u16,
            width: (right.clamp(0, limit) - x).max(0) as u16,
            height: (bottom.clamp(0, limit) - y).max(0) as u16,
        }
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
    /// The screen around an open dialog, which the dialog itself paints over.
    /// A click out there is the pointer's way of asking for the dialog to go.
    Dismiss,
}

thread_local! {
    /// Regions in paint order; the last one covering a cell is the one on top.
    static REGIONS: RefCell<Vec<(Rect, HitTarget)>> = const { RefCell::new(Vec::new()) };
    /// The clips in force while painting, innermost last.
    static CLIPS: RefCell<Vec<Rect>> = const { RefCell::new(Vec::new()) };
}

/// Finds the target under a cell: the last region painted over it.
pub fn hit_test(col: u16, row: u16) -> Option<HitTarget> {
    REGIONS.with(|regions| {
        regions
            .borrow()
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(col, row))
            .map(|(_, target)| target.clone())
    })
}

/// Drops the previous frame's regions. Called by [`crate::frame`] alone, from
/// the outermost component, before anything has painted into this frame.
pub(crate) fn clear() {
    REGIONS.with(|regions| regions.borrow_mut().clear());
    CLIPS.with(|clips| clips.borrow_mut().clear());
}

/// The rect a component painted into, in canvas coordinates.
fn painted_rect(drawer: &ComponentDrawer<'_>) -> Rect {
    let position = drawer.canvas_position();
    let size = drawer.size();
    let (left, top) = (i32::from(position.x), i32::from(position.y));
    Rect::from_edges(
        left,
        top,
        left + i32::from(size.width),
        top + i32::from(size.height),
    )
}

/// The registration hooks, in the shape iocraft documents for extending
/// [`Hooks`]. Both are called unconditionally, once per render, like every
/// other hook.
pub trait UseHit {
    /// Registers the box this component paints as a click region, refreshed
    /// every frame it draws. `None` registers nothing, so a component whose
    /// target depends on its props — a markdown line is a link only when it
    /// carries a URL — can still call this on every render.
    fn use_hit_region(&mut self, target: Option<HitTarget>);

    /// Clips every region this component's children register to the box this
    /// one painted, so a card laid out past the bottom edge of the pane cannot
    /// answer a click landing on the status bar below it. The component must
    /// hide its own overflow, or it will be promising a bound the canvas does
    /// not keep.
    fn use_hit_clip(&mut self);
}

impl UseHit for Hooks<'_, '_> {
    fn use_hit_region(&mut self, target: Option<HitTarget>) {
        // The hook outlives the render that made it, so the target is assigned
        // fresh each time: the card a `HitBlock` stands for changes as the
        // pane scrolls, and the region has to name the one drawn now.
        self.use_hook(|| HitRegionHook { target: None }).target = target;
    }

    fn use_hit_clip(&mut self) {
        self.use_hook(|| HitClipHook);
    }
}

struct HitRegionHook {
    target: Option<HitTarget>,
}

impl Hook for HitRegionHook {
    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        let Some(target) = self.target.clone() else {
            return;
        };
        let rect = painted_rect(drawer);
        let clipped = CLIPS.with(|clips| match clips.borrow().last() {
            Some(clip) => rect.intersect(*clip),
            None => Some(rect),
        });
        if let Some(rect) = clipped {
            REGIONS.with(|regions| regions.borrow_mut().push((rect, target)));
        }
    }
}

struct HitClipHook;

impl Hook for HitClipHook {
    /// Pushed before the children paint and popped after they have, so the
    /// stack brackets exactly their subtree and can never come out unbalanced.
    fn pre_component_draw(&mut self, drawer: &mut ComponentDrawer) {
        // Inset by the border, which is what the canvas clips children to.
        let border = drawer.layout().border;
        let position = drawer.canvas_position();
        let size = drawer.size();
        let (left, top) = (i32::from(position.x), i32::from(position.y));
        let rect = Rect::from_edges(
            left + border.left as i32,
            top + border.top as i32,
            left + i32::from(size.width) - border.right as i32,
            top + i32::from(size.height) - border.bottom as i32,
        );
        CLIPS.with(|clips| {
            let mut clips = clips.borrow_mut();
            // A container that is itself entirely clipped away lets nothing
            // through, which an empty rect says exactly.
            let clipped = match clips.last() {
                Some(outer) => rect.intersect(*outer),
                None => Some(rect),
            };
            clips.push(clipped.unwrap_or(Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            }));
        });
    }

    fn post_component_draw(&mut self, _drawer: &mut ComponentDrawer) {
        CLIPS.with(|clips| {
            clips.borrow_mut().pop();
        });
    }
}
