"use client";

import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { useEffect, useRef } from "react";
import { loadSamTui, type SamTui } from "./sam-tui";

// The page is a true terminal: xterm.js relays raw bytes to and from the
// iocraft backend, which speaks full ANSI (alternate screen, mouse capture,
// synchronized updates). No frame protocol, no key mapping, and no second
// implementation of anything the backend already knows — the shell prompt, the
// blog's titles and where a link leads are all its.
//
// So this file is the host and nothing more: it owns the xterm instance, the
// things only a browser can do (the URL bar, opening a tab, touch), and the
// artwork overlay, which needs cell metrics the backend cannot see.
//
// The URL bar is the app's: every view the backend can be in is a path the site
// can be entered at, so a permalink opens its post and moving around the app
// rewrites the URL through the History API without ever loading a second
// document. `App::route` and `App::go_to` in crates/sam-tui/src/lib.rs are the
// other half of it.
//
// Card artwork is the one thing layered on top. The backend already draws it
// in-band as truecolor half-blocks — that is what the native `dev-sam` binary
// shows — and reports where each one landed. Here we cover those cells with the
// real asset, so the web gets full resolution and the half-blocks become the
// fallback if an image fails to load.

declare global {
  interface Window {
    /** Test hook: the xterm instance driving this page, for cell-exact automation. */
    __samTerminal?: Terminal;
  }
}

/** The path in the URL bar, with any trailing slash trimmed off. */
function currentPath(): string {
  const path = window.location.pathname;
  return path.length > 1 ? path.replace(/\/+$/, "") : path;
}

/**
 * Phones and tablets, where the keyboard is an overlay that eats half the
 * viewport. A laptop with a touchscreen is not one of these: it has a real
 * keyboard, so focusing the terminal there costs the visitor nothing.
 */
function isTouchOnlyDevice(): boolean {
  return window.matchMedia("(hover: none) and (pointer: coarse)").matches;
}

function run(container: HTMLDivElement): { dispose(): void } {
  const touchOnlyDevice = isTouchOnlyDevice();

  const terminal = new Terminal({
    // The shell writes its URLs as OSC 8 hyperlinks — the `@` tags of
    // `cat about.txt`, the contact list, resume.pdf. xterm draws those as
    // plain text unless it is told what activating one means.
    linkHandler: { activate: (_event, uri) => openLink(uri) },
    scrollback: 1000,
    cursorBlink: true,
    fontSize: 16,
    convertEol: false,
    drawBoldTextInBrightColors: false,
    fontFamily:
      'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
    theme: {
      // The site's light mode: body #f7f7f7, white cards, blue-500 accent.
      background: "#f7f7f7",
      foreground: "#1c1e21",
      black: "#1c1e21",
      red: "#c33b30",
      green: "#1a8f52",
      yellow: "#b45309",
      blue: "#3e7ae2",
      magenta: "#9a30ad",
      cyan: "#0e7490",
      white: "#f7f7f7",
      cursor: "#1c1e21",
      cursorAccent: "#f7f7f7",
      brightBlack: "#6b7280",
      brightWhite: "#ffffff",
    },
  });
  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  // Bare URLs in the shell's output are links too, not just the OSC 8 ones.
  terminal.loadAddon(new WebLinksAddon((_event, uri) => openLink(uri)));
  terminal.open(container);
  fitAddon.fit();
  // xterm.css hardcodes a black viewport; the site body is #f7f7f7.
  const viewportStyle = document.createElement("style");
  viewportStyle.textContent = ".xterm .xterm-viewport { background-color: #f7f7f7 !important; }";
  document.head.appendChild(viewportStyle);
  // oxlint-disable-next-line no-underscore-dangle -- test hook
  window.__samTerminal = terminal;

  // On a phone, focus is what summons the keyboard, and xterm takes focus
  // itself on every tap — it has to, that is how a terminal works. Marking
  // the field it focuses read-only is what phones actually honour: the
  // keyboard stays away while the field still takes focus, so taps, mouse
  // reports and a paired hardware keyboard all keep working.
  if (touchOnlyDevice && terminal.textarea != null) {
    terminal.textarea.readOnly = true;
  }

  // Try WebGL: its renderer draws block elements procedurally, so the
  // half-block artwork tiles seamlessly. The DOM renderer leaves hairline
  // gaps between rows because it depends on font metrics and line height.
  try {
    terminal.loadAddon(new WebglAddon());
  } catch {
    // No WebGL here; the DOM renderer still draws everything, just seamed.
  }

  let disposed = false;
  let backend: SamTui | null = null;

  /**
   * Hands a link the terminal printed to the backend, which knows whether it
   * names one of its own views. One that does is followed in place; anything
   * else comes back as an `open` event.
   */
  const openLink = (url: string): void => {
    backend?.openLink(url);
    pump();
  };

  /** The screen's cell grid in px, plus where it sits in the container. */
  type CellMetrics = {
    width: number;
    height: number;
    offsetLeft: number;
    offsetTop: number;
    rect: DOMRect;
  };

  const cellMetrics = (): CellMetrics | null => {
    const screen = container.querySelector(".xterm-screen");
    if (screen == null) {
      return null;
    }
    const rect = screen.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
      return null;
    }
    const containerRect = container.getBoundingClientRect();
    return {
      width: rect.width / terminal.cols,
      height: rect.height / terminal.rows,
      offsetLeft: rect.left - containerRect.left,
      offsetTop: rect.top - containerRect.top,
      rect,
    };
  };

  // --- Card artwork ---------------------------------------------------------
  const imageLayer = document.createElement("div");
  Object.assign(imageLayer.style, {
    position: "absolute",
    inset: "0",
    pointerEvents: "none",
    overflow: "hidden",
  });
  container.appendChild(imageLayer);
  // Pooled per URL: an image that stays on screen across frames is moved
  // rather than torn down and refetched, since rebuilding the layer every
  // frame would flicker at scroll rate. A pool rather than one element per
  // URL because an asset can appear more than once at the same time —
  // `flow.webp` and `graduation-sam.webp` each illustrate two timeline
  // events, and both can be on screen together.
  const pool = new Map<string, HTMLImageElement[]>();

  const syncImages = (): void => {
    const regions = backend?.imageRegions() ?? [];
    if (regions.length === 0 && pool.size === 0) {
      return;
    }
    const cell = cellMetrics();
    if (cell == null) {
      return;
    }

    const used = new Map<string, number>();
    for (const region of regions) {
      // "x y cols rows top right bottom left url" in canvas cells; the app owns
      // the alternate screen, so cell (0, 0) is the top left of the viewport.
      const [x, y, cols, rows, top, right, bottom, left, ...rest] = region.split(" ");
      const url = rest.join(" ");
      if (url === "") {
        continue;
      }
      const index = used.get(url) ?? 0;
      used.set(url, index + 1);
      let elements = pool.get(url);
      if (elements == null) {
        elements = [];
        pool.set(url, elements);
      }
      let element = elements[index];
      if (element == null) {
        element = document.createElement("img");
        element.src = url;
        element.alt = "";
        Object.assign(element.style, { position: "absolute", objectFit: "cover" });
        elements.push(element);
        imageLayer.appendChild(element);
      }
      // Placed at the full rectangle so the picture keeps its shape, then
      // clipped by what the pane's clipping took off each side — otherwise a
      // card half off the bottom would spill its artwork over the status bar.
      // The rectangle can start above the viewport, at a negative y, when a
      // post is scrolled part-way through one of its images.
      const inset = [
        Number(top) * cell.height,
        Number(right) * cell.width,
        Number(bottom) * cell.height,
        Number(left) * cell.width,
      ];
      Object.assign(element.style, {
        left: `${cell.offsetLeft + Number(x) * cell.width}px`,
        top: `${cell.offsetTop + Number(y) * cell.height}px`,
        width: `${Number(cols) * cell.width}px`,
        height: `${Number(rows) * cell.height}px`,
        clipPath: inset.every((side) => side === 0)
          ? "none"
          : `inset(${inset.map((side) => `${side}px`).join(" ")})`,
      });
    }
    // Retire whatever this frame did not place — a card that scrolled off, a
    // duplicate that is no longer doubled up, or the whole set at once when the
    // app exits and the shell reports no artwork at all.
    for (const [url, elements] of pool) {
      const keep = used.get(url) ?? 0;
      for (const element of elements.splice(keep)) {
        element.remove();
      }
      if (elements.length === 0) {
        pool.delete(url);
      }
    }
  };

  // --- Host events ----------------------------------------------------------
  /**
   * Acts on one line from `pollEvent` — the things the backend cannot do for
   * itself. `router.replace` is a no-op under `output: "export"` (see the same
   * note in budget/Tabs.tsx), and the History API is what a terminal wants
   * anyway: no navigation, no reload, just the address kept honest.
   */
  const onHostEvent = (event: string): void => {
    const [head, title] = event.split("\t");
    const [kind, ...rest] = (head ?? "").split(" ");
    if (kind === "open") {
      // Only http(s) leaves: the page renders whatever bytes reach it, and a
      // `javascript:` URL would run in this document. The backend refuses those
      // too — this is the guard at the one place that actually navigates.
      const url = rest.join(" ");
      if (/^https?:\/\//i.test(url)) {
        window.open(url, "_blank", "noopener");
      }
      return;
    }
    if (kind !== "route") {
      return;
    }
    const [mode, path] = rest;
    if (path == null) {
      return;
    }
    if (path !== currentPath()) {
      window.history[mode === "replace" ? "replaceState" : "pushState"](null, "", path);
    }
    document.title = title ?? "";
  };

  /** Pump everything the backend produced into xterm, plus side effects. */
  const pump = (): void => {
    if (backend == null) {
      return;
    }
    const output = backend.drain();
    if (output !== "") {
      terminal.write(output);
    }
    syncImages();
    for (let event = backend.pollEvent(); event != null; event = backend.pollEvent()) {
      onHostEvent(event);
    }
  };

  // Back and forward move the app, rather than the document: the browser has
  // nowhere else to go, since every route is this same page. A path the app has
  // no view for means the visitor left it, which the backend answers by exiting
  // to the shell.
  const handlePopState = (): void => {
    backend?.navigate(currentPath());
    pump();
  };
  window.addEventListener("popstate", handlePopState);

  // --- Input ----------------------------------------------------------------
  // Every byte xterm produces is forwarded verbatim — keys AND mouse reports,
  // at the shell prompt as much as in the app, because the backend is a real
  // terminal at both. There is no key handling on this side of the bridge.
  const encoder = new TextEncoder();
  terminal.onData((data) => {
    if (backend == null) {
      return;
    }
    backend.input(encoder.encode(data));
    pump();
  });

  const refocus = (): void => terminal.focus();
  container.addEventListener("click", refocus);

  // --- Touch ----------------------------------------------------------------
  // While the app is up, a touch gesture is read here rather than left to the
  // browser, because neither half of it survives the trip otherwise.
  //
  // Scrolling: a phone has no wheel, and there is nothing for the browser's
  // own scrolling to move either — the app owns the alternate screen, so
  // every row on it comes from the backend, which scrolls only when a wheel
  // tells it to. A drag is metered out as wheel notches, one per WHEEL_ROWS
  // rows, the same distance the backend moves for one, which keeps the
  // content under the finger.
  //
  // Tapping: xterm only reports a click when the browser synthesizes the
  // mouse events for one, and a phone does that late, grudgingly, and not at
  // all if the finger drifted — which is what made taps need a second go. So
  // the tap is reported straight from the gesture instead, and the whole
  // gesture is claimed at touchstart so no synthesized click doubles it.
  const WHEEL_ROWS = 3;
  /** How far a finger may drift, in px, and still have meant a tap. */
  const TAP_SLOP = 12;
  /** And for how long, in ms — past this it was a press, not a tap. */
  const TAP_TIME = 700;

  type Gesture = { x: number; y: number; dragY: number; at: number; tap: boolean };
  let gesture: Gesture | null = null;

  /** The cell under a viewport point, 1-based, as a mouse report writes it. */
  const cellAt = (x: number, y: number): { col: number; row: number } | null => {
    const cell = cellMetrics();
    if (cell == null) {
      return null;
    }
    const col = Math.floor((x - cell.rect.left) / cell.width);
    const row = Math.floor((y - cell.rect.top) / cell.height);
    if (col < 0 || row < 0 || col >= terminal.cols || row >= terminal.rows) {
      return null;
    }
    return { col: col + 1, row: row + 1 };
  };

  const handleTouchStart = (event: TouchEvent): void => {
    const touch = event.touches.length === 1 ? event.touches[0] : undefined;
    // Two fingers is a pinch; that one is the browser's to handle.
    if (touch == null) {
      gesture = null;
      return;
    }
    gesture = {
      x: touch.clientX,
      y: touch.clientY,
      dragY: touch.clientY,
      at: event.timeStamp,
      tap: true,
    };
    // The gesture is the app's from here: no synthesized mouse events for
    // it, no rubber-banding the page behind it, and no pull-to-refresh from
    // a drag that starts at the top. Focus comes with it, since the click
    // that used to carry it is one of the events just suppressed.
    event.preventDefault();
    terminal.focus();
  };

  const handleTouchMove = (event: TouchEvent): void => {
    const y = event.touches[0]?.clientY;
    const x = event.touches[0]?.clientX;
    if (backend == null || gesture == null || x == null || y == null) {
      return;
    }
    if (Math.abs(x - gesture.x) > TAP_SLOP || Math.abs(y - gesture.y) > TAP_SLOP) {
      gesture.tap = false;
    }
    const cell = cellMetrics();
    if (cell == null) {
      return;
    }
    const notchHeight = cell.height * WHEEL_ROWS;
    event.preventDefault();
    const notches = Math.trunc((gesture.dragY - y) / notchHeight);
    if (notches === 0) {
      return;
    }
    // Keep the remainder, so a slow drag still accumulates into a notch.
    gesture.dragY -= notches * notchHeight;
    // SGR wheel reports, the same ones xterm sends for a real wheel: button
    // 64 is a notch up, 65 down. A wheel carries a position too, which the
    // app ignores, so the top left cell stands in for the finger.
    const report = notches > 0 ? "\x1b[<65;1;1M" : "\x1b[<64;1;1M";
    backend.input(encoder.encode(report.repeat(Math.abs(notches))));
    pump();
  };

  const handleTouchEnd = (event: TouchEvent): void => {
    const finished = gesture;
    gesture = null;
    if (backend == null || finished == null || !finished.tap) {
      return;
    }
    if (event.timeStamp - finished.at > TAP_TIME) {
      return;
    }
    // Where the finger landed, not where it left: within the slop they are
    // the same cell, and the landing is what the visitor aimed at.
    const cell = cellAt(finished.x, finished.y);
    if (cell == null) {
      return;
    }
    // An SGR press and release of the left button, the pair xterm sends for
    // a real click. The app acts on the press; the release keeps the
    // backend's button state honest.
    const { col, row } = cell;
    backend.input(encoder.encode(`\x1b[<0;${col};${row}M\x1b[<0;${col};${row}m`));
    pump();
  };

  const handleTouchCancel = (): void => {
    gesture = null;
  };

  container.addEventListener("touchstart", handleTouchStart, { passive: false });
  container.addEventListener("touchmove", handleTouchMove, { passive: false });
  container.addEventListener("touchend", handleTouchEnd);
  container.addEventListener("touchcancel", handleTouchCancel);

  const resizeObserver = new ResizeObserver(() => {
    fitAddon.fit();
    if (backend != null) {
      backend.resize(terminal.cols, terminal.rows);
      pump();
    }
  });
  resizeObserver.observe(container);

  const boot = async (): Promise<void> => {
    try {
      backend = await loadSamTui();
    } catch (error) {
      terminal.write(`\x1b[31mfailed to load sam-tui.wasm:\x1b[0m ${String(error)}\r\n`);
      return;
    }
    if (disposed) {
      return;
    }
    // A visitor who arrived at a view asked for it by name, and gets it with no
    // banner and nothing to press; everyone else lands at the shell, with
    // `dev-sam` already typed at the prompt.
    backend.start(terminal.cols, terminal.rows, currentPath(), touchOnlyDevice);
    if (touchOnlyDevice) {
      // Nothing on a phone can press that Enter — the keyboard is off — so the
      // prompt runs the pre-typed command itself.
      backend.input(encoder.encode("\r"));
    }
    pump();
    terminal.focus();
  };
  void boot();

  return {
    dispose() {
      disposed = true;
      container.removeEventListener("click", refocus);
      container.removeEventListener("touchstart", handleTouchStart);
      container.removeEventListener("touchmove", handleTouchMove);
      container.removeEventListener("touchend", handleTouchEnd);
      container.removeEventListener("touchcancel", handleTouchCancel);
      window.removeEventListener("popstate", handlePopState);
      resizeObserver.disconnect();
      viewportStyle.remove();
      imageLayer.remove();
      // oxlint-disable-next-line no-underscore-dangle -- test hook
      delete window.__samTerminal;
      terminal.dispose();
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
