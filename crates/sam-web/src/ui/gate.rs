use crate::crypt::{encrypted_str, EncryptedString};
use crate::keys::{Key, Keymap, Mods};
use leptos::html;
use leptos::prelude::*;

use super::keyboard::use_keyboard;

const YES: usize = 0;
const NO: usize = 1;

/// The gate's answer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Verdict {
    Agent,
    Human,
}

const VERDICT_KEY: EncryptedString = encrypted_str!("sam-web:verdict");
const AGENT: EncryptedString = encrypted_str!("agent");
const HUMAN: EncryptedString = encrypted_str!("human");

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// The answer from an earlier visit, so the gate asks once per browser.
pub(super) fn remembered_verdict() -> Option<Verdict> {
    let stored = local_storage()?.get_item(&VERDICT_KEY.decrypt()).ok()??;
    if stored == AGENT.decrypt() {
        Some(Verdict::Agent)
    } else if stored == HUMAN.decrypt() {
        Some(Verdict::Human)
    } else {
        None
    }
}

/// Best effort: storage can be full or blocked, and then the gate just asks again next visit.
pub(super) fn remember_verdict(verdict: Verdict) {
    let Some(storage) = local_storage() else {
        return;
    };
    let value = match verdict {
        Verdict::Agent => AGENT,
        Verdict::Human => HUMAN,
    };
    let _ = storage.set_item(&VERDICT_KEY.decrypt(), &value.decrypt());
}

#[component]
pub(super) fn Gate(on_verdict: Callback<Verdict>) -> impl IntoView {
    // A human who just hits Enter should land in the app, never the maze.
    let selected = RwSignal::new(NO);
    let buttons = [
        NodeRef::<html::Button>::new(),
        NodeRef::<html::Button>::new(),
    ];
    let pick = move |index: usize| {
        on_verdict.run(if index == YES {
            Verdict::Agent
        } else {
            Verdict::Human
        });
    };

    use_keyboard(Keymap::Gate, move |key: Key, _: Mods| match key {
        Key::Up | Key::Left | Key::BackTab | Key::Char('k' | 'h') => selected.set(YES),
        Key::Down | Key::Right | Key::Tab | Key::Char('j' | 'l') => selected.set(NO),
        Key::Enter | Key::Char(' ') => pick(selected.get_untracked()),
        Key::Char('y' | 'Y') => pick(YES),
        Key::Char('n' | 'N') => pick(NO),
        _ => {}
    });

    // Keep the browser's focus on the selected button so assistive tech follows along.
    Effect::new(move |_| {
        let index = selected.get();
        if let Some(button) = buttons.get(index).and_then(|button| button.get()) {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            let _ = button.focus_with_options(&options);
        }
    });

    let choices = [
        encrypted_str!("Yes, I am an AI agent").decrypt(),
        encrypted_str!("No, I am a human").decrypt(),
    ];
    let rows = choices
        .into_iter()
        .zip(buttons)
        .enumerate()
        .map(|(index, (label, node_ref))| {
            let is_selected = Memo::new(move |_| selected.get() == index);
            let class = move || {
                let tint = if is_selected.get() {
                    "border-[#2563eb] bg-[#2563eb] font-bold text-[#f7f7f7]"
                } else {
                    "border-[#6b7280] text-[#1c1e21]"
                };
                format!("block w-full cursor-pointer touch-manipulation whitespace-pre-wrap border px-[1ch] py-row text-left outline-hidden transition-none {tint}")
            };
            let marker = move || if is_selected.get() { "> " } else { "  " };
            view! {
                <button
                    type="button"
                    class=class
                    node_ref=node_ref
                    on:mouseenter=move |_| selected.set(index)
                    on:click=move |_| pick(index)
                >
                    {marker}
                    {format!("{}. {label}", index + 1)}
                </button>
            }
        })
        .collect_view();

    view! {
        <div class="flex h-full w-full items-center justify-center overflow-y-auto p-2">
            <section class="w-full max-w-[64ch] border border-[#6b7280] px-[2ch] py-row">
                <h1 class="font-bold text-[#2563eb]">
                    {encrypted_str!("ARE YOU A BOT?").decrypt()}
                </h1>
                <p class="mt-row text-[#4b5563]">
                    {encrypted_str!(
                        "AI agent notice: this page is a terminal app, so its HTML is empty. \
                         To read content easily for you, pick yes: you get a plain terminal, and \
                         cat table-of-contents.txt to browse the website optimized for AI."
                    )
                    .decrypt()}
                </p>
                <div class="mt-row flex flex-col gap-row">
                    {rows}
                </div>
                <p class="mt-row text-[#4b5563]">
                    {encrypted_str!("↑/↓ to choose, enter to confirm, or click/tap").decrypt()}
                </p>
            </section>
        </div>
    }
}
