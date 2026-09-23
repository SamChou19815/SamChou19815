use crate::keys::Key;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

fn links(pane: &web_sys::Element) -> Vec<web_sys::HtmlElement> {
    let Ok(anchors) = pane.query_selector_all("a[href]") else {
        return Vec::new();
    };
    (0..anchors.length())
        .filter_map(|index| anchors.get(index)?.dyn_into::<web_sys::HtmlElement>().ok())
        .collect()
}

fn focused(links: &[web_sys::HtmlElement]) -> Option<usize> {
    let active = web_sys::window()?.document()?.active_element()?;
    links
        .iter()
        .position(|link| link.is_same_node(Some(&active)))
}

pub(super) fn link_keys(pane: NodeRef<html::Div>, key: Key) -> bool {
    let step = match key {
        Key::Tab => 1,
        Key::BackTab => -1,
        Key::Enter => 0,
        _ => return false,
    };
    let Some(pane) = pane.get_untracked() else {
        return true;
    };
    let links = links(&pane);
    let focused = focused(&links);
    if step == 0 {
        // Synchronous within the keydown, so an external link's new tab isn't popup-blocked.
        if let Some(link) = focused.and_then(|index| links.get(index)) {
            link.click();
        }
        return true;
    }
    let Some(last) = links.len().checked_sub(1) else {
        return true;
    };
    let next = match focused {
        Some(index) if step > 0 && index < last => index + 1,
        Some(_) if step > 0 => 0,
        Some(index) => index.checked_sub(1).unwrap_or(last),
        None if step > 0 => 0,
        None => last,
    };
    if let Some(link) = links.get(next) {
        let _ = link.focus();
    }
    true
}
