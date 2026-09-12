//! The Leptos front-end: one session that is either the `dev-sam-sh` prompt,
//! the full-screen app, or — on a touch device — a whole view drawn as a page
//! the browser scrolls.
//!
//! There is nothing to measure here. The session is one `.terminal` box that
//! pins the type and the two colors the site is drawn in, and every view is
//! ordinary HTML laid out by Tailwind classes ([`screen`]); the lengths are
//! `ch` and `lh`, which the browser resolves against the real font and the
//! real viewport, so the browser does the fitting, the wrapping, the
//! truncating, the scrolling and the hit-testing.

pub mod screen;

use crate::hit::HitTarget;
use crate::shell::{self, EditOutcome, Launch, LineEditor, Shell};
use crate::site_path::SitePath;
use crate::style::{Line, Span, TextStyle};
use crate::{
    has_view, request_quit, request_route, reset_route_sync, App, HostEvent, Key, Mods, Scroll,
    SHELL_TITLE, TAB_COUNT, TAB_ROUTES,
};
use leptos::prelude::*;
use std::fmt::Write as _;
use wasm_bindgen::JsCast;

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

/// What mode the session is in: the prompt, the full-screen app, or the page
/// the touch build drew.
#[derive(Clone, PartialEq)]
enum Mode {
    Shell,
    App,
    Page(SitePath),
}

/// The scrollback and the live prompt of the shell.
#[derive(Clone)]
struct ShellScreen {
    shell: Shell,
    editor: LineEditor,
    /// Printed lines, in order. The prompt row is not among them: it is
    /// rendered live, and frozen into this list when the visitor submits.
    lines: Vec<Line>,
}

/// Mounts the session into `parent`. `touch_device` is the host's answer for
/// whether this is a phone or tablet — the build that draws a whole view as a
/// page instead of taking the screen over.
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

/// The session's own box: the screen it takes over, the type it is set at, and
/// the two colors it pins. `#f7f7f7` and `#1c1e21` are [`theme::SURFACE`] and
/// [`theme::TEXT`] — the terminal owns both ends of its own contrast, because a
/// run with no color of its own must be drawn in this one rather than in
/// whatever the page inherits. The site's body is `dark:text-gray-200`, so
/// inheriting would paint every uncolored run — the whole line typed at the
/// prompt — near-white on a surface that stays light.
const TERM: &str = "terminal fixed inset-0 overflow-hidden bg-[#f7f7f7] font-terminal text-[15px] leading-[1.2] text-[#1c1e21]";

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

/// The touch build's way between its own views: it draws one of them, whole, so
/// reaching another means loading it, in this same tab.
fn load_page(path: &SitePath) {
    if path.as_str() != current_path().as_str() {
        let window = web_sys::window().expect("window");
        let _ = window.location().assign(path.as_str());
    }
}

/// Puts the URL bar and the document title on a view. The first view of a
/// session replaces, so booting from `/` leaves no shell entry behind for the
/// back button; every later one pushes.
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

/// The box the app scrolls: the pane under the header, which is the one box in
/// the tree that says it scrolls. What the front-end has to find its way back
/// to is marked with a data attribute rather than a class, so the classes stay
/// what they say they are — the box's styling, and nothing else.
fn pane_element() -> Option<web_sys::Element> {
    web_sys::window()?
        .document()?
        .query_selector("[data-pane]")
        .ok()
        .flatten()
}

/// One row of the type, as the pane is actually set at — read off it, since
/// `line-height` is what a row is.
fn pane_line_height() -> f64 {
    let Some(pane) = pane_element() else {
        return 18.0;
    };
    web_sys::window()
        .and_then(|window| window.get_computed_style(&pane).ok().flatten())
        .and_then(|style| style.get_property_value("line-height").ok())
        .and_then(|value| value.trim().trim_end_matches("px").parse::<f64>().ok())
        .filter(|height| *height > 0.0)
        .unwrap_or(18.0)
}

/// Moves the pane, in the units the keystroke asked for.
fn apply_scroll(by: Scroll) {
    let Some(pane) = pane_element() else {
        return;
    };
    let viewport = f64::from(pane.client_height());
    match by {
        Scroll::Rows(rows) => {
            pane.scroll_by_with_x_and_y(0.0, f64::from(rows) * pane_line_height());
        }
        // A screenful less two rows, so the lines the eye was on are still
        // there after the jump.
        Scroll::Pages(pages) => {
            let row = pane_line_height();
            let page = (viewport - 2.0 * row).max(row);
            pane.scroll_by_with_x_and_y(0.0, f64::from(pages) * page);
        }
        Scroll::Top => pane.set_scroll_top(0),
        Scroll::Bottom => pane.set_scroll_top(pane.scroll_height()),
    }
}

/// Brings the selected card into view, and no further: `nearest` moves the pane
/// the least it can, so arrowing down a list slides it by a card rather than
/// snapping the selection to an edge.
fn reveal_selection() {
    let Some(pane) = pane_element() else {
        return;
    };
    let Some(selected) = pane.query_selector("[data-selected]").ok().flatten() else {
        return;
    };
    let options = web_sys::ScrollIntoViewOptions::new();
    options.set_block(web_sys::ScrollLogicalPosition::Nearest);
    selected.scroll_into_view_with_scroll_into_view_options(&options);
}

/// The cards the pane is showing, as the first and last index of the ones any
/// part of which is on screen.
fn cards_in_view() -> Option<(usize, usize)> {
    let pane = pane_element()?;
    let view = pane.get_bounding_client_rect();
    let cards = pane.query_selector_all("[data-card]").ok()?;
    let mut range: Option<(usize, usize)> = None;
    for index in 0..cards.length() {
        let Some(card) = cards
            .item(index)
            .and_then(|node| node.dyn_into::<web_sys::Element>().ok())
        else {
            continue;
        };
        let box_ = card.get_bounding_client_rect();
        if box_.bottom() <= view.top() || box_.top() >= view.bottom() {
            continue;
        }
        let Some(number) = card
            .get_attribute("data-card")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            continue;
        };
        range = Some(match range {
            None => (number, number),
            Some((first, last)) => (first.min(number), last.max(number)),
        });
    }
    range
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
fn claims(mode: &Mode, key: Key, mods: Mods) -> bool {
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
        Mode::Page(_) => false,
    }
}

// --- Styled runs ---------------------------------------------------------------------

/// One styled run as a `span`, with the click target it carries, if any. A
/// link is clicked on the words it is written on, and nowhere else.
pub(crate) fn span_view(span: &Span, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
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
/// everything; the listings that must not wrap ([`screen::code_listing`], code
/// blocks) ask for `wraps: false` and scroll instead.
pub(crate) fn styled_line(
    line: &Line,
    wraps: bool,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> AnyView {
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

// --- Shell prompt rendering ----------------------------------------------------------

/// The prompt row as it was at the moment it froze: the prompt itself, then the
/// edited line with the character under the cursor put back in.
fn frozen_prompt_line(snapshot: &shell::PromptRow) -> Line {
    let mut contents = shell::prompt_spans();
    contents.extend(snapshot.before_cursor.iter().cloned());
    if let Some(character) = snapshot.at_cursor {
        contents.push(Span::styled(character.to_string(), TextStyle::default()));
    }
    contents.extend(snapshot.after_cursor.iter().cloned());
    contents
}

/// Freezes the typed prompt row into the scrollback exactly as it was, the way
/// a shell leaves it behind when a command runs.
fn freeze_prompt(screen: &mut ShellScreen, snapshot: &shell::PromptRow) {
    let line = frozen_prompt_line(snapshot);
    screen.lines.push(line);
}

/// Ctrl+C: the row freezes with `^C` printed at the cursor, over the characters
/// ahead of it, as a shell prints.
fn freeze_interrupted_prompt(screen: &mut ShellScreen, snapshot: &shell::PromptRow) {
    let mut contents = shell::prompt_spans();
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
    screen.lines.push(contents);
}

/// The live prompt: the prompt's own styled runs, the line split around the
/// cursor, and the block cursor over the character it sits on — or over an
/// empty cell at the end of the line.
fn prompt_view(
    snapshot: &shell::PromptRow,
    on_activate: impl Fn(&HitTarget) + Copy + 'static,
) -> AnyView {
    let mut runs: Vec<AnyView> = shell::prompt_spans()
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
    let mode = RwSignal::new(if touch_device {
        Mode::Page(SitePath::root())
    } else {
        Mode::Shell
    });
    let shell_screen = RwSignal::new(ShellScreen {
        shell: Shell::new(),
        editor: LineEditor::new(),
        lines: Vec::new(),
    });
    let app = RwSignal::new(App::new());
    // Bumped whenever something the DOM has to be current for is pending: the
    // pane has to have been drawn before it can be scrolled.
    let painted = RwSignal::new(0u32);
    let pending = StoredValue::new(Vec::<HostEvent>::new());
    // Where each tab was left, so switching away and back does not lose the
    // visitor's place. The browser owns the live position; this is only what it
    // is set back to.
    let scroll_positions = StoredValue::new([0.0f64; TAB_COUNT]);
    let shown = StoredValue::new(None::<SitePath>);
    let booted = StoredValue::new(false);

    // --- The browser errands -------------------------------------------------

    // Opening a link has to happen while the click that asked for it is still
    // being handled, or the browser takes it for a pop-up; moving the pane has
    // to wait until the pane has been drawn. So the first kind is applied here
    // and the second is held for the effect that runs after the frame.
    let apply_host_events = move || {
        while let Some(event) = crate::poll_host_event() {
            match event {
                HostEvent::Open(url) => open_url(&url),
                HostEvent::Navigate(path) => load_page(&path),
                HostEvent::Route {
                    replace,
                    path,
                    title,
                } => sync_url_bar(replace, &path, &title),
                scrolling => pending.update_value(|queue| queue.push(scrolling)),
            }
        }
    };
    // The same, and then a frame: whatever the app has just done, the view it
    // leaves behind is drawn and the effect below picks up where it left off.
    let sync_host_events = move || {
        apply_host_events();
        painted.update(|count| *count = count.wrapping_add(1));
    };

    // Where the tab on screen is scrolled to, noted before anything is allowed
    // to replace it. A post is not among them: it opens at its first line
    // however far down the last one was read.
    let remember_scroll = move || {
        let tab = app.read_untracked().tab();
        if app.read_untracked().reader().is_some() {
            return;
        }
        if let Some(pane) = pane_element() {
            scroll_positions.update_value(|saved| saved[tab] = f64::from(pane.scroll_top()));
        }
    };

    // --- Mode transitions ----------------------------------------------------

    // Runs `dev-sam`: the touch build opens the page at whatever view the host
    // asked for (or the app's own first tab), the full-screen app takes the
    // screen over.
    let launch = move |launch: Launch| {
        if launch.touch {
            let path = crate::take_pending_route()
                .unwrap_or_else(|| SitePath::new(TAB_ROUTES[crate::ABOUT_TAB]));
            reset_route_sync();
            mode.set(Mode::Page(path));
        } else {
            app.update(|app| {
                *app = App::new();
            });
            mode.set(Mode::App);
        }
        sync_host_events();
    };

    // The app's own exit: back to the prompt, with the visitor put back on `/`
    // so the back button never has to walk through the exit.
    let exit_to_shell = move || {
        reset_route_sync();
        crate::push_host_event(HostEvent::Route {
            replace: true,
            path: SitePath::root(),
            title: SHELL_TITLE.to_string(),
        });
        let _ = crate::take_pending_route();
        mode.set(Mode::Shell);
        shell_screen.update(|screen| {
            for line in screen.editor.after_dev_sam_app_exit() {
                screen.lines.push(line);
            }
            screen.editor = LineEditor::new();
        });
        sync_host_events();
    };

    let quit_if_requested = move || {
        if app.get_untracked().quit {
            exit_to_shell();
        }
    };

    // --- Inputs --------------------------------------------------------------

    let handle_app_key = move |key: Key, mods: Mods| {
        remember_scroll();
        app.update(|app| {
            app.handle_key(key, mods);
            app.take_actions();
        });
        sync_host_events();
        quit_if_requested();
    };

    // A click on something the app drew: the app acts on it in place; on the
    // touch page it becomes a browser errand, because the page holds one view
    // and reaching another means loading it.
    let activate = move |target: &HitTarget| {
        match mode.get_untracked() {
            Mode::App => {
                remember_scroll();
                app.update(|app| {
                    app.activate(target);
                    app.take_actions();
                });
                sync_host_events();
                quit_if_requested();
            }
            Mode::Page(path) => {
                let page = App::page(&path);
                if let Some(event) = page.tap(target) {
                    match event {
                        HostEvent::Open(url) => open_url(&url),
                        HostEvent::Navigate(path) => load_page(&path),
                        _ => {}
                    }
                }
            }
            // The shell prints links too — `cat contact.txt` is a page of them.
            Mode::Shell => {
                if let HitTarget::Link(url) = target {
                    open_url(url);
                }
            }
        }
        sync_host_events();
    };

    // The shell's turn at a key: the line editor acts, and the outcome decides
    // what the scrollback gains. The prompt row is snapshotted first, because a
    // submit clears the editor's own copy of the line.
    let handle_shell_key = move |key: Key, mods: Mods| {
        let snapshot = shell_screen.get_untracked().editor.prompt_row();
        let mut requested = None;
        shell_screen.update(|screen| {
            match screen.editor.handle_key(key, mods, &mut screen.shell) {
                EditOutcome::None | EditOutcome::Redraw => {}
                EditOutcome::Output(lines) => {
                    freeze_prompt(screen, &snapshot);
                    if lines.is_empty() {
                        screen.lines.push(Vec::new());
                    }
                    screen.lines.extend(lines);
                }
                EditOutcome::ClearScreen => screen.lines.clear(),
                EditOutcome::Launch(launch) => requested = Some(launch),
                EditOutcome::Completion(candidates) => {
                    freeze_prompt(screen, &snapshot);
                    screen.lines.push(vec![Span::new(candidates.join("   "))]);
                }
                EditOutcome::Interrupt => freeze_interrupted_prompt(screen, &snapshot),
            }
        });
        if let Some(requested) = requested {
            shell_screen.update(|screen| freeze_prompt(screen, &snapshot));
            launch(requested);
        }
        sync_host_events();
    };

    // Back and forward move the app, rather than the document: the browser has
    // nowhere else to go, since every route is this same page. A path the app
    // has no view for means the visitor left it, which the app answers by
    // exiting to the shell.
    let navigate = move |path: &SitePath| {
        if has_view(path) {
            request_route(path);
            match mode.get_untracked() {
                Mode::App => {
                    remember_scroll();
                    app.update(|app| {
                        if let Some(route) = crate::take_pending_route() {
                            app.go_to(&route);
                        }
                    });
                    sync_host_events();
                    quit_if_requested();
                }
                // The prompt and the page both come back through a launch: the
                // same way a run of the app starts, so whatever view the history
                // entry names is what opens.
                Mode::Shell | Mode::Page(_) => launch(Launch {
                    touch: touch_device,
                }),
            }
        } else if mode.get_untracked() == Mode::App {
            request_quit();
            app.update(|app| {
                let _ = crate::take_pending_route();
                if crate::take_pending_quit() {
                    app.quit = true;
                }
            });
            exit_to_shell();
        }
    };

    // --- After the frame is drawn --------------------------------------------

    // The pane is a scrolling box, so everything about where it sits has to
    // wait until it has been drawn: putting a tab back where it was left,
    // answering the keystrokes that move it, and keeping the selected card in
    // view.
    Effect::new(move |_| {
        painted.track();
        mode.track();
        app.track();
        // The frame that has just been drawn published the view it is on, so
        // the URL bar and the title follow it here.
        apply_host_events();
        let (route, reader, tab) = {
            let app = app.read_untracked();
            (app.route(), app.reader().is_some(), app.tab())
        };
        if shown.get_value().as_ref() != Some(&route) {
            shown.set_value(Some(route));
            if let Some(pane) = pane_element() {
                // A post opens at its first line; a tab comes back to where it
                // was left.
                let top = if reader {
                    0.0
                } else {
                    scroll_positions.with_value(|saved| saved[tab])
                };
                pane.set_scroll_top(top as i32);
            }
        }
        let queued = pending.try_update_value(std::mem::take).unwrap_or_default();
        for event in queued {
            match event {
                HostEvent::Scroll(by) => apply_scroll(by),
                HostEvent::Reveal => reveal_selection(),
                _ => {}
            }
        }
    });

    // --- Wiring --------------------------------------------------------------

    // Wire the keyboard and the history once, and — once — boot. There is
    // nothing here that watches the window's size: what fits is CSS's question,
    // and the browser answers it on every resize by itself.
    Effect::new(move |_| {
        // Keyboard is captured at the window: the app answers every key it maps,
        // wherever the focus happens to be.
        let on_keydown = move |event: web_sys::KeyboardEvent| {
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
            if !claims(&mode, key, mods) {
                return;
            }
            event.prevent_default();
            match mode {
                Mode::Shell => handle_shell_key(key, mods),
                Mode::App => handle_app_key(key, mods),
                Mode::Page(_) => {}
            }
        };
        let keydown_closure =
            wasm_bindgen::closure::Closure::wrap(Box::new(on_keydown) as Box<dyn FnMut(_)>);
        web_sys::window()
            .expect("window")
            .add_event_listener_with_callback("keydown", keydown_closure.as_ref().unchecked_ref())
            .expect("adding the keydown listener");
        // Forgotten rather than dropped: the listener outlives the session.
        std::mem::forget(keydown_closure);

        // The selection follows the pane: whatever the visitor scrolls to is
        // what the next arrow press moves from, rather than snapping back to a
        // card that has long since gone off screen. A scroll event does not
        // bubble, so this listens on the way down instead.
        let on_scroll = move |_: web_sys::Event| {
            if mode.get_untracked() != Mode::App {
                return;
            }
            let Some((first, last)) = cards_in_view() else {
                return;
            };
            app.update(|app| app.selection_in_view(first, last));
        };
        let scroll_closure = wasm_bindgen::closure::Closure::wrap(
            Box::new(on_scroll) as Box<dyn FnMut(web_sys::Event)>
        );
        web_sys::window()
            .expect("window")
            .add_event_listener_with_callback_and_bool(
                "scroll",
                scroll_closure.as_ref().unchecked_ref(),
                true,
            )
            .expect("adding the scroll listener");
        std::mem::forget(scroll_closure);

        let on_popstate = move |_: web_sys::Event| navigate(&current_path());
        let popstate_closure = wasm_bindgen::closure::Closure::wrap(
            Box::new(on_popstate) as Box<dyn FnMut(web_sys::Event)>
        );
        web_sys::window()
            .expect("window")
            .add_event_listener_with_callback("popstate", popstate_closure.as_ref().unchecked_ref())
            .expect("adding the popstate listener");
        std::mem::forget(popstate_closure);

        // A visitor who arrived at a view asked for it by name, and gets it with
        // no banner and nothing to press; everyone else lands at the shell, with
        // `dev-sam` already typed at the prompt — and on a touch device it runs
        // itself, as it always has.
        if !booted.get_value() {
            booted.set_value(true);
            if touch_device {
                if has_view(&path) {
                    request_route(&path);
                }
                launch(Launch { touch: true });
            } else if has_view(&path) {
                request_route(&path);
                launch(Launch { touch: false });
            } else {
                shell_screen.update(|screen| {
                    let (lines, _) = screen.editor.opening_screen(false);
                    screen.lines.extend(lines);
                });
            }
        }
    });

    view! {
        <div class=TERM>
            {move || {
                match mode.get() {
                    Mode::Shell => shell_view(shell_screen.get(), activate).into_any(),
                    Mode::App => {
                        let current = app.get();
                        crate::publish_route(current.route());
                        screen::screen(&current, false, activate).into_any()
                    }
                    Mode::Page(path) => screen::page(&path, activate).into_any(),
                }
            }}
        </div>
    }
}

// --- The shell ------------------------------------------------------------------

fn shell_view(screen: ShellScreen, on_activate: impl Fn(&HitTarget) + Copy + 'static) -> AnyView {
    let mut rows: Vec<AnyView> = screen
        .lines
        .iter()
        .map(|line| styled_line(line, true, on_activate))
        .collect();
    rows.push(prompt_view(&screen.editor.prompt_row(), on_activate));
    view! {
        <div class=format!("{SCROLL} h-full w-full") data-pane="">
            {rows}
        </div>
    }
    .into_any()
}
