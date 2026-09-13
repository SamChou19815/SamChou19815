//! The component tree under the router.

mod about;
mod app;
mod blog;
mod header;
mod help;
mod keyboard;
mod listbox;
mod nav;
mod pane;
mod prompt;
mod reader;
mod text;
mod timeline;

use crate::routes::{has_view, title_for};
use crate::shell::{LineEditor, Shell};
use crate::style::Line;
use crate::tab::Tab;
use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Redirect, Route, Router, Routes};
use leptos_router::hooks::use_navigate;
use leptos_router::path;

use nav::{replacing, use_path};
use prompt::ShellState;

pub(crate) fn mount(parent: web_sys::HtmlElement, touch_device: bool) {
    console_error_panic_hook::set_once();
    // Dropping the mount handle would tear the site down.
    leptos::mount::mount_to(parent, move || {
        view! { <Router><Session touch_device /></Router> }
    })
    .forget();
}

/// The route table: the prompt at `/`, the app over everything else.
#[component]
fn Session(touch_device: bool) -> impl IntoView {
    let path = use_path();
    let app_up = Memo::new(move |_| has_view(&path.get()));
    let scrollback = RwSignal::new(Vec::<Line>::new());
    let state = RwSignal::new(ShellState {
        shell: Shell::new(),
        editor: LineEditor::new(),
    });

    Effect::new(move |_| document().set_title(&title_for(&path.get())));

    // A touch device has no Enter to press, so it skips the prompt.
    if !app_up.get_untracked() {
        if touch_device {
            use_navigate()(Tab::About.route().as_str(), replacing());
        } else {
            let lines = state
                .try_update(|state| state.editor.opening_screen())
                .unwrap_or_default();
            scrollback.set(lines);
        }
    }

    // Any exit from the app (`q`, Ctrl+C, back button) prints the exit line.
    Effect::watch(
        move || app_up.get(),
        move |up, was_up, _| {
            if !*up && was_up == Some(&true) {
                state.update(|state| state.editor = LineEditor::new());
                scrollback.update(|lines| lines.extend(LineEditor::after_dev_sam_app_exit()));
            }
        },
        false,
    );

    let blog = move || Tab::Blog.route().to_string();
    view! {
        // Pins its own text color: the site body is `dark:text-gray-200`.
        <div class="terminal fixed inset-0 overflow-hidden bg-[#f7f7f7] font-terminal text-[15px] leading-[1.2] text-[#1c1e21]">
            <Routes fallback=|| view! { <Redirect path="/" options=replacing() /> }>
                <Route path=path!("/") view=move || view! { <prompt::Prompt scrollback state /> } />
                <ParentRoute path=path!("") view=move || view! { <app::App touch_device /> }>
                    <Route path=path!("about") view=about::About />
                    <Route path=path!("timeline") view=timeline::Timeline />
                    <Route path=path!("blog") view=blog::Blog />
                    <Route path=path!("blog/:year/:month/:day/:slug") view=reader::Reader />
                    <Route path=path!("blog/*rest") view=move || view! { <Redirect path=blog() options=replacing() /> } />
                    <Route path=path!("help") view=help::Help />
                </ParentRoute>
            </Routes>
        </div>
    }
}
