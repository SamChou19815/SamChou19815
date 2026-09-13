//! The blog reader: a post over the blog index.

use crate::crypt::{encrypted_str, EncryptedString};
use crate::keys::Key;
use crate::markdown::{self, Block};
use crate::posts;
use crate::tab::Tab;
use crate::theme;
use leptos::attr::custom::custom_attribute;
use leptos::html;
use leptos::prelude::*;
use leptos_router::components::Redirect;

use super::app::Lists;
use super::keyboard::use_view_keys;
use super::nav::{replacing, use_path, use_show, Link};
use super::pane::{scroll_keys, Pane};
use super::text::{runs, style_of, StyledLine};

const CLOSE_LABEL: EncryptedString = encrypted_str!("Close");

#[component]
pub(super) fn Reader() -> impl IntoView {
    let path = use_path();
    let post = Memo::new(move |_| posts::find(&path.get()));
    move || match post.get() {
        Some(post) => view! { <Post post /> }.into_any(),
        None => {
            view! { <Redirect path=Tab::Blog.route().to_string() options=replacing() /> }.into_any()
        }
    }
}

#[component]
fn Post(post: usize) -> impl IntoView {
    // Closing the reader lands on this post's card, however it was opened.
    let Lists { blog, .. } = expect_context::<Lists>();
    Effect::new(move |_| blog.set(post));

    let pane = NodeRef::<html::Div>::new();
    let show = use_show();
    // Every non-Ctrl key is the reader's: tab keys are inert while a post is open.
    use_view_keys(move |key, mods| {
        if mods.ctrl {
            return false;
        }
        match key {
            Key::Esc | Key::Backspace | Key::Char('q') | Key::Left | Key::Char('h') => {
                show.run(Tab::Blog.route());
            }
            Key::Char('?') => show.run(Tab::Help.route()),
            key => {
                scroll_keys(pane, key);
            }
        }
        true
    });

    let blocks = markdown::post_blocks(&posts::POSTS[post].body().decrypt())
        .into_iter()
        .map(|block| view! { <BlockView block /> })
        .collect_view();
    view! {
        <Pane node_ref=pane>
            <div>
                <PostHeader post />
                {blocks}
            </div>
        </Pane>
    }
}

#[component]
fn PostHeader(post: usize) -> impl IntoView {
    let title = posts::POSTS[post].title().decrypt();
    let date = posts::POSTS[post].formatted_date();
    // `role=heading` rather than `<h1>`: the site stylesheet styles `h1`.
    // Leptos has no typed `aria-level`.
    let heading = view! {
        <div
            role="heading"
            class="min-w-0 break-words text-[1.6em] leading-[1.25] font-bold text-[#2563eb]"
        >
            {title}
        </div>
    }
    .add_any_attr(custom_attribute("aria-level", "1"));
    view! {
        <div class="w-full">
            <div class="h-row shrink-0"></div>
            <div class="grid w-full grid-cols-[1fr_3ch] items-start">
                {heading}
                // `leading-[2]` centers the x on the title's first line.
                <Link
                    url=Tab::Blog.route().to_string()
                    class="whitespace-pre text-center font-bold leading-[2] text-[#2563eb]"
                    {..}
                    aria-label=CLOSE_LABEL.decrypt()
                >
                    " x "
                </Link>
            </div>
            <div class="min-h-row whitespace-pre pt-[0.5lh]">
                <span style=style_of(theme::MUTED)>{date}</span>
            </div>
            <div class="h-row shrink-0"></div>
        </div>
    }
}

#[component]
fn BlockView(block: Block) -> impl IntoView {
    match block {
        Block::Line(line) => view! { <StyledLine line wraps=true /> }.into_any(),
        Block::Bullet(line) => view! {
            <div class="min-h-row whitespace-pre-wrap pl-[2ch]">
                <span class="-ml-[2ch]">{runs(&line)}</span>
            </div>
        }
        .into_any(),
        Block::Quote(line) => view! {
            <div class="min-h-row whitespace-pre-wrap border-l border-[#6b7280] pl-[2ch]">
                {runs(&line)}
            </div>
        }
        .into_any(),
        Block::Code {
            lang,
            lines,
            closed,
        } => {
            let fence_rule = |label: String| {
                view! {
                    <div class="flex w-full items-center">
                        <span class="shrink-0 whitespace-pre" style=style_of(theme::BORDER)>
                            {label}
                        </span>
                        <div class="h-px min-w-0 flex-1" style=format!("background:{};", theme::BORDER.css())></div>
                    </div>
                }
            };
            let rows = lines
                .into_iter()
                .map(|line| view! { <StyledLine line wraps=false /> })
                .collect_view();
            view! {
                <div class="overflow-x-auto">
                    {fence_rule(format!("── {lang} "))}
                    {rows}
                    {closed.then(|| fence_rule(String::from("──  ")))}
                </div>
            }
            .into_any()
        }
        Block::Image { url } => view! {
            <img
                class="block h-auto max-w-full w-auto max-h-[16lh]"
                src=url.to_string()
                alt=""
            />
        }
        .into_any(),
    }
}
