//! WebAssembly surface: runs the iocraft app in a fullscreen render loop
//! whose output is an in-memory ANSI byte stream. The host (xterm.js) pushes
//! raw input bytes and drains output bytes — a true terminal bridge, with no
//! frame protocol.
//!
//! wasm-bindgen generates the JS glue and the `.d.ts` for these; the names
//! below are what the host sees:
//!
//! - `start(cols, rows)` — boot the app (idempotent);
//! - `input(bytes)` — feed input bytes;
//! - `resize(cols, rows)` — report a resize;
//! - `drain()` — take the pending ANSI output;
//! - `pollAction()` — the next URL to open, if any;
//! - `hasView(path)` — whether a site path names a view of this app;
//! - `navigate(path)` — show the view a site path names;
//! - `route()` — the view the last frame drew, as a site path;
//! - `imageRegions()` — where this frame drew its artwork;
//! - `shouldQuit()` — true when the app exited.

use crate::view;
use iocraft::prelude::*;
use std::cell::RefCell;
use std::future::Future;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use wasm_bindgen::prelude::*;

struct Engine {
    task: Pin<Box<dyn Future<Output = io::Result<()>> + Send>>,
    output: Arc<Mutex<Vec<u8>>>,
    woken: Arc<Woken>,
    done: bool,
}

thread_local! {
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
    /// The size the app was last told about, so [`sam_navigate`] can wake the
    /// render loop without resizing anything.
    static SIZE: RefCell<(u16, u16)> = const { RefCell::new((0, 0)) };
}

/// Wake flag for the render future. The host drives polling synchronously, so
/// waking just records "poll once more" for the loop in [`pump`].
struct Woken(AtomicBool);

impl Wake for Woken {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.store(true, Ordering::Release);
    }
}

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
        let waker = Waker::from(engine.woken.clone());
        let mut context = Context::from_waker(&waker);
        loop {
            match engine.task.as_mut().poll(&mut context) {
                Poll::Ready(_) => {
                    engine.done = true;
                    return;
                }
                Poll::Pending => {}
            }
            if !engine.woken.0.swap(false, Ordering::AcqRel) {
                return;
            }
        }
    });
}

#[wasm_bindgen(js_name = start)]
pub fn sam_start(cols: u16, rows: u16) {
    SIZE.with(|size| *size.borrow_mut() = (cols, rows));
    crossterm::set_size(cols, rows);
    // Seed the app with the real size before the first frame, so layout and
    // scroll math never run against the placeholder width.
    crossterm::push_event(crossterm::event::Event::Resize(cols, rows));
    crate::hit::begin_frame(false);
    ENGINE.with(|cell| {
        let mut engine = cell.borrow_mut();
        // Restart after quit: drop the finished engine and leak a fresh tree.
        if engine.as_ref().is_some_and(|engine| engine.done) {
            *engine = None;
        }
        if engine.is_none() {
            let output = Arc::new(Mutex::new(Vec::new()));
            let woken = Arc::new(Woken(AtomicBool::new(true)));
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

#[wasm_bindgen(js_name = input)]
pub fn sam_input(bytes: &[u8]) {
    crossterm::push_input(bytes);
    pump();
}

#[wasm_bindgen(js_name = resize)]
pub fn sam_resize(cols: u16, rows: u16) {
    SIZE.with(|size| *size.borrow_mut() = (cols, rows));
    crossterm::set_size(cols, rows);
    crossterm::push_event(crossterm::event::Event::Resize(cols, rows));
    pump();
}

/// Takes the ANSI bytes written since the last drain.
#[wasm_bindgen(js_name = drain)]
pub fn sam_drain() -> String {
    ENGINE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let taken = borrow
            .as_mut()
            .map(|engine| std::mem::take(&mut *engine.output.lock().unwrap()))
            .unwrap_or_default();
        String::from_utf8_lossy(&taken).into_owned()
    })
}

#[wasm_bindgen(js_name = shouldQuit)]
pub fn sam_should_quit() -> bool {
    ENGINE.with(|cell| cell.borrow().as_ref().is_some_and(|engine| engine.done))
}

/// Whether this app has a view at `path`. The host asks before following a
/// link or a back button itself rather than leaving it to the browser.
#[wasm_bindgen(js_name = hasView)]
pub fn sam_has_view(path: &str) -> bool {
    crate::has_view(path)
}

/// Shows the view `path` names: the URL the page was entered at, a link
/// followed inside a post, or wherever the back button just went.
///
/// Before the app boots this only records the view, which `start` then opens
/// on. Afterwards the running render loop has to be woken to pick it up, and
/// iocraft forwards only key, mouse and resize events — so the wake is a
/// resize to the size the app already has, the same no-op `start` pushes.
#[wasm_bindgen(js_name = navigate)]
pub fn sam_navigate(path: &str) {
    crate::request_route(path);
    let running = ENGINE.with(|cell| cell.borrow().is_some());
    if running {
        let (cols, rows) = SIZE.with(|size| *size.borrow());
        crossterm::push_event(crossterm::event::Event::Resize(cols, rows));
        pump();
    }
}

/// The view the last frame drew, as a site path, for the host to put in the
/// URL bar.
#[wasm_bindgen(js_name = route)]
pub fn sam_route() -> String {
    crate::current_route()
}

/// Drains the next pending URL action, if any.
#[wasm_bindgen(js_name = pollAction)]
pub fn sam_poll_action() -> Option<String> {
    crate::PENDING_ACTIONS
        .with(|pending| pending.borrow_mut().pop())
        .map(|crate::Action::OpenUrl(url)| url)
}

/// Where the current frame drew its artwork, one row per image, as
/// `"x y cols rows visibleX visibleY visibleCols visibleRows url"` in canvas
/// cells. The app owns the alternate screen, so cell (0, 0) is the top left of
/// the viewport and the host can place an `<img>` straight onto it.
///
/// The second rectangle is the part that survived the pane's clipping: a card
/// scrolled half off the bottom paints only some of its artwork, and the
/// overlay has to crop to match. Images with nothing on screen are omitted.
/// The first rectangle is the whole picture, so its origin goes negative for
/// one scrolled part-way off the top of a pane.
///
/// Space-separated rather than JSON: no asset path contains a space, and a
/// serializer would cost the wasm binary more than the artwork itself does.
#[wasm_bindgen(js_name = imageRegions)]
pub fn sam_image_regions() -> Vec<String> {
    crate::image::regions()
        .into_iter()
        .map(|region| {
            format!(
                "{} {} {} {} {} {} {} {} {}",
                region.x,
                region.y,
                region.cols,
                region.rows,
                region.visible_x,
                region.visible_y,
                region.visible_cols,
                region.visible_rows,
                region.url,
            )
        })
        .collect()
}

// --- The dev-sam shell: same module, independent state -------------------------

thread_local! {
    static SHELL: RefCell<crate::shell::Shell> = RefCell::new(crate::shell::Shell::new());
}

#[wasm_bindgen(js_name = shellExecute)]
pub fn sam_shell_execute(line: &str) -> String {
    SHELL.with(|shell| shell.borrow_mut().execute(line.trim_end()))
}

#[wasm_bindgen(js_name = shellComplete)]
pub fn sam_shell_complete(line: &str) -> Vec<String> {
    let candidates = SHELL.with(|shell| shell.borrow().complete(line.trim_end()));
    if candidates.is_empty() {
        Vec::new()
    } else {
        candidates.lines().map(str::to_string).collect()
    }
}
