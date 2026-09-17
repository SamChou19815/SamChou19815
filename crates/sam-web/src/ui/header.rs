use crate::crypt::{encrypted_str, EncryptedString};
use crate::tab::Tab;
use leptos::prelude::*;
use leptos_router::components::A;

const TITLE: EncryptedString = encrypted_str!("DEV SAM");
const TABS_LABEL: EncryptedString = encrypted_str!("Tabs");

const GLYPHS: [(char, [&str; 3]); 11] = [
    ('A', ["█▀█", "█▀█", "▀ ▀"]),
    ('D', ["█▀▄", "█ █", "▀▀ "]),
    ('E', ["█▀▀", "█▀ ", "▀▀▀"]),
    ('L', ["█  ", "█  ", "▀▀▀"]),
    ('M', ["█▄█", "█ █", "▀ ▀"]),
    ('O', ["█▀█", "█ █", "▀▀▀"]),
    ('P', ["█▀█", "█▀▀", "▀  "]),
    ('R', ["█▀█", "█▀▄", "▀ ▀"]),
    ('S', ["█▀▀", "▀▀█", "▀▀▀"]),
    ('V', ["█ █", "█ █", " ▀ "]),
    (' ', [" ", " ", " "]),
];

fn banner_rows(text: &str) -> [String; 3] {
    let mut rows: [String; 3] = Default::default();
    for character in text.chars() {
        let glyph = &GLYPHS
            .iter()
            .find(|(letter, _)| *letter == character)
            .unwrap_or_else(|| panic!("{character} is not a letter the wordmark spells"))
            .1;
        for (row, cells) in rows.iter_mut().zip(glyph) {
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(cells);
        }
    }
    rows
}

#[component]
fn BlockCell(character: char) -> impl IntoView {
    let fill = match character {
        '█' => "bg-[#2563eb]",
        '▀' => "bg-[linear-gradient(#2563eb_50%,transparent_50%)]",
        '▄' => "bg-[linear-gradient(to_bottom,transparent_50%,#2563eb_50%)]",
        _ => "",
    };
    view! { <span class=format!("inline-block h-row w-[1ch] {fill}")></span> }
}

#[component]
fn Wordmark() -> impl IntoView {
    let rows = banner_rows(&TITLE.decrypt())
        .into_iter()
        .map(|row| {
            let cells = row
                .chars()
                .map(|character| view! { <BlockCell character /> })
                .collect_view();
            view! { <div class="flex">{cells}</div> }
        })
        .collect_view();
    view! { <div class="text-[#2563eb]">{rows}</div> }
}

#[component]
fn TabLink(tab: Tab, touch_device: bool) -> impl IntoView {
    let shown = if touch_device && tab == Tab::Help {
        "hidden aria-[current=page]:inline"
    } else {
        "hidden @min-[36ch]:inline aria-[current=page]:inline"
    };
    let class = format!(
        "{shown} transition-none text-[#374151] aria-[current=page]:font-bold aria-[current=page]:text-[#1e3a8a]"
    );
    let label = tab.name().decrypt();
    view! {
        <A href=tab.route().to_string() scroll=false {..} class=class>
            {label}
        </A>
    }
}

#[component]
pub(super) fn Header(touch_device: bool) -> impl IntoView {
    let tabs = Tab::ALL
        .iter()
        .map(|&tab| view! { <TabLink tab touch_device /> })
        .collect_view();
    view! {
        <header class="shrink-0 border-b border-[#d1d5db]">
            <div class="px-2">
                <div class="mx-auto w-full max-w-88 pt-row">
                    <div class="flex flex-wrap items-center">
                        <div class="hidden [@media(min-width:542px)_and_(min-height:432px)]:@min-[60ch]:block">
                            <Wordmark />
                        </div>
                        <nav
                            class="flex min-w-max flex-1 items-center justify-end gap-3 @max-[67ch]:justify-start"
                            aria-label=TABS_LABEL.decrypt()
                        >
                            {tabs}
                        </nav>
                    </div>
                    <div class="h-row shrink-0"></div>
                </div>
            </div>
        </header>
    }
}
