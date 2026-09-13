//! The scrolling box: what a scroll request means, and the pane that answers
//! it — one scroll position per tab, held by the app.

use crate::keys::Key;
use crate::tab::Tab;
use leptos::html;
use leptos::prelude::*;

/// A scrolling box. Tailwind has no scrollbar utilities, so the scrollbar
/// styling is arbitrary properties. The box must be pinned to less room than
/// its content (`h-full` under a definite-height parent), or nothing overflows
/// and nothing scrolls.
pub(super) const SCROLL: &str = concat!(
    "overflow-x-hidden overflow-y-auto ",
    "[scrollbar-width:thin] [scrollbar-color:#64656666_transparent] ",
    "[&::-webkit-scrollbar]:w-3.5 ",
    "[&::-webkit-scrollbar-track]:bg-transparent ",
    "[&::-webkit-scrollbar-thumb]:rounded-full ",
    "[&::-webkit-scrollbar-thumb]:bg-[#64656666] ",
    "[&::-webkit-scrollbar-thumb:hover]:bg-[#646566b3]",
);

#[derive(PartialEq)]
pub(super) enum Scroll {
    /// Rows, negative up.
    Rows(i32),
    /// Screenfuls, negative up.
    Pages(i32),
    Top,
    Bottom,
}

fn line_height(pane: &web_sys::Element) -> f64 {
    web_sys::window()
        .and_then(|window| window.get_computed_style(pane).ok().flatten())
        .and_then(|style| style.get_property_value("line-height").ok())
        .and_then(|value| value.trim().trim_end_matches("px").parse::<f64>().ok())
        .filter(|height| *height > 0.0)
        .unwrap_or(18.0)
}

pub(super) fn apply_scroll(pane: &web_sys::Element, by: Scroll) {
    let viewport = f64::from(pane.client_height());
    match by {
        Scroll::Rows(rows) => {
            pane.scroll_by_with_x_and_y(0.0, f64::from(rows) * line_height(pane));
        }
        // A screenful less two rows of overlap.
        Scroll::Pages(pages) => {
            let row = line_height(pane);
            let page = (viewport - 2.0 * row).max(row);
            pane.scroll_by_with_x_and_y(0.0, f64::from(pages) * page);
        }
        Scroll::Top => pane.set_scroll_top(0),
        Scroll::Bottom => pane.set_scroll_top(pane.scroll_height()),
    }
}

/// Held by the app so it survives the view unmounting.
#[derive(Clone, Copy)]
pub(super) struct ScrollPositions(pub(super) StoredValue<[f64; Tab::ALL.len()]>);

#[component]
pub(super) fn Pane(
    #[prop(optional)] tab: Option<Tab>,
    #[prop(optional)] node_ref: NodeRef<html::Div>,
    children: Children,
) -> impl IntoView {
    let ScrollPositions(positions) = expect_context::<ScrollPositions>();
    if let Some(tab) = tab {
        Effect::new(move |_| {
            if let Some(pane) = node_ref.get() {
                pane.set_scroll_top(positions.with_value(|saved| saved[tab.index()]) as i32);
            }
        });
    }
    let on_scroll = move |_| {
        if let (Some(tab), Some(pane)) = (tab, node_ref.get_untracked()) {
            positions.update_value(|saved| saved[tab.index()] = f64::from(pane.scroll_top()));
        }
    };
    view! {
        <div class=format!("{SCROLL} h-full") node_ref=node_ref on:scroll=on_scroll>
            <div class="mx-auto w-full max-w-88">{children()}</div>
        </div>
    }
}

pub(super) fn scroll_keys(pane: NodeRef<html::Div>, key: Key) -> bool {
    let by = match key {
        Key::Up | Key::Char('k') => Scroll::Rows(-1),
        Key::Down | Key::Char('j') => Scroll::Rows(1),
        Key::PageUp => Scroll::Pages(-1),
        Key::PageDown => Scroll::Pages(1),
        Key::Home | Key::Char('g') => Scroll::Top,
        Key::End | Key::Char('G') => Scroll::Bottom,
        _ => return false,
    };
    if let Some(pane) = pane.get_untracked() {
        apply_scroll(&pane, by);
    }
    true
}
