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
use crate::site_path::SitePath;
use crate::style::Line;
use crate::tab::Tab;
use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Redirect, Route, Router, Routes};
use leptos_router::hooks::use_navigate;
use leptos_router::{path, ParamSegment, StaticSegment, WildcardSegment};

use gate::Verdict;
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
    // The gate stands in front of every URL until it has an answer, which is kept across visits.
    // Phones and tablets are people: they skip it.
    let verdict = RwSignal::new(if touch_device {
        Some(Verdict::Human)
    } else {
        gate::remembered_verdict()
    });
    if verdict.get_untracked() == Some(Verdict::Agent) {
        scrollback.set(LineEditor::opening_screen());
    }
    let on_verdict = Callback::new(move |answer: Verdict| {
        gate::remember_verdict(answer);
        if answer == Verdict::Agent {
            scrollback.set(LineEditor::opening_screen());
        }
        verdict.set(Some(answer));
    });

    Effect::new(move |_| document().set_title(&title_for(&path.get())));

    // Agents only get the terminal, which lives at `/`.
    let navigate = use_navigate();
    Effect::new(move |_| {
        if verdict.get() == Some(Verdict::Agent) && app_up.get() {
            navigate(SitePath::root().as_str(), replacing());
        }
    });

    let about_route = move || Tab::About.route().to_string();
    let blog = move || Tab::Blog.route().to_string();
    // Not `path!`: its segments would sit in the wasm as plain text.
    let tab_segment = |tab: Tab| StaticSegment(leak(tab.route().as_str().trim_start_matches('/')));
    let about_index = tab_segment(Tab::About);
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
    let site = move || {
        match verdict.get() {
        None => view! { <gate::Gate on_verdict /> }.into_any(),
        Some(Verdict::Agent) => view! { <prompt::Prompt scrollback state /> }.into_any(),
        Some(Verdict::Human) => view! {
            <Routes fallback=|| view! { <Redirect path="/" options=replacing() /> }>
                // `/` is the gate's spot, and humans are past it.
                <Route path=path!("/") view=move || view! { <Redirect path=about_route() options=replacing() /> } />
                <ParentRoute path=path!("") view=move || view! { <app::App /> }>
                    <Route path=(about_index,) view=about::About />
                    <Route path=(timeline,) view=timeline::Timeline />
                    <Route path=(blog_index,) view=blog::Blog />
                    <Route path=post view=reader::Reader />
                    <Route path=blog_rest view=move || view! { <Redirect path=blog() options=replacing() /> } />
                </ParentRoute>
            </Routes>
        }
        .into_any(),
    }
    };
    view! {
        <div class="terminal fixed inset-0 overflow-hidden bg-[#f7f7f7] font-terminal text-[15px] leading-[1.2] text-[#1c1e21]">
            {site}
        </div>
    }
}
