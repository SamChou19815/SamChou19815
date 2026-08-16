//! WebAssembly C-ABI surface. Compiled only for `wasm32-unknown-unknown`;
//! there is no JS glue, so the module instantiates with zero imports.
//!
//! Protocol:
//! - `sam_resize(cols, rows)` — must be called before anything else;
//! - `sam_key(key, codepoint, mods)` / `sam_mouse(kind, button, col, row)` — input;
//! - `sam_render()` → output length; `sam_output_ptr()` → output buffer address;
//!   the buffer holds [`crate::ansi`] frames, or a bare URL for
//!   `sam_poll_action()`;
//! - `sam_should_quit()` → 1 when the app asked to quit;
//! - `sam_reset()` — drop the session so `sam_resize` starts fresh.

use crate::{ansi, Action, App, Input, Key, Mods, MouseButton, MouseEv, MouseKind};
use ratatui_core::backend::TestBackend;
use ratatui_core::layout::Rect;
use ratatui_core::terminal::Terminal;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU8, Ordering};

const KEY_UP: u32 = 0;
const KEY_DOWN: u32 = 1;
const KEY_LEFT: u32 = 2;
const KEY_RIGHT: u32 = 3;
const KEY_ENTER: u32 = 4;
const KEY_ESC: u32 = 5;
const KEY_TAB: u32 = 6;
const KEY_BACKTAB: u32 = 7;
const KEY_BACKSPACE: u32 = 8;
const KEY_PAGE_UP: u32 = 9;
const KEY_PAGE_DOWN: u32 = 10;
const KEY_HOME: u32 = 11;
const KEY_END: u32 = 12;
const KEY_DELETE: u32 = 13;
const KEY_CHAR: u32 = 14;

const MOD_CTRL: u32 = 1;
const MOD_SHIFT: u32 = 2;
const MOD_ALT: u32 = 4;

const MOUSE_PRESS: u32 = 0;
const MOUSE_RELEASE: u32 = 1;
const MOUSE_SCROLL_UP: u32 = 2;
const MOUSE_SCROLL_DOWN: u32 = 3;

struct State {
    app: App,
    terminal: Terminal<TestBackend>,
    output: Vec<u8>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = RefCell::new(None);
}

fn with_state<R>(run: impl FnOnce(&mut State) -> R) -> Option<R> {
    STATE.with(|cell| cell.borrow_mut().as_mut().map(run))
}

/// Creates or resizes the app. Must be the first call from JS.
#[unsafe(no_mangle)]
pub extern "C" fn sam_resize(cols: u32, rows: u32) {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let size = (cols.max(1) as u16, rows.max(1) as u16);
        match state.as_mut() {
            Some(state) => {
                state.app.resize(size.0, size.1);
                let _ = state.terminal.resize(Rect::new(0, 0, size.0, size.1));
            }
            None => {
                let mut app = App::new();
                app.resize(size.0, size.1);
                *state = Some(State {
                    app,
                    terminal: Terminal::new(TestBackend::new(size.0, size.1))
                        .expect("TestBackend cannot fail"),
                    output: Vec::new(),
                });
            }
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_key(key: u32, codepoint: u32, mods: u32) {
    let key = match key {
        KEY_UP => Key::Up,
        KEY_DOWN => Key::Down,
        KEY_LEFT => Key::Left,
        KEY_RIGHT => Key::Right,
        KEY_ENTER => Key::Enter,
        KEY_ESC => Key::Esc,
        KEY_TAB => Key::Tab,
        KEY_BACKTAB => Key::BackTab,
        KEY_BACKSPACE => Key::Backspace,
        KEY_PAGE_UP => Key::PageUp,
        KEY_PAGE_DOWN => Key::PageDown,
        KEY_HOME => Key::Home,
        KEY_END => Key::End,
        KEY_DELETE => Key::Delete,
        KEY_CHAR => match char::from_u32(codepoint) {
            Some(character) => Key::Char(character),
            None => return,
        },
        _ => return,
    };
    let mods = Mods {
        ctrl: mods & MOD_CTRL != 0,
        shift: mods & MOD_SHIFT != 0,
        alt: mods & MOD_ALT != 0,
    };
    with_state(|state| state.app.handle(Input::Key { key, mods }));
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_mouse(kind: u32, button: u32, col: u32, row: u32) {
    let button = match button {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        _ => MouseButton::Right,
    };
    let kind = match kind {
        MOUSE_PRESS => MouseKind::Press(button),
        MOUSE_RELEASE => MouseKind::Release(button),
        MOUSE_SCROLL_UP => MouseKind::ScrollUp,
        MOUSE_SCROLL_DOWN => MouseKind::ScrollDown,
        _ => return,
    };
    let mouse = MouseEv {
        kind,
        col: (col.max(1) - 1) as u16,
        row: (row.max(1) - 1) as u16,
    };
    with_state(|state| state.app.handle(Input::Mouse(mouse)));
}

/// Draws a frame and serializes it; returns the byte length.
#[unsafe(no_mangle)]
pub extern "C" fn sam_render() -> u32 {
    with_state(|state| {
        let State {
            app,
            terminal,
            output,
        } = state;
        let _ = terminal.draw(|frame| app.draw(frame));
        let buffer = terminal.backend().buffer().clone();
        let links = app.hit.links.clone();
        *output = ansi::serialize_frame(&buffer, &links);
        output.len() as u32
    })
    .unwrap_or(0)
}

/// Pops the next pending action URL into the output buffer; 0 when done.
#[unsafe(no_mangle)]
pub extern "C" fn sam_poll_action() -> u32 {
    with_state(|state| {
        let action = state.app.take_actions().pop();
        match action {
            Some(Action::OpenUrl(url)) => {
                state.output = url.into_bytes();
                state.output.len() as u32
            }
            None => 0,
        }
    })
    .unwrap_or(0)
}

/// Drops the app state; the next `sam_resize` starts a fresh session.
#[unsafe(no_mangle)]
pub extern "C" fn sam_reset() {
    STATE.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

// The shell owns its own byte buffers so it works before the app exists.
const SHELL_INPUT_CAPACITY: usize = 1024;
static SHELL_INPUT: [AtomicU8; SHELL_INPUT_CAPACITY] =
    [const { AtomicU8::new(0) }; SHELL_INPUT_CAPACITY];

thread_local! {
    static SHELL: RefCell<crate::shell::Shell> = RefCell::new(crate::shell::Shell::new());
    static SHELL_OUTPUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

/// Writes one byte of a shell command line into the input buffer.
#[unsafe(no_mangle)]
pub extern "C" fn sam_write_input_byte(index: u32, byte: u32) {
    if let Some(slot) = SHELL_INPUT.get(index as usize) {
        slot.store(byte as u8, Ordering::Relaxed);
    }
}

fn shell_line(length: u32) -> String {
    let bytes: Vec<u8> = SHELL_INPUT[..length.min(SHELL_INPUT_CAPACITY as u32) as usize]
        .iter()
        .map(|byte| byte.load(Ordering::Relaxed))
        .collect();
    String::from_utf8_lossy(&bytes).trim_end().to_string()
}

fn write_shell_output(output: &str) -> u32 {
    SHELL_OUTPUT.with(|cell| {
        let mut buffer = cell.borrow_mut();
        *buffer = output.as_bytes().to_vec();
        buffer.len() as u32
    })
}

/// Runs one shell command line; returns the output length.
#[unsafe(no_mangle)]
pub extern "C" fn sam_shell_execute(length: u32) -> u32 {
    let line = shell_line(length);
    SHELL.with(|shell| write_shell_output(&shell.borrow_mut().execute(&line)))
}

/// Tab-completion candidates for a shell line, one per line.
#[unsafe(no_mangle)]
pub extern "C" fn sam_shell_complete(length: u32) -> u32 {
    let line = shell_line(length);
    SHELL.with(|shell| write_shell_output(&shell.borrow().complete(&line)))
}

/// Address of the shell output buffer, valid until the next shell call.
#[unsafe(no_mangle)]
pub extern "C" fn sam_shell_output_ptr() -> u32 {
    SHELL_OUTPUT.with(|cell| cell.borrow().as_ptr() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_should_quit() -> u32 {
    with_state(|state| u32::from(state.app.quit)).unwrap_or(0)
}

/// Address of the output buffer, valid until the next `sam_*` call.
#[unsafe(no_mangle)]
pub extern "C" fn sam_output_ptr() -> u32 {
    with_state(|state| state.output.as_ptr() as u32).unwrap_or(0)
}
