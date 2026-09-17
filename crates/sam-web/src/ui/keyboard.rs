use crate::keys::{claims, map_key, Key, Keymap, Mods};
use leptos::prelude::*;

/// Window-level so focus doesn't matter.
pub(super) fn use_keyboard(keymap: Keymap, handle: impl Fn(Key, Mods) + 'static) {
    let listener = window_event_listener(leptos::ev::keydown, move |event| {
        if event.meta_key() {
            return;
        }
        let Some((key, mods)) = map_key(&event) else {
            return;
        };
        if !claims(keymap, key, mods) {
            return;
        }
        event.prevent_default();
        handle(key, mods);
    });
    on_cleanup(move || listener.remove());
}

/// App-wide keys (tab switching, quit). Provided by [`super::app::App`].
#[derive(Clone, Copy)]
pub(super) struct AppKeys(Callback<(Key, Mods)>);

impl AppKeys {
    pub(super) fn new(handle: Callback<(Key, Mods)>) -> Self {
        AppKeys(handle)
    }
}

/// The view handles the key first. Unhandled keys and all Ctrl combos go to the app.
pub(super) fn use_view_keys(handle: impl Fn(Key, Mods) -> bool + 'static) {
    let AppKeys(app) = expect_context::<AppKeys>();
    use_keyboard(Keymap::App, move |key, mods| {
        if mods.ctrl || !handle(key, mods) {
            app.run((key, mods));
        }
    });
}
