use crate::style::{Line, Span};
use crate::theme;
use leptos::prelude::*;
use std::fmt::Write as _;

use super::nav::Link;

pub(super) fn style_of(color: theme::Color) -> String {
    format!("color:{};", color.css())
}

#[component]
pub(super) fn Run(span: Span) -> impl IntoView {
    let mut class = String::new();
    let mut style = String::new();
    if let Some(color) = span.style.color {
        let _ = write!(style, "color:{};", color.css());
    }
    if span.style.bold {
        class.push_str(" font-bold");
    }
    if span.style.italic {
        class.push_str(" italic");
    }
    if span.style.underline {
        class.push_str(" underline decoration-1 underline-offset-2");
    }
    let text = span.text;
    match span.link {
        Some(url) => {
            class.push_str(" text-inherit");
            view! { <Link url class style>{text}</Link> }.into_any()
        }
        None => view! { <span class=class style=style>{text}</span> }.into_any(),
    }
}

pub(super) fn runs(line: &Line) -> impl IntoView {
    line.iter()
        .cloned()
        .map(|span| view! { <Run span /> })
        .collect_view()
}

#[component]
pub(super) fn StyledLine(line: Line, wraps: bool) -> impl IntoView {
    let class = if wraps {
        "min-h-row whitespace-pre-wrap break-words"
    } else {
        "min-h-row whitespace-pre"
    };
    view! { <div class=class>{runs(&line)}</div> }
}

#[component]
pub(super) fn HangingLine(line: Line, hang: usize) -> impl IntoView {
    let style = format!("padding-left:{hang}ch;text-indent:-{hang}ch");
    view! { <div class="min-h-row whitespace-pre-wrap break-words" style=style>{runs(&line)}</div> }
}
