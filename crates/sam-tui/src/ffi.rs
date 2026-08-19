//! WebAssembly C-ABI surface: runs the iocraft app in a fullscreen render
//! loop whose output is an in-memory ANSI byte stream. The host (xterm.js)
//! pushes raw input bytes and drains output bytes — a true terminal bridge,
//! with no frame protocol.
//!
//! - `sam_start(cols, rows)` — boot the app (idempotent);
//! - `sam_input_byte(i, b)` + `sam_input(len)` — feed input bytes;
//! - `sam_resize(cols, rows)` — report a resize;
//! - `sam_output_take()` → length; `sam_out_ptr()` → buffer address;
//! - `sam_poll_action()` → URL length (0 when none) via the same buffer;
//! - `sam_should_quit()` — 1 when the app exited.

use crate::view;
use iocraft::prelude::*;
use std::cell::RefCell;
use std::future::Future;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

struct Engine {
    task: Pin<Box<dyn Future<Output = io::Result<()>> + Send>>,
    output: Arc<Mutex<Vec<u8>>>,
    woken: Arc<AtomicBool>,
    done: bool,
}

thread_local! {
    static ENGINE: RefCell<Option<Engine>> = RefCell::new(None);
    static OUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    static INPUT_BYTES: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

fn clone_waker(data: *const ()) -> RawWaker {
    let arc = unsafe { Arc::from_raw(data as *const AtomicBool) };
    let cloned = arc.clone();
    std::mem::forget(arc);
    RawWaker::new(Arc::into_raw(cloned) as *const (), &VTABLE)
}

fn wake_waker(data: *const ()) {
    let arc = unsafe { Arc::from_raw(data as *const AtomicBool) };
    arc.store(true, Ordering::Release);
    std::mem::forget(arc);
}

fn drop_waker(data: *const ()) {
    drop(unsafe { Arc::from_raw(data as *const AtomicBool) });
}

static VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_waker, wake_waker, wake_waker, drop_waker);

struct SinkWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SinkWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn pump() {
    ENGINE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(engine) = borrow.as_mut() else {
            return;
        };
        if engine.done {
            return;
        }
        loop {
            let raw = RawWaker::new(Arc::into_raw(engine.woken.clone()) as *const (), &VTABLE);
            let waker = unsafe { Waker::from_raw(raw) };
            let mut context = Context::from_waker(&waker);
            match engine.task.as_mut().poll(&mut context) {
                Poll::Ready(_) => {
                    engine.done = true;
                    return;
                }
                Poll::Pending => {}
            }
            if !engine.woken.swap(false, Ordering::AcqRel) {
                return;
            }
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_start(cols: u16, rows: u16) {
    crossterm::set_size(cols, rows);
    crate::hit::clear();
    ENGINE.with(|cell| {
        let mut engine = cell.borrow_mut();
        // Restart after quit: drop the finished engine and leak a fresh tree.
        if engine.as_ref().is_some_and(|engine| engine.done) {
            *engine = None;
        }
        if engine.is_none() {
            let output = Arc::new(Mutex::new(Vec::new()));
            let woken = Arc::new(AtomicBool::new(true));
            let element: &'static mut _ = Box::leak(Box::new(view::root_element()));
            let future = element
                .fullscreen()
                .stdout(SinkWriter(output.clone()))
                .enable_mouse_capture();
            *engine = Some(Engine {
                task: Box::pin(future),
                output,
                woken,
                done: false,
            });
        }
    });
    pump();
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_input_byte(index: u32, byte: u32) {
    INPUT_BYTES.with(|cell| {
        let mut buffer = cell.borrow_mut();
        let index = index as usize;
        if buffer.len() <= index {
            buffer.resize(index + 1, 0);
        }
        buffer[index] = byte as u8;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_input(len: u32) {
    let bytes = INPUT_BYTES.with(|cell| {
        let mut buffer = cell.borrow_mut();
        let len = (len as usize).min(buffer.len());
        let taken = buffer[..len].to_vec();
        buffer.clear();
        taken
    });
    crossterm::push_input(&bytes);
    pump();
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_resize(cols: u16, rows: u16) {
    crossterm::set_size(cols, rows);
    crossterm::push_event(crossterm::event::Event::Resize(cols, rows));
    pump();
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_output_take() -> u32 {
    let taken = ENGINE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        borrow
            .as_mut()
            .map(|engine| std::mem::take(&mut *engine.output.lock().unwrap()))
            .unwrap_or_default()
    });
    let len = taken.len() as u32;
    if len > 0 {
        OUT.with(|cell| *cell.borrow_mut() = taken);
    }
    len
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_out_ptr() -> u32 {
    OUT.with(|cell| cell.borrow().as_ptr() as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_should_quit() -> u32 {
    ENGINE.with(|cell| {
        let borrow = cell.borrow();
        u32::from(borrow.as_ref().is_some_and(|engine| engine.done))
    })
}

// --- The dev-sam shell: same module, independent state -------------------------

thread_local! {
    static SHELL: RefCell<crate::shell::Shell> = RefCell::new(crate::shell::Shell::new());
    static SHELL_OUTPUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    static SHELL_INPUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

fn shell_line(len: u32) -> String {
    SHELL_INPUT.with(|cell| {
        let mut buffer = cell.borrow_mut();
        let len = (len as usize).min(buffer.len());
        let taken = buffer[..len].to_vec();
        buffer.clear();
        String::from_utf8_lossy(&taken).trim_end().to_string()
    })
}

fn write_shell_output(output: &str) -> u32 {
    SHELL_OUTPUT.with(|cell| {
        *cell.borrow_mut() = output.as_bytes().to_vec();
        output.len() as u32
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_write_input_byte(index: u32, byte: u32) {
    SHELL_INPUT.with(|cell| {
        let mut buffer = cell.borrow_mut();
        let index = index as usize;
        if buffer.len() <= index {
            buffer.resize(index + 1, 0);
        }
        buffer[index] = byte as u8;
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_shell_execute(length: u32) -> u32 {
    let line = shell_line(length);
    SHELL.with(|shell| write_shell_output(&shell.borrow_mut().execute(&line)))
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_shell_complete(length: u32) -> u32 {
    let line = shell_line(length);
    SHELL.with(|shell| write_shell_output(&shell.borrow().complete(&line)))
}

#[unsafe(no_mangle)]
pub extern "C" fn sam_shell_output_ptr() -> u32 {
    SHELL_OUTPUT.with(|cell| cell.borrow().as_ptr() as u32)
}

/// Drains pending URL actions into the shared output buffer.
#[unsafe(no_mangle)]
pub extern "C" fn sam_poll_action() -> u32 {
    let action = crate::PENDING_ACTIONS.with(|pending| pending.borrow_mut().pop());
    match action {
        Some(crate::Action::OpenUrl(url)) => {
            OUT.with(|out| *out.borrow_mut() = url.into_bytes());
            OUT.with(|out| out.borrow().len() as u32)
        }
        None => 0,
    }
}
