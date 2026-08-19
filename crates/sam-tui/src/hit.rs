//! Click regions recorded by components during rendering, consumed by the
//! app's mouse dispatch. `use_component_rect` reports the previous frame's
//! rect, so registrations are refreshed every render.

use std::cell::RefCell;
use std::collections::HashMap;

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
}

thread_local! {
    static REGISTRY: RefCell<HashMap<(u8, usize), (Rect, HitTarget)>> =
        RefCell::new(HashMap::new());
}

/// Kind key for deduplication across re-renders.
pub const TAB: u8 = 0;
pub const ITEM: u8 = 1;
pub const LINK: u8 = 2;

pub fn register(kind: u8, index: usize, rect: Rect, target: HitTarget) {
    REGISTRY.with(|registry| {
        registry.borrow_mut().insert((kind, index), (rect, target));
    });
}

pub fn clear() {
    REGISTRY.with(|registry| registry.borrow_mut().clear());
}

/// Finds the target under a cell, preferring links over items over tabs.
pub fn hit_test(col: u16, row: u16) -> Option<HitTarget> {
    REGISTRY.with(|registry| {
        let registry = registry.borrow();
        let mut best: Option<(u8, &HitTarget)> = None;
        for (rect, target) in registry.values() {
            if rect.contains(col, row) {
                let priority = match target {
                    HitTarget::Link(_) => 3,
                    HitTarget::Item(_) => 2,
                    HitTarget::Tab(_) => 1,
                };
                if best.is_none_or(|(best_priority, _)| priority > best_priority) {
                    best = Some((priority, target));
                }
            }
        }
        best.map(|(_, target)| target.clone())
    })
}

/// All registered link regions (for hover affordances on the web).
pub fn links() -> Vec<(Rect, String)> {
    REGISTRY.with(|registry| {
        registry
            .borrow()
            .values()
            .filter_map(|(rect, target)| match target {
                HitTarget::Link(url) => Some((*rect, url.clone())),
                _ => None,
            })
            .collect()
    })
}
