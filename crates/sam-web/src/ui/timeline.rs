//! The timeline: the life story as a listbox.

use crate::crypt::{encrypted_str, EncryptedString};
use crate::data;
use crate::keys::Key;
use crate::site_path::SitePath;
use crate::tab::Tab;
use leptos::html;
use leptos::prelude::*;

use super::app::Lists;
use super::keyboard::use_view_keys;
use super::listbox::{focus_follows_selection, hover, is_selected, list_keys};
use super::nav::{use_open_link, Link};
use super::pane::Pane;

const TIMELINE_LIST_LABEL: EncryptedString = encrypted_str!("Timeline");

#[component]
pub(super) fn Timeline() -> impl IntoView {
    let Lists {
        timeline: selected, ..
    } = expect_context::<Lists>();
    let pane = NodeRef::<html::Div>::new();
    let options: Vec<NodeRef<html::Li>> =
        (0..data::TIMELINE.len()).map(|_| NodeRef::new()).collect();
    focus_follows_selection(selected, &options);

    let open_link = use_open_link();
    use_view_keys(move |key, _| {
        if list_keys(selected, data::TIMELINE.len(), pane, key) {
            return true;
        }
        let Key::Char(c @ '1'..='9') = key else {
            return false;
        };
        let event = &data::TIMELINE[selected.get_untracked().min(data::TIMELINE.len() - 1)];
        if let Some(link) = event.links.get(c as usize - '1' as usize) {
            open_link(&link.url.decrypt());
        }
        true
    });

    let cards = data::TIMELINE
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let option = options[index];
            view! { <TimelineCard event index option /> }
        })
        .collect_view();
    view! {
        <Pane tab=Tab::Timeline node_ref=pane>
            <ul role="listbox" aria-orientation="vertical" aria-label=TIMELINE_LIST_LABEL.decrypt()>
                {cards}
            </ul>
        </Pane>
    }
}

fn link_button_label(index: usize, link: &data::Link) -> String {
    format!("{} {}", index + 1, link.name.decrypt().to_uppercase())
}

#[component]
fn TimelineCard(
    event: &'static data::TimelineEvent,
    index: usize,
    option: NodeRef<html::Li>,
) -> impl IntoView {
    let Lists {
        timeline: selected, ..
    } = expect_context::<Lists>();
    let is_selected = is_selected(selected, index);
    let on_click = move |event: web_sys::MouseEvent| {
        event.stop_propagation();
        selected.set(index);
    };
    // As a CSS variable so the selected variant can override it.
    let tag_style = format!("--tag:{};", event.category.color().css());
    let tag = format!("[{}]", event.category.label());

    let image = event.image.map(|image| {
        let url = SitePath::new(image.decrypt());
        view! {
            <div class="mt-row">
                <div class="pl-3">
                    <img
                        class="block h-auto max-w-full w-auto max-w-[min(100%,32ch)] max-h-[8lh]"
                        src=url.to_string()
                        alt=""
                    />
                </div>
            </div>
        }
    });
    let detail = event.detail.map(|detail| {
        view! {
            <div class="mt-row">
                <div class="whitespace-pre-wrap pl-3">
                    <span class="text-[#374151] group-aria-selected:text-[#1e3a8a]">
                        {detail.decrypt()}
                    </span>
                </div>
            </div>
        }
    });
    let links = (!event.links.is_empty()).then(|| {
        let buttons = event
            .links
            .iter()
            .enumerate()
            .map(|(link_index, link)| {
                let label = link_button_label(link_index, link);
                view! {
                    <Link
                        url=link.url.decrypt()
                        class="border border-[#2563eb] px-1 font-bold text-[#2563eb] hover:bg-[#2563eb] hover:text-[#f7f7f7]"
                    >
                        {label}
                    </Link>
                }
            })
            .collect_view();
        view! {
            <div class="mt-row">
                <div class="flex w-full flex-wrap gap-x-1 gap-y-row whitespace-pre pl-3">{buttons}</div>
            </div>
        }
    });

    view! {
        <li
            role="option"
            class="group relative w-full px-1 aria-selected:bg-[#dbeafe] outline-hidden"
            aria-selected=move || if is_selected.get() { "true" } else { "false" }
            aria-setsize=data::TIMELINE.len()
            aria-posinset=index + 1
            tabindex=move || if is_selected.get() { 0 } else { -1 }
            node_ref=option
            on:click=on_click
            on:mousemove=hover(selected, index)
        >
            // The rail; the marker's opaque box breaks it.
            <div class="pointer-events-none absolute inset-y-0 left-[calc(1.5ch_-_0.5px)] w-px bg-[#2563eb]"></div>
            <div class="h-row shrink-0"></div>
            <div class="flex w-full whitespace-pre">
                <div class="w-3 shrink-0 bg-[#f7f7f7] font-bold text-[#2563eb] group-aria-selected:bg-[#dbeafe]">
                    <span class="group-aria-selected:hidden">"●  "</span>
                    <span class="hidden group-aria-selected:inline">"▸  "</span>
                </div>
                <div class="min-w-0 flex-1">
                    <span class="block truncate font-bold text-[#1c1e21] group-aria-selected:text-[#1e3a8a]">
                        {event.title.decrypt()}
                    </span>
                </div>
                <span class="shrink-0 text-(--tag) group-aria-selected:text-[#1e3a8a]" style=tag_style>
                    {tag}
                </span>
            </div>
            <div class="flex w-full whitespace-pre">
                <div class="w-3 shrink-0"></div>
                <span class="text-[#4b5563] group-aria-selected:text-[#1e3a8a]">
                    {event.time.decrypt()}
                </span>
            </div>
            {image}
            {detail}
            {links}
            <div class="h-row shrink-0"></div>
        </li>
    }
}
