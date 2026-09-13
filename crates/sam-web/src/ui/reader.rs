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
use super::text::{runs, style_of};

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

    let blocks = markdown::post_blocks(&posts::POSTS[post])
        .into_iter()
        // Blank lines were the TUI's paragraph spacing; margins do that now.
        .filter(|block| !matches!(block, Block::Line(line) if line.is_empty()))
        .map(|block| view! { <BlockView block /> })
        .collect_view();
    view! {
        <Pane node_ref=pane>
            // The article as the site drew it before the wasm rewrite: a
            // white card on the gray page, without the drop shadow that card
            // carried — this is still a terminal. Its gray frame is the same
            // 2ch the app insets everything by, on every side. The inner
            // measures are written in rem — in the terminal a Tailwind
            // spacing unit is a character, not a quarter of one.
            <div class="my-[2ch] rounded-md border border-[#e5e7eb] bg-white p-[1rem]">
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
            class="min-w-0 break-words text-[1.9em] leading-[1.3] font-bold text-[#2563eb]"
        >
            {title}
        </div>
    }
    .add_any_attr(custom_attribute("aria-level", "1"));
    view! {
        <div class="w-full pb-[0.6lh]">
            <div class="grid w-full grid-cols-[1fr_4ch] items-start gap-x-[1ch]">
                {heading}
                // `leading-[2.45em]` centers the x on the title's first line.
                <Link
                    url=Tab::Blog.route().to_string()
                    class="whitespace-pre text-center font-bold leading-[2.45em] text-[#2563eb]"
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

/// Prose leading: relaxed, where the terminal's own rows stay tight.
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
            // The levels breathe on their own scale: the deeper the heading,
            // the less air it needs before it.
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
            // The marker and the space after it hang: wrapped lines align
            // exactly under the first line's text, `•` at two characters, a
            // number as wide as it is written.
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
            // Code keeps the terminal's tight grid — just a step smaller than
            // the prose around it, with air inside its fences.
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
