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
