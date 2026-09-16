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
use super::text::{runs, style_of};

const CLOSE_LABEL: EncryptedString = encrypted_str!("Close");
const NEWER_LABEL: EncryptedString = encrypted_str!("Newer Post");
const OLDER_LABEL: EncryptedString = encrypted_str!("Older Post");
const POST_NAV_LABEL: EncryptedString = encrypted_str!("Blog post page navigation");

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
    // So closing the reader lands on this post's card.
    let Lists { blog, .. } = expect_context::<Lists>();
    Effect::new(move |_| blog.set(post));

    let pane = NodeRef::<html::Div>::new();
    // Navigating to a neighbor post reuses the pane, so reset scroll.
    Effect::new(move |_| {
        if let Some(pane) = pane.get() {
            pane.set_scroll_top(0);
        }
    });
    let show = use_show();
    use_view_keys(move |key, mods| {
        if mods.ctrl {
            return false;
        }
        match key {
            Key::Esc | Key::Backspace | Key::Char('q') | Key::Left | Key::Char('h') => {
                show.run(Tab::Blog.route());
            }
            key => {
                scroll_keys(pane, key);
            }
        }
        true
    });

    let blocks = markdown::post_blocks(&posts::POSTS[post])
        .into_iter()
        .filter(|block| !matches!(block, Block::Line(line) if line.is_empty()))
        .map(|block| view! { <BlockView block /> })
        .collect_view();
    view! {
        <Pane node_ref=pane>
            <div class="py-[2ch]">
                <div class="rounded-md border border-[#e5e7eb] bg-white p-[1rem]">
                    <PostHeader post />
                    {blocks}
                </div>
                <PostNav post />
            </div>
        </Pane>
    }
}

#[component]
fn PostNav(post: usize) -> impl IntoView {
    let newer = post.checked_sub(1);
    let older = (post + 1 < posts::POSTS.len()).then_some(post + 1);
    let half = "flex min-w-0 flex-1 @max-[56ch]:empty:hidden";
    view! {
        <nav class="mt-[2ch] flex gap-[1rem] @max-[56ch]:flex-col" aria-label=POST_NAV_LABEL.decrypt()>
            <div class=half>
                {newer.map(|index| view! { <NeighborCard index newer=true /> })}
            </div>
            <div class=format!("{half} text-right")>
                {older.map(|index| view! { <NeighborCard index newer=false /> })}
            </div>
        </nav>
    }
}

#[component]
fn NeighborCard(index: usize, newer: bool) -> impl IntoView {
    let post = &posts::POSTS[index];
    let label = if newer { NEWER_LABEL } else { OLDER_LABEL }.decrypt();
    let title = post.title();
    let mut title = if newer {
        format!("« {title}")
    } else {
        format!("{title} »")
    };
    if post.is_external() {
        title.push_str(" ↗");
    }
    view! {
        <Link
            url=post.url()
            class="group block w-full rounded-md border border-[#e5e7eb] bg-white p-[1rem] hover:border-[#2563eb] hover:bg-[#dbeafe]"
        >
            <div class="text-[0.9em]">
                <span class="text-[#4b5563] group-hover:text-[#1e3a8a]">{label}</span>
            </div>
            <div class="min-w-0 break-words pt-[0.25lh] font-bold leading-[1.3] text-[#2563eb] group-hover:text-[#1e3a8a]">
                {title}
            </div>
        </Link>
    }
}

#[component]
fn PostHeader(post: usize) -> impl IntoView {
    let title = posts::POSTS[post].title().decrypt();
    let date = posts::POSTS[post].formatted_date();
    let heading = view! {
        <div
            role="heading"
            class="min-w-0 break-words text-[1.9em] leading-[1.3] font-bold text-[#2563eb]"
        >
            {title}
        </div>
    }
    .add_any_attr(custom_attribute("aria-level", "1"));
    view! {
        <div class="w-full pb-[0.6lh]">
            <div class="grid w-full grid-cols-[1fr_auto] items-start gap-x-[1ch]">
                {heading}
                <Link
                    url=Tab::Blog.route().to_string()
                    class="whitespace-pre text-[1.9em] leading-[1.3] font-bold text-[#2563eb]"
                    {..}
                    aria-label=CLOSE_LABEL.decrypt()
                >
                    " x "
                </Link>
            </div>
            <div class="pt-[0.5lh] text-[0.9em]">
                <span style=style_of(theme::MUTED)>{format!("~ {date}")}</span>
            </div>
        </div>
    }
}

const RELAXED: &str = "leading-[1.7]";

#[component]
fn BlockView(block: Block) -> impl IntoView {
    match block {
        Block::Line(line) => view! {
            <div class=format!("{RELAXED} mt-[0.9lh] min-h-row whitespace-pre-wrap break-words")>
                {runs(&line)}
            </div>
        }
        .into_any(),
        Block::Heading { level, line } => {
            let (size, top) = match level {
                1 => ("text-[1.5em]", "mt-[2.2lh]"),
                2 => ("text-[1.3em]", "mt-[1.9lh]"),
                _ => ("text-[1.12em]", "mt-[1.7lh]"),
            };
            view! {
                <div class=format!("{size} {top} mb-[0.5lh] font-bold leading-[1.3] min-h-row break-words")>
                    {runs(&line)}
                </div>
            }
            .into_any()
        }
        Block::Bullet { marker, line } => {
            let hang = marker.chars().count() + 1;
            view! {
                <div
                    class=format!("{RELAXED} mt-[0.45lh] min-h-row whitespace-pre-wrap break-words")
                    style=format!("padding-left:{hang}ch;text-indent:-{hang}ch")
                >
                    <span class="font-bold" style=style_of(theme::ACCENT_TEXT)>
                        {format!("{marker} ")}
                    </span>
                    {runs(&line)}
                </div>
            }
        }
        .into_any(),
        Block::Quote(line) => view! {
            <div class=format!("{RELAXED} mt-[0.9lh] min-h-row whitespace-pre-wrap break-words border-l-2 border-[#6b7280] pl-[3ch]")>
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
                .iter()
                .map(|line| {
                    view! {
                        <div class="min-h-row whitespace-pre leading-[1.6]">{runs(line)}</div>
                    }
                })
                .collect_view();
            view! {
                <div class="mt-[1.2lh] w-full overflow-x-auto text-[0.95em]">
                    {fence_rule(format!("── {lang} "))}
                    <div class="py-[1lh]">{rows}</div>
                    {closed.then(|| fence_rule(String::from("──  ")))}
                </div>
            }
            .into_any()
        }
        Block::Image { url } => view! {
            <img
                class="mt-[1.2lh] block h-auto max-w-full w-auto max-h-[16lh]"
                src=url.to_string()
                alt=""
            />
        }
        .into_any(),
    }
}
