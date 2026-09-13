//! The developersam.com app: a Leptos site set in the terminal's monospace
//! type, compiled to wasm and hosted by the Next.js site. The URL is the
//! state; [`ui`] is the component tree under the router.

mod crypt;
mod data;
mod highlight;
mod keys;
mod markdown;
mod posts;
mod routes;
mod shell;
mod site_path;
mod style;
mod tab;
mod theme;
mod ui;

use wasm_bindgen::prelude::wasm_bindgen;

/// The wasm entry point: the host loads the bundle, hands it a mount element
/// and whether this is a touch device, and the session takes it from there.
#[wasm_bindgen(js_name = "start")]
pub fn sam_start(element: web_sys::HtmlElement, touch_device: bool) {
    ui::mount(element, touch_device);
}
