//! The Help tab: the key bindings.

use crate::crypt::encrypted_str;
use crate::tab::Tab;
use crate::theme;
use leptos::html;
use leptos::prelude::*;

use super::keyboard::use_view_keys;
use super::pane::{scroll_keys, Pane};
use super::text::style_of;

#[component]
pub(super) fn Help() -> impl IntoView {
    let pane = NodeRef::<html::Div>::new();
    use_view_keys(move |key, _| scroll_keys(pane, key));
    let rows = [
        (
            encrypted_str!("←/→ or h/l"),
            encrypted_str!("switch between tabs"),
        ),
        (
            encrypted_str!("1 … 9"),
            encrypted_str!("open a link of the selected card (timeline) · jump to a tab elsewhere"),
        ),
        (
            encrypted_str!("↑/↓ or j/k"),
            encrypted_str!("move selection / scroll"),
        ),
        (encrypted_str!("Enter"), encrypted_str!("read a post")),
        (
            encrypted_str!("g / G"),
            encrypted_str!("jump to top / bottom"),
        ),
        (encrypted_str!("Esc"), encrypted_str!("close the reader")),
        (encrypted_str!("?"), encrypted_str!("open this tab")),
        (encrypted_str!("q / Ctrl+C"), encrypted_str!("quit")),
        (
            encrypted_str!("mouse"),
            encrypted_str!("click tabs, cards and links · the pane scrolls"),
        ),
    ]
    .iter()
    .map(|(keys, description)| {
        let keys = keys.decrypt();
        let description = description.decrypt();
        view! { <HelpRow keys description /> }
    })
    .collect_view();
    view! {
        <Pane tab=Tab::Help node_ref=pane>
            <div>
                <div class="h-row shrink-0"></div>
                {rows}
            </div>
        </Pane>
    }
}

#[component]
fn HelpRow(keys: String, description: String) -> impl IntoView {
    let keys_style = style_of(theme::ACCENT_TEXT);
    let description_style = style_of(theme::SUBTLE);
    let keys_wide = format!("  {keys:<14}");
    let keys_narrow = format!("  {keys}");
    let description_indented = format!("    {description}");
    view! {
        <div>
            <div class="min-h-row whitespace-pre-wrap break-words @max-[56ch]:hidden">
                <span class="font-bold" style=keys_style.clone()>{keys_wide}</span>
                <span style=description_style.clone()>{description}</span>
            </div>
            <div class="@min-[56ch]:hidden">
                <div class="min-h-row whitespace-pre-wrap">
                    <span class="font-bold" style=keys_style>{keys_narrow}</span>
                </div>
                <div class="min-h-row whitespace-pre-wrap break-words">
                    <span style=description_style>{description_indented}</span>
                </div>
            </div>
        </div>
    }
}
