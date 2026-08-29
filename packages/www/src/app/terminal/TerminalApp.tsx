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
// full-resolution overlay the backend cannot place itself, and `gestures` turns
// touch into the mouse reports xterm would send for a real one. `backend` is the
// bridge, including the browser errands the backend asks for through
// `pollEvent` — the URL bar, the document title, opening a tab.
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

  /** Drains everything the backend produced, plus the errands it asked for. */
  const pump = (): void => {
    if (backend == null) {
      return;
    }
    const output = backend.drain();
    if (output !== "") {
      screen.terminal.write(output);
    }
    artwork.sync(backend.imageRegions(), screen.metrics());
    for (let event = backend.pollEvent(); event != null; event = backend.pollEvent()) {
      applyHostEvent(event);
    }
  };

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

  const refocus = (): void => screen.terminal.focus();
  container.addEventListener("click", refocus);

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
    unbindGestures = bindGestures(container, screen, { wheelRows: backend.wheelRows(), send });
    // A visitor who arrived at a view asked for it by name, and gets it with no
    // banner and nothing to press; everyone else lands at the shell, with
    // `dev-sam` already typed at the prompt.
    const { cols, rows } = screen.fit();
    backend.start(cols, rows, currentPath(), touchOnly);
    if (touchOnly) {
      // Nothing on a phone can press that Enter — the keyboard is off — so the
      // prompt runs the pre-typed command itself.
      send("\r");
    }
    pump();
    screen.terminal.focus();
  };
  void boot();

  return {
    dispose() {
      disposed = true;
      unbindGestures?.();
      container.removeEventListener("click", refocus);
      window.removeEventListener("popstate", onPopState);
      resizeObserver.disconnect();
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
    <div className="fixed inset-0 overflow-hidden bg-[#f7f7f7]">
      <div
        ref={containerRef}
        className="relative h-full w-full p-1"
        aria-label="Developer Sam's portfolio as a full-screen terminal app"
        role="application"
      />
    </div>
  );
}
