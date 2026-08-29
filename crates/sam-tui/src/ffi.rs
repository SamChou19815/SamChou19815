//! WebAssembly surface: one terminal session, driven entirely by bytes. The
//! host (xterm.js) pushes raw input and drains raw ANSI output — a true
//! terminal bridge, with no frame protocol and no key mapping.
//!
//! A session is in one of two modes. It opens at the `dev-sam-sh` prompt, where
//! [`LineEditor`] edits the line and [`Shell`] runs the commands; `dev-sam`
//! hands the same byte stream to the full-screen iocraft app, and quitting it
//! hands it back. Both write into the same buffer, so [`sam_drain`] never has
//! to know which is up.
//!
//! wasm-bindgen generates the JS glue and the `.d.ts` for these; the names
//! below are what the host sees:
//!
//! - `start(cols, rows, path, touch)` — open a session, at `path`'s view if it
//!   is one and at the shell otherwise;
//! - `input(bytes)` — feed input bytes;
//! - `resize(cols, rows)` — report a resize;
//! - `drain()` — take the pending ANSI output;
//! - `navigate(path)` — show the view a site path names, for the back button;
//! - `openLink(url)` — activate a link the terminal printed;
//! - `pollEvent()` — the next thing only the browser can do;
//! - `imageRegions()` — where this frame drew its artwork.

use crate::shell::{LineEditor, Shell};
use crate::view;
use crossterm::event::Event;
use iocraft::prelude::*;
use std::cell::RefCell;
use std::future::Future;
use std::io::{self, Write};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Wake, Waker};
use std::time::Duration;
use wasm_bindgen::prelude::*;

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// Runs `body` against the live session, if there is one.
fn with_session<T>(body: impl FnOnce(&mut Session) -> T) -> Option<T> {
    SESSION.with(|cell| cell.borrow_mut().as_mut().map(body))
}

/// Wake flag for the render future. The host drives polling synchronously, so
/// waking just records "poll once more" for the loop in [`Engine::pump`].
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

/// The running full-screen app: an iocraft render future the host polls itself.
struct Engine {
    task: Pin<Box<dyn Future<Output = io::Result<()>> + Send>>,
    woken: Arc<Woken>,
    done: bool,
}

impl Engine {
    fn pump(&mut self) {
        if self.done {
            return;
        }
        let waker = Waker::from(self.woken.clone());
        let mut context = Context::from_waker(&waker);
        loop {
            if self.task.as_mut().poll(&mut context).is_ready() {
                self.done = true;
                return;
            }
            if !self.woken.0.swap(false, Ordering::AcqRel) {
                return;
            }
        }
    }
}

enum Mode {
    Shell,
    App(Engine),
}

struct Session {
    mode: Mode,
    shell: Shell,
    editor: LineEditor,
    /// Where both modes write. Taken whole by [`sam_drain`].
    output: Arc<Mutex<Vec<u8>>>,
    cols: u16,
    rows: u16,
}

impl Session {
    fn new(cols: u16, rows: u16) -> Self {
        Session {
            mode: Mode::Shell,
            shell: Shell::new(),
            editor: LineEditor::new(),
            output: Arc::new(Mutex::new(Vec::new())),
            cols,
            rows,
        }
    }

    fn write(&mut self, text: &str) {
        self.output
            .lock()
            .unwrap()
            .extend_from_slice(text.as_bytes());
    }

    /// Boots the full-screen app, which takes the byte stream over from the
    /// shell. Restarting after a quit leaks a fresh tree rather than reviving
    /// the finished one.
    fn launch(&mut self) {
        crate::hit::begin_frame(false);
        let element: &'static mut _ = Box::leak(Box::new(view::root_element()));
        let future = element
            .fullscreen()
            .stdout(SinkWriter(self.output.clone()))
            .enable_mouse_capture();
        let mut engine = Engine {
            task: Box::pin(future),
            woken: Arc::new(Woken(AtomicBool::new(true))),
            done: false,
        };
        // Seed the app with the real size before the first frame, so layout and
        // scroll math never run against the placeholder width.
        crossterm::set_size(self.cols, self.rows);
        crossterm::push_event(Event::Resize(self.cols, self.rows));
        engine.pump();
        self.mode = Mode::App(engine);
        self.settle();
    }

    /// Hands the byte stream back to the shell once the app has exited. The app
    /// leaves the alternate screen on its own way out, so the prompt lands back
    /// on the scrollback it was typed into.
    fn settle(&mut self) {
        if !matches!(&self.mode, Mode::App(engine) if engine.done) {
            return;
        }
        self.mode = Mode::Shell;
        // No frame will be drawn again, so the last one's artwork has to be
        // retracted by hand — the host would otherwise leave those `<img>`
        // elements floating over the shell forever.
        crate::image::begin_frame(crate::image::LAYER_PANE);
        // `/` is the shell, so leaving the app puts the visitor back at it
        // without adding a step for the back button to walk through — and the
        // next run's first view replaces that entry rather than pushing past it.
        crate::reset_route_sync();
        crate::push_host_event(crate::HostEvent::Route {
            replace: true,
            path: "/".to_string(),
            title: crate::SHELL_TITLE.to_string(),
        });
        let banner = self.editor.resume();
        self.write(&banner);
    }

    /// Consumes whatever input the host has just pushed.
    fn drive(&mut self) {
        match &mut self.mode {
            Mode::App(engine) => {
                engine.pump();
                self.settle();
            }
            Mode::Shell => self.edit(),
        }
    }

    /// The shell's turn: crossterm parses the pushed bytes into events, and the
    /// line editor acts on the keys among them.
    fn edit(&mut self) {
        while matches!(self.mode, Mode::Shell) {
            // Nothing buffered, or the stream is broken; either way there is
            // nothing more to read this turn.
            if !matches!(crossterm::event::poll(Duration::ZERO), Ok(true)) {
                return;
            }
            let Ok(Event::Key(key)) = crossterm::event::read() else {
                continue;
            };
            let (ansi, launch) = self.editor.key(key, &mut self.shell);
            self.write(&ansi);
            if launch {
                // Whatever is still queued belongs to the app, not the shell.
                self.launch();
            }
        }
    }

    /// Shows the view a site path names: the URL the page was entered at, a
    /// link followed inside a post, or wherever the back button just went. A
    /// path that is no view of this app means the visitor has left it.
    fn go_to(&mut self, path: &str) {
        if !crate::has_view(path) {
            if matches!(self.mode, Mode::App(_)) {
                crate::request_quit();
                self.wake();
            }
            return;
        }
        crate::request_route(path);
        match self.mode {
            Mode::App(_) => self.wake(),
            Mode::Shell => self.launch(),
        }
    }

    /// Wakes the running render loop so it picks up a pending route or quit.
    /// iocraft forwards only key, mouse and resize events, so the wake is a
    /// resize to the size the app already has — the same no-op [`launch`]
    /// pushes.
    ///
    /// [`launch`]: Session::launch
    fn wake(&mut self) {
        crossterm::push_event(Event::Resize(self.cols, self.rows));
        self.drive();
    }
}

#[wasm_bindgen(js_name = start)]
pub fn sam_start(cols: u16, rows: u16, path: &str, touch: bool) {
    SESSION.with(|cell| {
        let session = &mut *cell.borrow_mut();
        let session = session.insert(Session::new(cols, rows));
        crossterm::set_size(cols, rows);
        crate::reset_route_sync();
        if crate::has_view(path) {
            // A visitor who arrived at a view asked for it by name: open it,
            // with no banner and nothing to press.
            crate::request_route(path);
            session.launch();
        } else {
            // Otherwise open the shell; the app is one `dev-sam` away, already
            // typed at the prompt.
            let banner = session.editor.banner(touch);
            session.write(&banner);
        }
    });
}

#[wasm_bindgen(js_name = input)]
pub fn sam_input(bytes: &[u8]) {
    crossterm::push_input(bytes);
    with_session(Session::drive);
}

#[wasm_bindgen(js_name = resize)]
pub fn sam_resize(cols: u16, rows: u16) {
    with_session(|session| {
        session.cols = cols;
        session.rows = rows;
        crossterm::set_size(cols, rows);
        crossterm::push_event(Event::Resize(cols, rows));
        session.drive();
    });
}

/// Takes the ANSI bytes written since the last drain.
#[wasm_bindgen(js_name = drain)]
pub fn sam_drain() -> String {
    with_session(|session| {
        let taken = std::mem::take(&mut *session.output.lock().unwrap());
        String::from_utf8_lossy(&taken).into_owned()
    })
    .unwrap_or_default()
}

#[wasm_bindgen(js_name = navigate)]
pub fn sam_navigate(path: &str) {
    with_session(|session| session.go_to(path));
}

/// Activates a link the terminal printed — an OSC 8 hyperlink, or a bare URL
/// the host's link addon spotted. One naming a view of this app is followed
/// here; everything else comes back as [`crate::HostEvent::Open`].
#[wasm_bindgen(js_name = openLink)]
pub fn sam_open_link(url: &str) {
    with_session(|session| match crate::link_target(url) {
        crate::LinkTarget::View(path) => session.go_to(&path),
        crate::LinkTarget::External(url) => crate::push_host_event(crate::HostEvent::Open(url)),
        crate::LinkTarget::Ignore => {}
    });
}

/// Takes the next thing only the browser can do, as
/// `open <url>` or `route push|replace <path>\t<title>`.
#[wasm_bindgen(js_name = pollEvent)]
pub fn sam_poll_event() -> Option<String> {
    crate::poll_host_event().map(|event| event.encode())
}

/// Where the current frame drew its artwork, one row per image, as
/// `"x y cols rows top right bottom left url"` in canvas cells. The app owns
/// the alternate screen, so cell (0, 0) is the top left of the viewport and the
/// host can place an `<img>` straight onto it.
///
/// The four sides are what the pane's clipping took off: a card scrolled half
/// off the bottom paints only some of its artwork, and the overlay has to crop
/// to match. Images with nothing on screen are omitted. The rectangle is the
/// whole picture, so its origin goes negative for one scrolled part-way off the
/// top of a pane.
///
/// Space-separated rather than JSON: no asset path contains a space, and a
/// serializer would cost the wasm binary more than the artwork itself does.
#[wasm_bindgen(js_name = imageRegions)]
pub fn sam_image_regions() -> Vec<String> {
    crate::image::regions()
        .into_iter()
        .map(|region| {
            let (top, right, bottom, left) = region.insets();
            format!(
                "{} {} {} {} {top} {right} {bottom} {left} {}",
                region.x, region.y, region.cols, region.rows, region.url,
            )
        })
        .collect()
}
