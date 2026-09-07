"use client";

import { useEffect, useRef } from "react";
import { mountArtwork } from "./artwork";
import { applyHostEvent, currentPath, loadBackend, type Backend } from "./backend";
import { bindGestures } from "./gestures";
import { openScreen } from "./screen";

// The page is a true terminal: xterm.js relays raw bytes to and from the
// iocraft backend, which speaks full ANSI (alternate screen, mouse capture,
// synchronized updates). No frame protocol, no key mapping.
//
// So the browser's side of this is only what a terminal emulator does, split
// three ways: `screen` is the emulator and its geometry, `artwork` is the
// full-resolution overlay the backend cannot place itself, and `gestures` tells
// a tap from a drag. `backend` is the bridge, including the browser errands the
// backend asks for through `pollEvent` — the URL bar, the document title,
// opening a tab, loading a page.
//
// A phone gets a different backend build (`dev-sam --touch`), and it is the one
// thing here that changes what this file does. That build draws a whole view
// into the terminal's own scrollback instead of owning the screen, so a drag
// scrolls the emulator rather than asking the backend for the next screenful —
// no frame per row, and in exchange a tap that has to be counted from the top
// of the page rather than the top of the viewport.
//
// Everything else is the backend's: the shell prompt, the blog's titles, where
// a link leads. This file just wires the two together and pumps.

/**
 * Phones and tablets, where the keyboard is an overlay that eats half the
 * viewport. A laptop with a touchscreen is not one of these: it has a real
 * keyboard, so focusing the terminal there costs the visitor nothing.
 */
function isTouchOnlyDevice(): boolean {
  return window.matchMedia("(hover: none) and (pointer: coarse)").matches;
}

function run(container: HTMLDivElement): { dispose(): void } {
  const touchOnly = isTouchOnlyDevice();
  const encoder = new TextEncoder();

  let disposed = false;
  let backend: Backend | null = null;

  const screen = openScreen(container, {
    touchOnly,
    // The backend knows whether a URL names one of its own views. One that does
    // is followed in place; anything else comes back as an `open` event.
    onLink: (url) => {
      backend?.openLink(url);
      pump();
    },
  });
  const artwork = mountArtwork(container);

  // The last frame's artwork, kept because it outlives the frame that reported
  // it: the touch build draws its page once and the visitor then scrolls it,
  // which moves every picture on it without the backend drawing anything.
  let regions: string[] = [];
  const place = (): void => artwork.sync(regions, screen.metrics(), screen.scrollRows());
  const openPage = (): void => {
    screen.terminal.scrollToTop();
    place();
  };

  /** Drains everything the backend produced, plus the errands it asked for. */
  const pump = (): void => {
    if (backend == null) {
      return;
    }
    const output = backend.drain();
    if (output !== "") {
      // xterm parses a write on its own schedule, and leaves the viewport at
      // the bottom of whatever it was given. That is what a terminal does and
      // what the shell wants; a page is read from the top, so the touch build
      // asks for it back — once the write it is about has actually landed,
      // which is also when its artwork can be placed against the real buffer.
      screen.terminal.write(output, touchOnly ? openPage : undefined);
    }
    regions = backend.imageRegions();
    place();
    for (let event = backend.pollEvent(); event != null; event = backend.pollEvent()) {
      applyHostEvent(event);
    }
  };

  // Scrolling is the one thing that moves the picture without moving the app.
  const scroll = screen.terminal.onScroll(place);

  /** Hands raw input bytes to the backend and shows what came back. */
  const send = (bytes: string): void => {
    backend?.input(encoder.encode(bytes));
    pump();
  };

  // Every byte xterm produces is forwarded verbatim — keys AND mouse reports, at
  // the shell prompt as much as in the app, because the backend is a real
  // terminal at both. There is no key handling on this side of the bridge.
  const input = screen.terminal.onData(send);

  // Back and forward move the app, rather than the document: the browser has
  // nowhere else to go, since every route is this same page. A path the app has
  // no view for means the visitor left it, which the backend answers by exiting
  // to the shell.
  const onPopState = (): void => {
    backend?.navigate(currentPath());
    pump();
  };
  window.addEventListener("popstate", onPopState);

  // Focus is what a keyboard needs, so it is only worth taking where there is
  // one: the touch build reads its page with taps, and focusing would summon
  // nothing but a scroll to wherever the cursor was left.
  const refocus = (): void => screen.terminal.focus();
  if (!touchOnly) {
    container.addEventListener("click", refocus);
  }

  const resizeObserver = new ResizeObserver(() => {
    const { cols, rows } = screen.fit();
    backend?.resize(cols, rows);
    pump();
  });
  resizeObserver.observe(container);

  let unbindGestures: (() => void) | null = null;

  const boot = async (): Promise<void> => {
    try {
      backend = await loadBackend();
    } catch (error) {
      screen.terminal.write(`\x1b[31mfailed to load sam-tui.wasm:\x1b[0m ${String(error)}\r\n`);
      return;
    }
    if (disposed) {
      return;
    }
    const app = backend;
    unbindGestures = bindGestures(container, screen, {
      drag: touchOnly
        ? {
            // The page is already in the emulator's scrollback, so moving it is
            // the emulator's own business — by the row, and without the backend
            // hearing about it at all.
            rows: 1,
            // The artwork follows through `onScroll`, which xterm fires from
            // inside this call — so placing it here as well would only be doing
            // the same work twice per row.
            scroll: (steps) => screen.terminal.scrollLines(steps),
          }
        : {
            // The backend owns every row of the alternate screen and moves them
            // only for a wheel, so a step is the notch it answers.
            rows: app.wheelRows(),
            // SGR wheel reports, the same ones xterm sends for a real wheel:
            // button 64 is a notch up, 65 down. A wheel carries a position too,
            // which the app ignores, so the top left cell stands in for the
            // finger.
            scroll: (steps) =>
              send((steps > 0 ? "\x1b[<65;1;1M" : "\x1b[<64;1;1M").repeat(Math.abs(steps))),
          },
      onTap: touchOnly
        ? (col, row) => {
            // The page has scrolled since it was drawn, so the cell under the
            // finger is that far down the canvas — and the backend counts from
            // zero, as a mouse report does once crossterm has parsed it.
            app.tap(col - 1, screen.scrollRows() + row - 1);
            pump();
          }
        : // An SGR press and release of the left button, the pair xterm sends
          // for a real click. The app acts on the press; the release keeps the
          // backend's button state honest.
          (col, row) => send(`\x1b[<0;${col};${row}M\x1b[<0;${col};${row}m`),
      focus: !touchOnly,
    });
    // A visitor who arrived at a view asked for it by name, and gets it with no
    // banner and nothing to press; everyone else lands at the shell, with
    // `dev-sam` already typed at the prompt.
    const { cols, rows } = screen.fit();
    app.start(cols, rows, currentPath(), touchOnly);
    if (touchOnly) {
      // Nothing on a phone can press that Enter — the keyboard is off — so the
      // prompt runs the pre-typed command itself, `--touch` and all.
      send("\r");
    }
    pump();
    if (!touchOnly) {
      screen.terminal.focus();
    }
  };
  void boot();

  return {
    dispose() {
      disposed = true;
      unbindGestures?.();
      container.removeEventListener("click", refocus);
      window.removeEventListener("popstate", onPopState);
      resizeObserver.disconnect();
      scroll.dispose();
      input.dispose();
      artwork.dispose();
      screen.dispose();
    },
  };
}

export default function TerminalApp(): React.JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (container == null) {
      return;
    }
    return run(container).dispose;
  }, []);

  return (
    // No padding: a character grid already leaves a remainder of its own, and
    // `screen.fit` centers the grid so that remainder falls evenly on either
    // side rather than piling up at the right and the bottom.
    <div className="fixed inset-0 overflow-hidden bg-[#f7f7f7]">
      <div
        ref={containerRef}
        className="relative h-full w-full"
        aria-label="Developer Sam's portfolio as a terminal app"
        role="application"
      />
    </div>
  );
}
