//! Roving tabindex listbox. Focusing the selected item is what scrolls it into view.

use crate::keys::Key;
use leptos::html;
use leptos::prelude::*;

use super::app::Pointer;
use super::pane::{apply_scroll, Scroll};

/// Memo so only the two affected cards rerender on selection change.
pub(super) fn is_selected(selected: RwSignal<usize>, index: usize) -> Memo<bool> {
    Memo::new(move |_| selected.get() == index)
}

/// On mount, don't scroll: the pane is about to restore its own scroll position.
pub(super) fn focus_follows_selection(selected: RwSignal<usize>, cards: &[NodeRef<html::Li>]) {
    let cards = cards.to_vec();
    Effect::new(move |previous: Option<()>| {
        let index = selected.get();
        if let Some(option) = cards.get(index).and_then(|card| card.get()) {
            let scroll = web_sys::FocusOptions::new();
            scroll.set_prevent_scroll(previous.is_none());
            let _ = option.focus_with_options(&scroll);
        }
    });
}

/// Browsers fire `mousemove` when content scrolls under a stationary pointer, which would undo
/// keyboard selection. Ignore events where the pointer didn't actually move.
pub(super) fn hover(selected: RwSignal<usize>, index: usize) -> impl Fn(web_sys::MouseEvent) {
    let Pointer(pointer) = expect_context::<Pointer>();
    move |event: web_sys::MouseEvent| {
        let at = (event.screen_x(), event.screen_y());
        if pointer.try_update_value(|last| std::mem::replace(last, at)) == Some(at) {
            return;
        }
        if selected.get_untracked() != index {
            selected.set(index);
        }
    }
}

pub(super) fn list_keys(
    selected: RwSignal<usize>,
    len: usize,
    pane: NodeRef<html::Div>,
    key: Key,
) -> bool {
    let last = len.saturating_sub(1);
    match key {
        Key::Up | Key::Char('k') => selected.set(selected.get_untracked().saturating_sub(1)),
        Key::Down | Key::Char('j') => selected.set((selected.get_untracked() + 1).min(last)),
        Key::Home | Key::Char('g') => selected.set(0),
        Key::End | Key::Char('G') => selected.set(last),
        Key::PageUp | Key::PageDown => {
            let pages = if matches!(key, Key::PageUp) { -1 } else { 1 };
            if let Some(pane) = pane.get_untracked() {
                apply_scroll(&pane, Scroll::Pages(pages));
            }
        }
        _ => return false,
    }
    true
}
