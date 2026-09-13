//! The Leptos front-end: one session that is either the `dev-sam-sh` prompt or
//! the app.
//!
//! There is nothing to measure here. The session is one `.terminal` box that
//! pins the type and the two colors the site is drawn in, and every view is
//! ordinary HTML laid out by Tailwind classes ([`screen`]); the lengths are
//! `ch` and `lh`, which the browser resolves against the real font and the
//! real viewport, so the browser does the fitting, the wrapping, the
//! truncating, the scrolling and the hit-testing.
//!
//! The app is one signal, and the views read it through memos of the pieces
//! they show ([`Model`]). An input is a call on the app that hands back what
//! the browser has to do for it ([`crate::Errand`]): a link opens while the
//! click is still being handled, and the pane is moved once the frame that
//! shows the new state has been drawn. The URL bar and the title are an
//! effect of the view the app is on, like anything else drawn from it.

pub mod screen;
use crate::hit::HitTarget;
use crate::shell::{self, EditOutcome, LineEditor, Shell};
use crate::site_path::SitePath;
use crate::style::{Line, Span, TextStyle};
use crate::{
    has_view, title_for, App, Errand, Key, Mods, Scroll, BLOG_TAB, SHELL_TITLE, TAB_COUNT,
    TIMELINE_TAB,
};
use leptos::html;
use leptos::prelude::*;
use std::fmt::Write as _;

/// Where the app was entered: the view the URL names, if any.
fn current_path() -> SitePath {
    let window = web_sys::window().expect("window");
    let path = window.location().pathname().expect("pathname");
    let trimmed = if path.len() > 1 {
        path.trim_end_matches('/')
    } else {
        &path
    };
    SitePath::parse(trimmed).unwrap_or_else(SitePath::root)
}

/// What the session is showing: the prompt, or the app.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Shell,
    App,
}

/// What a click means to the session: the one handler every element that
/// carries a [`HitTarget`] is given. Cheap to copy into each of them, and
/// safe to hold in a reactive closure, which is where a card's class reads it
/// from.
pub(crate) trait Activate: Fn(&HitTarget) + Copy + Send + Sync + 'static {}

impl<F: Fn(&HitTarget) + Copy + Send + Sync + 'static> Activate for F {}

/// The prompt row and the shell behind it: what a keystroke at the prompt
/// edits. The scrollback is kept apart, since a keystroke changes it only on
/// Enter.
#[derive(Clone)]
struct Prompt {
    shell: Shell,
    editor: LineEditor,
}

/// The app's state as the views read it: one memo per thing a view can depend
/// on, so a change to one — the selection, say — redraws the runs that show
/// it and nothing else. With it, the box the views draw into.
#[derive(Clone, Copy)]
pub(crate) struct Model {
    /// The tab the header marks.
    pub tab: Memo<usize>,
    /// The post in the reader, if one is open.
    pub reader: Memo<Option<usize>>,
    /// The selected card of the Timeline tab.
    pub timeline_selected: Memo<usize>,
    /// The selected card of the Blog tab.
    pub blog_selected: Memo<usize>,
    /// The pane: the box under the header the browser scrolls.
    pub pane: NodeRef<html::Div>,
    /// Where the pointer was last reported, in screen coordinates — nowhere
    /// until it has been. What tells a pointer that moved onto a card from
    /// one the pane slid under ([`screen::hover`]).
    pub pointer: StoredValue<(i32, i32)>,
}

/// Runs `work` once the frame showing the app's new state has been drawn. The
/// views are redrawn by tasks the reactive runtime queued when the state
/// changed, and its next tick comes after everything queued before it.
fn after_render(work: impl FnOnce() + 'static) {
    leptos::task::spawn_local(async move {
        leptos::task::tick().await;
        work();
    });
}

/// Mounts the session into `parent`. `touch_device` is the host's answer for
/// whether this is a phone or tablet — a host with no keyboard, where the app
/// runs itself rather than waiting at a prompt no one can type at.
pub fn mount(parent: web_sys::HtmlElement, touch_device: bool) {
    console_error_panic_hook::set_once();
    let path = current_path();
    // The session lives as long as the page: the mount handle is forgotten,
    // since dropping it would tear the site back down.
    leptos::mount::mount_to(parent, move || {
        view! { <Session path touch_device /> }
    })
    .forget();
}

// --- The classes the session is drawn with -------------------------------------------
//
// Styling is Tailwind, written at the point of use: the scanner reads these
// files straight out of the crate (the `@source` line in
// `packages/www/src/lib/common.css`), so a class named here is a class the
// stylesheet ships. Only a run's color is left to an inline style — the palette
// picks it as the view is built — and `.terminal`, in `common.css`, is what
// turns the Tailwind spacing scale into the type's own measures: one unit is
// one character cell, and `row` is one line of the type.

/// A box the browser scrolls. Tailwind has no scrollbar utilities, so the two
/// standard properties and the WebKit pseudo-elements are written as arbitrary
/// ones: a slim bar with a neutral gray thumb at 40%, and at 70% under the
/// pointer.
///
/// A box styled with this class is the one that overflows, so it has to be
/// pinned to less room than its content — `h-full` under a parent whose height
/// is definite (a fixed box, or a flex child that `flex-1 min-h-0` has sized),
/// `w-full` under a full-width one. Left at its natural `auto` height it grows
/// to fit its content, nothing overflows, and nothing can scroll.
pub(crate) const SCROLL: &str = concat!(
    "overflow-x-hidden overflow-y-auto ",
    "[scrollbar-width:thin] [scrollbar-color:#64656666_transparent] ",
    "[&::-webkit-scrollbar]:w-3.5 ",
    "[&::-webkit-scrollbar-track]:bg-transparent ",
    "[&::-webkit-scrollbar-thumb]:rounded-full ",
    "[&::-webkit-scrollbar-thumb]:bg-[#64656666] ",
    "[&::-webkit-scrollbar-thumb:hover]:bg-[#646566b3]",
);

// --- Browser errands ---------------------------------------------------------------

/// Only http(s) leaves: the page renders whatever bytes reach it, and a
/// `javascript:` URL would run in this document.
fn open_url(url: &str) {
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("https://") && !lower.starts_with("http://") {
        return;
    }
    let window = web_sys::window().expect("window");
    let _ = window.open_with_url_and_target_and_features(url, "_blank", "noopener");
}

/// Puts the URL bar and the document title on a view. A path the bar is
/// already on is left alone — the back button put it there, and pushing it
/// again would bury the entry the visitor came back to.
fn sync_url_bar(replace: bool, path: &SitePath, title: &str) {
    let window = web_sys::window().expect("window");
    let history = window.history().expect("history");
    if path.as_str() != current_path().as_str() {
        let result = if replace {
            history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(path.as_str()))
        } else {
            history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(path.as_str()))
        };
        let _ = result;
    }
    if let Some(document) = window.document() {
        document.set_title(title);
    }
}

/// One row of the type, as the pane is actually set at — read off it, since
/// `line-height` is what a row is.
fn line_height(pane: &web_sys::Element) -> f64 {
    web_sys::window()
        .and_then(|window| window.get_computed_style(pane).ok().flatten())
        .and_then(|style| style.get_property_value("line-height").ok())
        .and_then(|value| value.trim().trim_end_matches("px").parse::<f64>().ok())
        .filter(|height| *height > 0.0)
        .unwrap_or(18.0)
}

/// Moves the pane, in the units the keystroke asked for.
fn apply_scroll(pane: &web_sys::Element, by: Scroll) {
    let viewport = f64::from(pane.client_height());
    match by {
        Scroll::Rows(rows) => {
            pane.scroll_by_with_x_and_y(0.0, f64::from(rows) * line_height(pane));
        }
        // A screenful less two rows, so the lines the eye was on are still
        // there after the jump.
        Scroll::Pages(pages) => {
            let row = line_height(pane);
            let page = (viewport - 2.0 * row).max(row);
            pane.scroll_by_with_x_and_y(0.0, f64::from(pages) * page);
        }
        Scroll::Top => pane.set_scroll_top(0),
        Scroll::Bottom => pane.set_scroll_top(pane.scroll_height()),
    }
}

/// Maps a DOM keyboard event to the app's key model. `None` for keys the app
/// never sees.
fn map_key(event: &web_sys::KeyboardEvent) -> Option<(Key, Mods)> {
    let mods = Mods {
        ctrl: event.ctrl_key(),
        shift: event.shift_key(),
        alt: event.alt_key(),
    };
    let key = match event.key().as_str() {
        "ArrowUp" => Key::Up,
        "ArrowDown" => Key::Down,
        "ArrowLeft" => Key::Left,
        "ArrowRight" => Key::Right,
        "Enter" => Key::Enter,
        "Escape" => Key::Esc,
        "Tab" if mods.shift => Key::BackTab,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Delete" => Key::Delete,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "Home" => Key::Home,
        "End" => Key::End,
        other => {
            let mut chars = other.chars();
            let only = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            Key::Char(only)
        }
    };
    Some((key, mods))
}

/// Whether a keystroke is the session's to answer. A plain key always is. A key
/// held with a modifier is only the session's when it names something it
/// actually does: `Ctrl+C` and the line editor's own bindings. Everything else
/// is left to the browser, which is where a reload or a new tab has to keep
/// working.
fn claims(mode: Mode, key: Key, mods: Mods) -> bool {
    if mods.alt {
        return false;
    }
    if !mods.ctrl {
        return true;
    }
    match mode {
        // The bindings `LineEditor::handle_control_key` answers.
        Mode::Shell => matches!(key, Key::Char('c' | 'l' | 'a' | 'e' | 'u')),
        // The app's own way out ([`App::handle_key`]).
        Mode::App => matches!(key, Key::Char('c' | 'd')),
    }
}

// --- Styled runs ---------------------------------------------------------------------

/// One styled run as a `span`, with the click target it carries, if any. A
/// link is clicked on the words it is written on, and nowhere else.
pub(crate) fn span_view(span: &Span, on_activate: impl Activate) -> AnyView {
    let mut class = String::new();
    let mut style = String::new();
    // Every color is one the palette picks as the view is built, so it is the
    // one thing about a run that no class can be named for.
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
    let text = span.text.clone();
    match &span.link {
        Some(url) => {
            class.push_str(" cursor-pointer");
            let target = HitTarget::Link(url.clone());
            let on_click = move |event: web_sys::MouseEvent| {
                event.stop_propagation();
                on_activate(&target);
            };
            view! { <span class=class style=style on:click=on_click>{text}</span> }.into_any()
        }
        None => view! { <span class=class style=style>{text}</span> }.into_any(),
    }
}

/// One printed line as a block. The shell's scrollback and the reader's prose
/// wrap at the width they land at — at words, the way the browser wraps
/// everything; the reader's code blocks ask for `wraps: false` and scroll
/// instead, and the About listing wraps under its indent ([`hanging_line`]).
pub(crate) fn styled_line(line: &Line, wraps: bool, on_activate: impl Activate) -> AnyView {
    let runs: Vec<AnyView> = line
        .iter()
        .map(|span| span_view(span, on_activate))
        .collect();
    // A word too long for the whole line — a long URL, say — is broken rather
    // than let off the edge, which is the one habit worth keeping from the
    // columns this site used to be counted in.
    if wraps {
        view! { <div class="min-h-row whitespace-pre-wrap break-words">{runs}</div> }.into_any()
    } else {
        view! { <div class="min-h-row whitespace-pre">{runs}</div> }.into_any()
    }
}

/// One printed line that wraps under its own indent, the way an editor
/// soft-wraps code: the row is padded `hang` characters in and its first line
/// is pulled back out by as much, so what is left of a line too long for the
/// measure lands past the line's indent rather than back at the margin, and
/// still reads as the tail of the line above. A run too long to break at a
/// space — a URL — is broken where it runs out of room; it stays one run, so
/// a link is clickable on both of its rows.
pub(crate) fn hanging_line(line: &Line, hang: usize, on_activate: impl Activate) -> AnyView {
    let runs: Vec<AnyView> = line
        .iter()
        .map(|span| span_view(span, on_activate))
        .collect();
    // The hang is a length in characters, one that is the line's own, so it is
    // written on the element rather than named as a class.
    let style = format!("padding-left:{hang}ch;text-indent:-{hang}ch");
    view! { <div class="min-h-row whitespace-pre-wrap break-words" style=style>{runs}</div> }
        .into_any()
}

// --- Shell prompt rendering ----------------------------------------------------------

/// The prompt row as it was at the moment it froze: the prompt itself, then the
/// edited line with the character under the cursor put back in.
fn frozen_prompt_line(snapshot: &shell::PromptRow) -> Line {
    let mut contents = snapshot.prompt.clone();
    contents.extend(snapshot.before_cursor.iter().cloned());
    if let Some(character) = snapshot.at_cursor {
        contents.push(Span::styled(character.to_string(), TextStyle::default()));
    }
    contents.extend(snapshot.after_cursor.iter().cloned());
    contents
}

/// Ctrl+C: the row frozen with `^C` printed at the cursor, over the characters
/// ahead of it, as a shell prints.
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

/// The live prompt: the prompt's own styled runs, the line split around the
/// cursor, and the block cursor over the character it sits on — or over an
/// empty cell at the end of the line.
fn prompt_view(snapshot: &shell::PromptRow, on_activate: impl Activate) -> AnyView {
    let mut runs: Vec<AnyView> = snapshot
        .prompt
        .iter()
        .map(|span| span_view(span, on_activate))
        .collect();
    runs.extend(
        snapshot
            .before_cursor
            .iter()
            .map(|span| span_view(span, on_activate)),
    );
    // `#1c1e21` and `#f7f7f7` are [`theme::TEXT`] and [`theme::SURFACE`], the
    // two colors the terminal pins on itself.
    let cursor_character = snapshot
        .at_cursor
        .map(String::from)
        .unwrap_or_else(|| "\u{00a0}".to_string());
    runs.push(
        view! {
            <span class="animate-cursor-blink bg-[#1c1e21] text-[#f7f7f7]">{cursor_character}</span>
        }
        .into_any(),
    );
    runs.extend(
        snapshot
            .after_cursor
            .iter()
            .map(|span| span_view(span, on_activate)),
    );
    view! { <div class="min-h-row whitespace-pre-wrap break-words">{runs}</div> }.into_any()
}

// --- The session component -----------------------------------------------------------

/// The session: everything on the page, in one component.
#[component]
fn Session(path: SitePath, touch_device: bool) -> impl IntoView {
    let mode = RwSignal::new(Mode::Shell);
    let scrollback = RwSignal::new(Vec::<Line>::new());
    let prompt = RwSignal::new(Prompt {
        shell: Shell::new(),
        editor: LineEditor::new(),
    });
    let app = RwSignal::new(App::new());
    let pane = NodeRef::<html::Div>::new();
    let model = Model {
        tab: Memo::new(move |_| app.with(|app| app.tab())),
        reader: Memo::new(move |_| app.with(|app| app.reader())),
        timeline_selected: Memo::new(move |_| app.with(|app| app.selected(TIMELINE_TAB))),
        blog_selected: Memo::new(move |_| app.with(|app| app.selected(BLOG_TAB))),
        pane,
        pointer: StoredValue::new((i32::MIN, i32::MIN)),
    };
    // The view the app is on. Read for the URL bar, and to tell a view change
    // from a keystroke within one.
    let route = Memo::new(move |_| app.with(|app| app.route()));
    // Where each tab was left, so switching away and back does not lose the
    // visitor's place. The browser owns the live position; this is only what it
    // is set back to.
    let scroll_positions = StoredValue::new([0.0f64; TAB_COUNT]);
    // The view the pane was last put in place for.
    let shown = StoredValue::new(None::<SitePath>);

    // --- The URL bar ---------------------------------------------------------

    // The URL bar and the title follow the view the session is on. Entering
    // or leaving the app replaces the entry the browser is on — booting from
    // `/` leaves no shell entry behind for the back button, and quitting puts
    // `/` back over the last view — while moving between views pushes one.
    let location = Memo::new(move |_| match mode.get() {
        Mode::Shell => (SitePath::root(), SHELL_TITLE.to_string()),
        Mode::App => {
            let route = route.get();
            let title = title_for(&route);
            (route, title)
        }
    });
    Effect::new(move |previous: Option<Mode>| {
        let (path, title) = location.get();
        let now = mode.get_untracked();
        sync_url_bar(previous != Some(now), &path, &title);
        now
    });

    // --- Moving the pane -----------------------------------------------------

    // Where the tab on screen is scrolled to, noted before anything is allowed
    // to replace it. A post is not among them: it opens at its first line
    // however far down the last one was read.
    let remember_scroll = move || {
        let (tab, reading) = app.with_untracked(|app| (app.tab(), app.reader().is_some()));
        if reading {
            return;
        }
        if let Some(pane) = pane.get_untracked() {
            scroll_positions.update_value(|saved| saved[tab] = f64::from(pane.scroll_top()));
        }
    };

    // What the browser has to do once the frame showing the app's new state
    // has been drawn: put the pane where the view it now shows was left, then
    // move it as the input asked.
    let settle = move |errands: Vec<Errand>| {
        after_render(move || {
            let Some(pane) = pane.get_untracked() else {
                return;
            };
            let route = route.get_untracked();
            let (reading, tab) = app.with_untracked(|app| (app.reader().is_some(), app.tab()));
            if shown.get_value().as_ref() != Some(&route) {
                shown.set_value(Some(route));
                // A post opens at its first line; a tab comes back to where it
                // was left.
                let top = if reading {
                    0.0
                } else {
                    scroll_positions.with_value(|saved| saved[tab])
                };
                pane.set_scroll_top(top as i32);
            }
            for errand in errands {
                match errand {
                    Errand::Scroll(by) => apply_scroll(&pane, by),
                    Errand::Open(_) | Errand::Quit => {}
                }
            }
        });
    };

    // Keeps the prompt in view as the shell prints, the way a terminal does.
    let follow_prompt = move || {
        after_render(move || {
            if let Some(pane) = pane.get_untracked() {
                pane.set_scroll_top(pane.scroll_height());
            }
        });
    };

    // --- Mode transitions ----------------------------------------------------

    // Runs `dev-sam`: a fresh app, on the view `path` names if it names one.
    let launch = move |path: Option<&SitePath>| {
        let mut fresh = App::new();
        if let Some(path) = path {
            fresh.go_to(path);
        }
        app.set(fresh);
        scroll_positions.set_value([0.0; TAB_COUNT]);
        shown.set_value(None);
        mode.set(Mode::App);
    };

    // The app's own exit: back to the prompt, with a fresh line under it.
    let exit_to_shell = move || {
        mode.set(Mode::Shell);
        prompt.update(|prompt| prompt.editor = LineEditor::new());
        scrollback.update(|lines| lines.extend(LineEditor::after_dev_sam_app_exit()));
        follow_prompt();
    };

    // One input to the app: the state changes, and what the browser has to do
    // for it is done — a link opened now, while the click that asked for it is
    // still being handled and the browser will not take it for a pop-up; the
    // pane moved once the frame is drawn; the app left, if that was the ask.
    let drive = move |input: &dyn Fn(&mut App)| {
        remember_scroll();
        let errands = app
            .try_update(|app| {
                input(app);
                app.take_errands()
            })
            .unwrap_or_default();
        for errand in &errands {
            if let Errand::Open(url) = errand {
                open_url(url);
            }
        }
        if errands.contains(&Errand::Quit) {
            exit_to_shell();
            return;
        }
        settle(errands);
    };

    // --- Inputs --------------------------------------------------------------

    let handle_app_key = move |key: Key, mods: Mods| {
        drive(&|app| app.handle_key(key, mods));
    };

    // A click on something the session drew. The shell prints links too —
    // `cat contact.txt` is a page of them.
    let activate = move |target: &HitTarget| match (mode.get_untracked(), target) {
        // A pointer moving over a card reports in on every movement and asks
        // nothing of the browser: the selection moves, and the cards are
        // redrawn only when it actually did.
        (Mode::App, HitTarget::Hover(index)) => {
            app.maybe_update(|app| app.hover(*index));
        }
        (Mode::App, target) => drive(&|app| app.activate(target)),
        (Mode::Shell, HitTarget::Link(url)) => open_url(url),
        (Mode::Shell, _) => {}
    };

    // The shell's turn at a key: the line editor acts, and the outcome decides
    // what the scrollback gains. The prompt row is snapshotted first, because a
    // submit clears the editor's own copy of the line.
    let handle_shell_key = move |key: Key, mods: Mods| {
        let snapshot = prompt.with_untracked(|prompt| prompt.editor.prompt_row(&prompt.shell));
        let outcome = prompt
            .try_update(|prompt| prompt.editor.handle_key(key, mods, &mut prompt.shell))
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
                launch(None);
                return;
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
        follow_prompt();
    };

    // Back and forward move the app, rather than the document: the browser has
    // nowhere else to go, since every route is this same page. A path the app
    // has no view for means the visitor left it, which the app answers by
    // exiting to the shell.
    let navigate = move |path: &SitePath| match mode.get_untracked() {
        Mode::App if has_view(path) => drive(&|app| {
            app.go_to(path);
        }),
        Mode::App => exit_to_shell(),
        // The prompt comes back through a launch: the same way a run of the
        // app starts, so whatever view the history entry names is what opens.
        Mode::Shell => {
            if has_view(path) {
                launch(Some(path));
            }
        }
    };

    // --- Wiring --------------------------------------------------------------

    // Keyboard is captured at the window: the app answers every key it maps,
    // wherever the focus happens to be. There is nothing here that watches
    // the window's size: what fits is CSS's question, and the browser answers
    // it on every resize by itself.
    window_event_listener(leptos::ev::keydown, move |event| {
        // Except the ones the browser and the operating system own: a
        // reload, a new tab, the address bar, the find bar. A page that
        // swallowed those would be a page a visitor cannot get out of.
        if event.meta_key() {
            return;
        }
        let Some((key, mods)) = map_key(&event) else {
            return;
        };
        let mode = mode.get_untracked();
        if !claims(mode, key, mods) {
            return;
        }
        event.prevent_default();
        match mode {
            Mode::Shell => handle_shell_key(key, mods),
            Mode::App => handle_app_key(key, mods),
        }
    });
    window_event_listener(leptos::ev::popstate, move |_| navigate(&current_path()));

    // A visitor who arrived at a view asked for it by name, and gets it with
    // no banner and nothing to press; everyone else lands at the shell, with
    // `dev-sam` already typed at the prompt — and on a touch device, with no
    // keyboard to press Enter on, it runs itself.
    let opening = has_view(&path).then_some(&path);
    if touch_device || opening.is_some() {
        launch(opening);
    } else {
        let lines = prompt
            .try_update(|prompt| prompt.editor.opening_screen())
            .unwrap_or_default();
        scrollback.set(lines);
    }

    view! {
        // The terminal pins its own text color: the site body is `dark:text-gray-200`.
        <div class="terminal fixed inset-0 overflow-hidden bg-[#f7f7f7] font-terminal text-[15px] leading-[1.2] text-[#1c1e21]">
            {move || match mode.get() {
                Mode::Shell => shell_view(scrollback, prompt, pane, activate),
                Mode::App => screen::screen(model, touch_device, activate),
            }}
        </div>
    }
}

// --- The shell ------------------------------------------------------------------

/// The prompt: the scrollback, then the live prompt row. The two are read
/// apart, so a keystroke redraws the row it edits and not the lines above it.
fn shell_view(
    scrollback: RwSignal<Vec<Line>>,
    prompt: RwSignal<Prompt>,
    pane: NodeRef<html::Div>,
    on_activate: impl Activate,
) -> AnyView {
    let lines = move || {
        scrollback.with(|lines| {
            lines
                .iter()
                .map(|line| styled_line(line, true, on_activate))
                .collect::<Vec<AnyView>>()
        })
    };
    let row = move || {
        prompt.with(|prompt| prompt_view(&prompt.editor.prompt_row(&prompt.shell), on_activate))
    };
    view! {
        <div class=format!("{SCROLL} h-full w-full") node_ref=pane>
            {lines}
            {row}
        </div>
    }
    .into_any()
}
