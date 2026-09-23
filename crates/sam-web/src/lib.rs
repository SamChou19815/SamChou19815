// Every panic site bakes its source path into the wasm (`src/ui/about.rs`, ...), which maps out
// the site. Stable Rust can't strip those, so the app doesn't panic. Tests may.
#![cfg_attr(
    not(test),
    deny(
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::integer_division_remainder_used,
        clippy::panic,
        clippy::string_slice,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::unwrap_used
    )
)]

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

#[wasm_bindgen(js_name = "start")]
pub fn sam_start(element: web_sys::HtmlElement, touch_device: bool) {
    ui::mount(element, touch_device);
}
