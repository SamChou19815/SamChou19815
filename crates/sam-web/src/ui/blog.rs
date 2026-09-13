//! The blog index: posts as a listbox.

use crate::crypt::{encrypted_str, EncryptedString};
use crate::keys::Key;
use crate::posts;
use crate::tab::Tab;
use leptos::html;
use leptos::prelude::*;

use super::app::Lists;
use super::keyboard::use_view_keys;
use super::listbox::{focus_follows_selection, hover, is_selected, list_keys};
use super::nav::{use_open_link, Link};
use super::pane::Pane;

const BLOG_LIST_LABEL: EncryptedString = encrypted_str!("Blog posts");

#[component]
pub(super) fn Blog() -> impl IntoView {
    let Lists { blog: selected, .. } = expect_context::<Lists>();
    let pane = NodeRef::<html::Div>::new();
    let options: Vec<NodeRef<html::Li>> = (0..posts::POSTS.len()).map(|_| NodeRef::new()).collect();
    focus_follows_selection(selected, &options);

    let open_link = use_open_link();
    use_view_keys(move |key, _| {
        if list_keys(selected, posts::POSTS.len(), pane, key) {
            return true;
        }
        match key {
            Key::Enter => {
                let post = &posts::POSTS[selected.get_untracked().min(posts::POSTS.len() - 1)];
                open_link(&post.url());
                true
            }
            _ => false,
        }
    });

    let cards = posts::POSTS
        .iter()
        .enumerate()
        .map(|(index, post)| {
            let option = options[index];
            view! { <PostCard post index option /> }
        })
        .collect_view();
    view! {
        <Pane tab=Tab::Blog node_ref=pane>
            // The index as the site drew it before the wasm rewrite: a stack
            // of the same white cards the article is read on, `1rem` apart as
            // they were then, inside the reader's `2ch` frame.
            <ul
                role="listbox"
                aria-orientation="vertical"
                aria-label=BLOG_LIST_LABEL.decrypt()
                class="flex w-full flex-col gap-[1rem] py-[2ch]"
            >
                {cards}
            </ul>
        </Pane>
    }
}

#[component]
fn PostCard(post: &'static posts::Post, index: usize, option: NodeRef<html::Li>) -> impl IntoView {
    let Lists { blog: selected, .. } = expect_context::<Lists>();
    let is_selected = is_selected(selected, index);
    let title = if post.is_external() {
        format!("{} ↗", post.title())
    } else {
        post.title().to_string()
    };
    view! {
        <li
            role="option"
            class="group w-full outline-hidden"
            aria-selected=move || if is_selected.get() { "true" } else { "false" }
            aria-setsize=posts::POSTS.len()
            aria-posinset=index + 1
            tabindex=move || if is_selected.get() { 0 } else { -1 }
            node_ref=option
        >
            // The reader's card. The old site lifted a card's shadow on
            // hover; this is still a terminal, so the cursor is the listbox
            // highlight: the selected card fills and its border takes the
            // accent.
            <Link
                url=post.url()
                class="block w-full rounded-md border border-[#e5e7eb] bg-white p-[1rem] group-aria-selected:border-[#2563eb] group-aria-selected:bg-[#dbeafe]"
                {..}
                on:click=move |_| selected.set(index)
                on:mousemove=hover(selected, index)
            >
                // The reader's title a step down — its second-level heading —
                // wrapping rather than truncating: a card has the room.
                <div class="min-w-0 break-words text-[1.3em] leading-[1.3] font-bold text-[#2563eb] group-aria-selected:text-[#1e3a8a]">
                    {title}
                </div>
                <div class="pt-[0.5lh] text-[0.9em]">
                    <span class="text-[#4b5563] group-aria-selected:text-[#1e3a8a]">
                        {format!("~ {}", post.formatted_date())}
                    </span>
                </div>
            </Link>
        </li>
    }
}
