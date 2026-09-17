use crate::keys::{Key, Keymap, Mods};
use crate::shell::{self, EditOutcome, LineEditor, Shell};
use crate::style::{Line, Span, TextStyle};
use crate::tab::Tab;
use leptos::html;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use super::keyboard::use_keyboard;
use super::nav::replacing;
use super::pane::SCROLL;
use super::text::{runs, StyledLine};

/// Owned by the session, not the prompt, so it survives a run of the app.
#[derive(Clone)]
pub(super) struct ShellState {
    pub(super) shell: Shell,
    pub(super) editor: LineEditor,
}

fn after_render(work: impl FnOnce() + 'static) {
    leptos::task::spawn_local(async move {
        leptos::task::tick().await;
        work();
    });
}

fn frozen_prompt_line(snapshot: &shell::PromptRow) -> Line {
    let mut contents = snapshot.prompt.clone();
    contents.extend(snapshot.before_cursor.iter().cloned());
    if let Some(character) = snapshot.at_cursor {
        contents.push(Span::styled(character.to_string(), TextStyle::default()));
    }
    contents.extend(snapshot.after_cursor.iter().cloned());
    contents
}

fn interrupted_prompt_line(snapshot: &shell::PromptRow) -> Line {
    let mut contents = snapshot.prompt.clone();
    let mut line: String = snapshot
        .before_cursor
        .iter()
        .map(|span| span.text.clone())
        .collect();
    let mut rest: String = snapshot.at_cursor.map(String::from).unwrap_or_default();
    for span in &snapshot.after_cursor {
        rest.push_str(&span.text);
    }
    let mut rest = rest.chars();
    let mut replaced = String::from("^C");
    rest.next();
    replaced.extend(rest);
    line.push_str(&replaced);
    contents.push(Span::new(line));
    contents
}

#[component]
fn PromptRow(snapshot: shell::PromptRow) -> impl IntoView {
    let cursor_character = snapshot
        .at_cursor
        .map(String::from)
        .unwrap_or_else(|| "\u{00a0}".to_string());
    view! {
        <div class="min-h-row whitespace-pre-wrap break-words">
            {runs(&snapshot.prompt)}
            {runs(&snapshot.before_cursor)}
            <span class="animate-cursor-blink bg-[#1c1e21] text-[#f7f7f7]">{cursor_character}</span>
            {runs(&snapshot.after_cursor)}
        </div>
    }
}

#[component]
pub(super) fn Prompt(
    scrollback: RwSignal<Vec<Line>>,
    state: RwSignal<ShellState>,
) -> impl IntoView {
    let pane = NodeRef::<html::Div>::new();
    let navigate = use_navigate();

    use_keyboard(Keymap::Prompt, move |key: Key, mods: Mods| {
        // Snapshot before handle_key clears the line.
        let snapshot = state.with_untracked(|state| state.editor.prompt_row(&state.shell));
        let outcome = state
            .try_update(|state| state.editor.handle_key(key, mods, &mut state.shell))
            .unwrap_or(EditOutcome::None);
        match outcome {
            EditOutcome::None => {}
            EditOutcome::Output(lines) => {
                scrollback.update(|scrollback| {
                    scrollback.push(frozen_prompt_line(&snapshot));
                    if lines.is_empty() {
                        scrollback.push(Vec::new());
                    }
                    scrollback.extend(lines);
                });
            }
            EditOutcome::ClearScreen => scrollback.set(Vec::new()),
            EditOutcome::Launch => {
                scrollback.update(|scrollback| scrollback.push(frozen_prompt_line(&snapshot)));
                navigate(Tab::About.route().as_str(), replacing());
            }
            EditOutcome::Completion(candidates) => {
                scrollback.update(|scrollback| {
                    scrollback.push(frozen_prompt_line(&snapshot));
                    scrollback.push(vec![Span::new(candidates.join("   "))]);
                });
            }
            EditOutcome::Interrupt => {
                scrollback.update(|scrollback| scrollback.push(interrupted_prompt_line(&snapshot)));
            }
        }
    });

    Effect::new(move |_| {
        scrollback.track();
        state.track();
        after_render(move || {
            if let Some(pane) = pane.get_untracked() {
                pane.set_scroll_top(pane.scroll_height());
            }
        });
    });

    let lines = move || {
        scrollback.with(|lines| {
            lines
                .iter()
                .cloned()
                .map(|line| view! { <StyledLine line wraps=true /> })
                .collect_view()
        })
    };
    let row = move || {
        let snapshot = state.with(|state| state.editor.prompt_row(&state.shell));
        view! { <PromptRow snapshot /> }
    };
    view! {
        <div class=format!("{SCROLL} h-full w-full") node_ref=pane>
            {lines}
            {row}
        </div>
    }
}
