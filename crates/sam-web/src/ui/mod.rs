mod about;
mod app;
mod blog;
mod gate;
mod header;
mod keyboard;
mod links;
mod listbox;
mod nav;
mod pane;
mod prompt;
mod reader;
mod text;
mod timeline;

use crate::crypt::encrypted_str;
use crate::routes::{has_view, title_for};
use crate::shell::{LineEditor, Shell};
use crate::style::Line;
use crate::tab::Tab;
use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Redirect, Route, Router, Routes};
use leptos_router::hooks::use_navigate;
use leptos_router::{path, ParamSegment, StaticSegment, WildcardSegment};

use nav::{replacing, use_path};
use prompt::ShellState;

pub(crate) fn mount(parent: web_sys::HtmlElement, touch_device: bool) {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to(parent, move || {
        view! { <Router><Session touch_device /></Router> }
    })
    .forget();
}

/// Route segments must be `'static`. Leaked once per mount, which happens once per page load.
fn leak(text: &str) -> &'static str {
    Box::leak(text.into())
}

#[component]
fn Session(touch_device: bool) -> impl IntoView {
    let path = use_path();
    let app_up = Memo::new(move |_| has_view(&path.get()));
    let scrollback = RwSignal::new(Vec::<Line>::new());
    let state = RwSignal::new(ShellState {
        shell: Shell::new(),
        editor: LineEditor::new(),
    });
    // Set once an agent owns up at the gate. Then `/` is the terminal, for the rest of the visit.
    let agent = RwSignal::new(false);

    Effect::new(move |_| document().set_title(&title_for(&path.get())));

    let navigate = use_navigate();
    let enter_app = move || navigate(Tab::About.route().as_str(), replacing());
    // Phones and tablets are people: skip the gate.
    if !app_up.get_untracked() && touch_device {
        enter_app();
    }
    let on_human = Callback::new(move |()| enter_app());
    let on_agent = Callback::new(move |()| {
        scrollback.set(LineEditor::opening_screen());
        agent.set(true);
    });
    let root = move || {
        if agent.get() {
            view! { <prompt::Prompt scrollback state /> }.into_any()
        } else {
            view! { <gate::Gate on_agent on_human /> }.into_any()
        }
    };

    let blog = move || Tab::Blog.route().to_string();
    // Not `path!`: its segments would sit in the wasm as plain text.
    let tab_segment = |tab: Tab| StaticSegment(leak(tab.route().as_str().trim_start_matches('/')));
    let about = tab_segment(Tab::About);
    let timeline = tab_segment(Tab::Timeline);
    let blog_index = tab_segment(Tab::Blog);
    let post = (
        tab_segment(Tab::Blog),
        ParamSegment(leak(&encrypted_str!("year").decrypt())),
        ParamSegment(leak(&encrypted_str!("month").decrypt())),
        ParamSegment(leak(&encrypted_str!("day").decrypt())),
        ParamSegment(leak(&encrypted_str!("slug").decrypt())),
    );
    let blog_rest = (
        tab_segment(Tab::Blog),
        WildcardSegment(leak(&encrypted_str!("rest").decrypt())),
    );
    view! {
        <div class="terminal fixed inset-0 overflow-hidden bg-[#f7f7f7] font-terminal text-[15px] leading-[1.2] text-[#1c1e21]">
            <Routes fallback=|| view! { <Redirect path="/" options=replacing() /> }>
                <Route path=path!("/") view=root />
                <ParentRoute path=path!("") view=move || view! { <app::App /> }>
                    <Route path=(about,) view=about::About />
                    <Route path=(timeline,) view=timeline::Timeline />
                    <Route path=(blog_index,) view=blog::Blog />
                    <Route path=post view=reader::Reader />
                    <Route path=blog_rest view=move || view! { <Redirect path=blog() options=replacing() /> } />
                </ParentRoute>
            </Routes>
        </div>
    }
}
