//! The wasm entry point: the host loads the bundle, hands it a mount element
//! and whether this is a touch device, and the session takes it from there.

use wasm_bindgen::prelude::wasm_bindgen;

/// Mounts the site into `element`.
#[wasm_bindgen(js_name = start)]
pub fn sam_start(element: web_sys::HtmlElement, touch_device: bool) {
    crate::ui::mount(element, touch_device);
}
