//! The keyboard, top to bottom: the window-level listener, and the app-wide
//! keys every view falls back to.

use crate::keys::{claims, map_key, Key, Keymap, Mods};
use leptos::prelude::*;

/// A window-level keydown listener for the life of the component, so keys
/// arrive wherever focus happens to be.
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

/// Handler for the keys that mean the same on every view: tab switching
/// (arrows, h/l, Tab, 1–4, ?) and quitting (q, Ctrl+C/D). Provided by [`App`]
/// (see [`super::app`]) and reached through [`use_view_keys`].
#[derive(Clone, Copy)]
pub(super) struct AppKeys(Callback<(Key, Mods)>);

impl AppKeys {
    pub(super) fn new(handle: Callback<(Key, Mods)>) -> Self {
        AppKeys(handle)
    }
}

/// The view in front gets every key first; what it returns `false` for, and
/// every Ctrl combo, goes to the app.
pub(super) fn use_view_keys(handle: impl Fn(Key, Mods) -> bool + 'static) {
    let AppKeys(app) = expect_context::<AppKeys>();
    use_keyboard(Keymap::App, move |key, mods| {
        if mods.ctrl || !handle(key, mods) {
            app.run((key, mods));
        }
    });
}
