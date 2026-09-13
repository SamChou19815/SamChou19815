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
            <ul role="listbox" aria-orientation="vertical" aria-label=BLOG_LIST_LABEL.decrypt()>
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
            <div class="h-row shrink-0"></div>
            <Link
                url=post.url()
                class="block w-full border border-[#d1d5db] bg-[#f7f7f7] px-2 py-row group-aria-selected:border-[#3b82f6] group-aria-selected:bg-[#dbeafe]"
                {..}
                on:click=move |_| selected.set(index)
                on:mousemove=hover(selected, index)
            >
                <div class="min-h-row">
                    <span class="block truncate font-bold text-[#2563eb] group-aria-selected:text-[#1e3a8a]">
                        {title}
                    </span>
                </div>
                <div class="min-h-row whitespace-pre">
                    <span class="text-[#4b5563] group-aria-selected:text-[#1e3a8a]">
                        {post.formatted_date()}
                    </span>
                </div>
            </Link>
        </li>
    }
}
