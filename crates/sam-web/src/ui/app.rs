//! Inside `.terminal` one Tailwind spacing unit is `1ch` and `row` is `1lh`
//! (see `common.css`), so every measure here is in characters and lines.

use crate::keys::{Key, Mods};
use crate::routes::screen_at;
use crate::site_path::SitePath;
use crate::tab::Tab;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use leptos_router::hooks::use_navigate;

use super::header::Header;
use super::keyboard::AppKeys;
use super::nav::{replacing, use_path, use_show};
use super::pane::ScrollPositions;

/// In context while the app is up: links to its views navigate in-app.
#[derive(Clone, Copy)]
pub(super) struct InApp;

/// Held by the app so it survives the list unmounting.
#[derive(Clone, Copy)]
pub(super) struct Lists {
    pub(super) timeline: RwSignal<usize>,
    pub(super) blog: RwSignal<usize>,
}

/// Last pointer position in screen coordinates. Shared across lists so a tab
/// switch under a resting pointer is not taken for movement.
#[derive(Clone, Copy)]
pub(super) struct Pointer(pub(super) StoredValue<(i32, i32)>);

#[component]
pub(super) fn App(touch_device: bool) -> impl IntoView {
    provide_context(InApp);
    provide_context(Lists {
        timeline: RwSignal::new(0),
        blog: RwSignal::new(0),
    });
    provide_context(ScrollPositions(StoredValue::new([0.0; Tab::ALL.len()])));
    provide_context(Pointer(StoredValue::new((i32::MIN, i32::MIN))));

    let path = use_path();
    let tab = Memo::new(move |_| screen_at(&path.get()).map_or(Tab::About, |screen| screen.tab()));
    let show = use_show();
    let navigate = use_navigate();
    let quit = move || navigate(SitePath::root().as_str(), replacing());
    let switch_tab = move |next: Tab| show.run(next.route());
    provide_context(AppKeys::new(Callback::new(
        move |(key, mods): (Key, Mods)| {
            if mods.ctrl {
                if matches!(key, Key::Char('c' | 'd')) {
                    quit();
                }
                return;
            }
            match key {
                Key::Left | Key::Char('h') | Key::BackTab => switch_tab(tab.get_untracked().prev()),
                Key::Right | Key::Char('l') | Key::Tab => switch_tab(tab.get_untracked().next()),
                Key::Char('?') => switch_tab(Tab::Help),
                Key::Char('q') => quit(),
                Key::Char(c @ '1'..='4') => switch_tab(Tab::ALL[c as usize - '1' as usize]),
                _ => {}
            }
        },
    )));

    view! {
        <div class="flex h-full w-full flex-col">
            <Header touch_device />
            <main class="flex min-h-0 w-full flex-1 flex-col">
                <div class="min-h-0 w-full flex-1 px-2">
                    <Outlet />
                </div>
            </main>
        </div>
    }
}
