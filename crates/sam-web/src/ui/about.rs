//! The About tab: the samlang source that writes itself.

use crate::highlight;
use crate::style::Line;
use crate::tab::Tab;
use leptos::html;
use leptos::prelude::*;

use super::keyboard::use_view_keys;
use super::pane::{scroll_keys, Pane};
use super::text::HangingLine;

#[component]
pub(super) fn About() -> impl IntoView {
    let pane = NodeRef::<html::Div>::new();
    use_view_keys(move |key, _| scroll_keys(pane, key));
    let mut lines = highlight::doc_comment_lines();
    lines.push(Line::new());
    lines.extend(highlight::program_lines());
    view! {
        <Pane tab=Tab::About node_ref=pane>
            <CodeListing lines />
        </Pane>
    }
}

/// samlang's indent width.
const INDENT: usize = 2;

fn hang_of(line: &Line) -> usize {
    let text: String = line.iter().map(|span| span.text.as_str()).collect();
    let indent = text.len() - text.trim_start().len();
    let gutter = if text[indent..].starts_with("* ") {
        2
    } else {
        0
    };
    indent + gutter + INDENT
}

/// Wraps under the indent rather than scrolling sideways: phones show no scrollbar.
#[component]
fn CodeListing(lines: Vec<Line>) -> impl IntoView {
    let rows = lines
        .into_iter()
        .map(|line| {
            let hang = hang_of(&line);
            view! { <HangingLine line hang /> }
        })
        .collect_view();
    view! {
        <div class="mx-auto w-fit max-w-full">
            <div class="h-row shrink-0"></div>
            <div>{rows}</div>
        </div>
    }
}
