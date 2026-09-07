//! WebAssembly surface: one terminal session, driven entirely by bytes. The
//! host (xterm.js) pushes raw input and drains raw ANSI output — a true
//! terminal bridge, with no frame protocol and no key mapping.
//!
//! A session is in one of three modes. It opens at the `dev-sam-sh` prompt,
//! where [`LineEditor`] edits the line and [`Shell`] runs the commands;
//! `dev-sam` hands the same byte stream to the full-screen iocraft app, and
//! quitting it hands it back. `dev-sam --touch` draws the same views as a page
//! instead — see [`Mode::Page`]. All three write into the same buffer, so
//! [`sam_drain`] never has to know which is up.
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
//! - `tap(col, row)` — a tap on the touch page, in the page's own cells;
//! - `pollEvent()` — the next thing only the browser can do;
//! - `wheelRows()` — how far one wheel notch scrolls;
//! - `imageRegions()` — where this frame drew its artwork.

use crate::shell::{Launch, LineEditor, Shell};
use crate::site_path::SitePath;
use crate::view;
use crossterm::cursor::{Hide, MoveTo};
use crossterm::event::Event;
use crossterm::terminal::{Clear, ClearType};
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
    /// The touch build, showing the view at this path. There is no engine and
    /// no frame loop: the view is drawn once, whole, into the terminal the host
    /// already has, and the host scrolls it from there. Reaching another view
    /// is a navigation the browser performs ([`crate::HostEvent::Navigate`]),
    /// so the only thing that ever redraws a page is a change of width.
    Page(SitePath),
}

struct Session {
    mode: Mode,
    shell: Shell,
    editor: LineEditor,
    /// Where every mode writes. Taken whole by [`sam_drain`].
    output: Arc<Mutex<Vec<u8>>>,
    cols: u16,
    rows: u16,
    /// Whether `dev-sam` was asked for the touch build. The host's device
    /// settles it when the session opens, and `--touch` at the prompt asks for
    /// it by hand.
    touch: bool,
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
            touch: false,
        }
    }

    fn write(&mut self, text: &str) {
        self.output
            .lock()
            .unwrap()
            .extend_from_slice(text.as_bytes());
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.output.lock().unwrap().extend_from_slice(bytes);
    }

    /// Runs `dev-sam`, in whichever build was asked for.
    fn launch(&mut self) {
        if self.touch {
            self.open_page();
        } else {
            self.launch_app();
        }
    }

    /// Opens the touch build at whatever view the host last asked for, or at
    /// the app's own first tab when it asked for nothing — the view the
    /// full-screen app opens on too.
    fn open_page(&mut self) {
        let path = crate::take_pending_route()
            .unwrap_or_else(|| SitePath::new(crate::TAB_ROUTES[crate::ABOUT_TAB]));
        self.mode = Mode::Page(path);
        self.paint_page();
    }

    /// Draws the page into the terminal the host already has, from the top.
    ///
    /// The screen and the scrollback above it are wiped first, so the page
    /// begins at the first row of the buffer and the host can turn a tap
    /// anywhere in it into a cell of the canvas by adding on how far it has
    /// scrolled. Nothing else is taken over: no alternate screen, no mouse
    /// capture, and every row stays where it was written, which is what lets
    /// the host scroll the page without the app drawing another frame.
    fn paint_page(&mut self) {
        let Mode::Page(path) = &self.mode else {
            return;
        };
        let mut element = view::touch_element(path.clone(), self.cols);
        let canvas = element.render(Some(usize::from(self.cols)));
        let mut ansi = format!(
            "{}{}{}{}",
            MoveTo(0, 0),
            Clear(ClearType::All),
            // The scrollback too: it is the page's own coordinate system.
            Clear(ClearType::Purge),
            // A page has no cursor to put anywhere, and one left blinking at
            // the end of the last line reads as a prompt that is not there.
            Hide,
        )
        .into_bytes();
        // Infallible: a `Vec` never fails to take bytes.
        let _ = canvas.write_ansi(&mut ansi);
        self.write_bytes(&ansi);
    }

    /// Boots the full-screen app, which takes the byte stream over from the
    /// shell. Restarting after a quit leaks a fresh tree rather than reviving
    /// the finished one.
    fn launch_app(&mut self) {
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
            path: crate::site_path::SitePath::root(),
            title: crate::SHELL_TITLE.to_string(),
        });
        let screen = self.editor.after_dev_sam_app_exit();
        self.write(&screen);
    }

    /// Consumes whatever input the host has just pushed.
    fn drive(&mut self) {
        match &mut self.mode {
            Mode::App(engine) => {
                engine.pump();
                self.settle();
            }
            Mode::Shell => self.edit(),
            // A page has no state a key could move, and the host it is drawn
            // for has no keyboard to press one with. Whatever arrived is read
            // off and dropped rather than left to pile up behind the page.
            Mode::Page(_) => {
                while matches!(crossterm::event::poll(Duration::ZERO), Ok(true)) {
                    if crossterm::event::read().is_err() {
                        return;
                    }
                }
            }
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
            let (ansi, launch) = self.editor.handle_key(key, &mut self.shell);
            self.write(&ansi);
            if let Some(Launch { touch }) = launch {
                // Whatever is still queued belongs to the app, not the shell.
                self.touch = touch;
                self.launch();
            }
        }
    }

    /// Shows the view a site path names: the URL the page was entered at, a
    /// link followed inside a post, or wherever the back button just went. A
    /// path that is no view of this app means the visitor has left it.
    fn go_to(&mut self, path: &str) {
        let Some(path) = crate::site_path::SitePath::parse(path).filter(crate::has_view) else {
            if matches!(self.mode, Mode::App(_)) {
                crate::request_quit();
                self.wake();
            }
            return;
        };
        crate::request_route(&path);
        match self.mode {
            Mode::App(_) => self.wake(),
            // The touch build normally never gets here — it moves between views
            // by asking the browser to load one — but a restored history entry
            // can still name a view, and drawing it is the whole answer.
            Mode::Page(_) | Mode::Shell => self.launch(),
        }
    }

    /// Activates a link the terminal printed. The touch build reaches another
    /// view by loading it, whether the finger landed on a tab or on a bare URL
    /// the host's link addon spotted, so this is the same errand a tap makes.
    fn follow_link(&mut self, url: &str) {
        if matches!(self.mode, Mode::Page(_)) {
            if let Some(event) = crate::errand_for(url) {
                crate::push_host_event(event);
            }
            return;
        }
        match crate::link_target(url) {
            crate::LinkTarget::View(path) => self.go_to(path.as_str()),
            crate::LinkTarget::External(url) => {
                crate::push_host_event(crate::HostEvent::Open(url));
            }
            crate::LinkTarget::Ignore => {}
        }
    }

    /// What a tap on the page means, in the page's own cells. Only the touch
    /// build takes taps: everywhere else a pointer is a mouse, and the host
    /// reports one as the bytes a terminal expects.
    fn tap(&self, col: u16, row: u16) {
        let Mode::Page(path) = &self.mode else {
            return;
        };
        // The regions were recorded by the paint the visitor is looking at;
        // this rebuilds only the state needed to say what the one under the
        // finger leads to.
        if let Some(event) = crate::App::page(path, self.cols).tap(col, row) {
            crate::push_host_event(event);
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
        // A host with no keyboard gets the touch build, whether it reaches it
        // through the pre-typed `dev-sam --touch` or straight from a URL.
        session.touch = touch;
        crossterm::set_size(cols, rows);
        crate::reset_route_sync();
        // A visitor who arrived at a view asked for it by name: open it, with
        // no banner and nothing to press. Anything else the browser may have
        // handed over is not this app's to serve, so the shell opens instead.
        if let Some(path) = crate::site_path::SitePath::parse(path).filter(crate::has_view) {
            crate::request_route(&path);
            session.launch();
        } else {
            let screen = session.editor.opening_screen(touch);
            session.write(&screen);
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
        let narrower_or_wider = session.cols != cols;
        session.cols = cols;
        session.rows = rows;
        crossterm::set_size(cols, rows);
        if matches!(session.mode, Mode::Page(_)) {
            // A page is exactly as tall as what it holds, so only its width can
            // change a line of it. Redrawing for every height the browser
            // reports — and a phone reports a new one each time its address bar
            // slides away under a scrolling finger — would put the page back at
            // the top mid-scroll, and cost the render the page exists to avoid.
            if narrower_or_wider {
                session.paint_page();
            }
            return;
        }
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
    with_session(|session| session.follow_link(url));
}

/// A tap on the touch page, in the page's own cells: column and row counted
/// from the top left of what the app drew, not of the viewport. The page has
/// scrolled under the finger without the app being told — that is the point of
/// it — so the host adds on how far and reports where the finger really landed.
#[wasm_bindgen(js_name = tap)]
pub fn sam_tap(col: u16, row: u16) {
    with_session(|session| session.tap(col, row));
}

/// Takes the next thing only the browser can do, as `open <url>`,
/// `navigate <path>`, or `route push|replace <path>\t<title>`.
#[wasm_bindgen(js_name = pollEvent)]
pub fn sam_poll_event() -> Option<String> {
    crate::poll_host_event().map(|event| event.encode())
}

/// Rows one wheel notch scrolls. A host with no wheel — a phone metering a
/// drag — has to know the distance the app moves for one to keep the content
/// under the finger.
#[wasm_bindgen(js_name = wheelRows)]
pub fn sam_wheel_rows() -> u16 {
    crate::WHEEL_ROWS as u16
}

/// Where the current frame drew its artwork, one row per image, as
/// `"x y cols rows top right bottom left url"` in canvas cells. Cell (0, 0) is
/// the top left of the canvas: the viewport, for the app that owns the
/// alternate screen, and the first row of the page for the touch build, which
/// the host offsets by however far it has scrolled.
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
