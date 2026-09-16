use crate::routes::{link_target, LinkTarget};
use crate::site_path::SitePath;
use leptos::prelude::*;
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::NavigateOptions;

use super::app::InApp;

pub(super) fn use_path() -> Memo<SitePath> {
    let location = use_location();
    Memo::new(move |_| {
        location
            .pathname
            .with(|path| SitePath::parse(path).unwrap_or_else(SitePath::root))
    })
}

pub(super) fn use_show() -> Callback<SitePath> {
    let navigate = use_navigate();
    Callback::new(move |path: SitePath| navigate(path.as_str(), showing()))
}

/// `scroll: false` since the panes scroll, not the document.
fn showing() -> NavigateOptions {
    NavigateOptions {
        scroll: false,
        ..NavigateOptions::default()
    }
}

/// For entering/leaving the app, so the back button skips the prompt.
pub(super) fn replacing() -> NavigateOptions {
    NavigateOptions {
        replace: true,
        ..showing()
    }
}

/// http(s) only, never `javascript:`.
pub(super) fn open_url(url: &str) {
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("https://") && !lower.starts_with("http://") {
        return;
    }
    let window = web_sys::window().expect("window");
    let _ = window.open_with_url_and_target_and_features(url, "_blank", "noopener");
}

/// Stops click propagation so an enclosing card doesn't also handle it.
#[component]
pub(super) fn Link(
    url: String,
    #[prop(into)] class: String,
    #[prop(optional, into)] style: String,
    children: Children,
) -> impl IntoView {
    let in_app = use_context::<InApp>().is_some();
    let anchor_class = format!("transition-none {class}");
    match link_target(&url, in_app) {
        LinkTarget::View(path) => {
            let show = use_show();
            let href = path.to_string();
            let on_click = move |event: web_sys::MouseEvent| {
                event.stop_propagation();
                // Let the browser handle open-in-new-tab etc.
                if event.meta_key() || event.ctrl_key() || event.shift_key() || event.alt_key() {
                    return;
                }
                event.prevent_default();
                show.run(path.clone());
            };
            view! {
                <a href=href class=anchor_class style=style on:click=on_click>
                    {children()}
                </a>
            }
            .into_any()
        }
        LinkTarget::External(url) => view! {
            <a
                href=url
                target="_blank"
                rel="noopener"
                class=anchor_class
                style=style
                on:click=|event: web_sys::MouseEvent| event.stop_propagation()
            >
                {children()}
            </a>
        }
        .into_any(),
        LinkTarget::Ignore => view! {
            <span class=format!("cursor-pointer {class}") style=style>{children()}</span>
        }
        .into_any(),
    }
}

/// Must be called synchronously from the key handler, or `window.open` gets popup-blocked.
pub(super) fn use_open_link() -> impl Fn(&str) + Copy {
    let show = use_show();
    move |url: &str| match link_target(url, true) {
        LinkTarget::View(path) => show.run(path),
        LinkTarget::External(url) => open_url(&url),
        LinkTarget::Ignore => {}
    }
}
