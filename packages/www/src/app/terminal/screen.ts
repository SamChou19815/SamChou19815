/**
 * The xterm.js screen: the terminal emulator half of the page.
 *
 * It owns the emulator and its geometry, and nothing about what is being shown
 * on it. The ANSI palette below is the emulator's colour table — the site's
 * light mode, matching the truecolor one the backend paints with in
 * `crates/sam-tui/src/theme.rs`.
 */

import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";

declare global {
  interface Window {
    /** Test hook: the xterm instance driving this page, for cell-exact automation. */
    __samTerminal?: Terminal;
  }
}

/** The screen's cell grid in px, plus where it sits inside the container. */
export type CellMetrics = {
  width: number;
  height: number;
  offsetLeft: number;
  offsetTop: number;
  rect: DOMRect;
};

export type Screen = {
  readonly terminal: Terminal;
  /** Refits to the container and reports the new size, if it changed. */
  fit(): { cols: number; rows: number };
  metrics(): CellMetrics | null;
  /** The cell under a viewport point, 1-based, as a mouse report writes it. */
  cellAt(x: number, y: number): { col: number; row: number } | null;
  dispose(): void;
};

export type ScreenOptions = {
  /**
   * Whether this is a phone or tablet, where the keyboard is an overlay that
   * eats half the viewport. A laptop with a touchscreen is not one of these.
   */
  touchOnly: boolean;
  /** What activating a link on the screen means. */
  onLink: (url: string) => void;
};

export function openScreen(container: HTMLDivElement, options: ScreenOptions): Screen {
  const terminal = new Terminal({
    // The shell writes its URLs as OSC 8 hyperlinks — the `@` tags of
    // `cat about.txt`, the contact list, resume.pdf. xterm draws those as
    // plain text unless it is told what activating one means.
    linkHandler: { activate: (_event, uri) => options.onLink(uri) },
    scrollback: 1000,
    cursorBlink: true,
    fontSize: 16,
    convertEol: false,
    drawBoldTextInBrightColors: false,
    fontFamily:
      'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
    theme: {
      // body #f7f7f7, white cards, blue-500 accent.
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
  terminal.loadAddon(new WebLinksAddon((_event, uri) => options.onLink(uri)));
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
  if (options.touchOnly && terminal.textarea != null) {
    terminal.textarea.readOnly = true;
  }

  // Try WebGL: its renderer draws box-drawing characters procedurally, so the
  // panes, dialogs and image frames join up. The DOM renderer leaves hairline
  // gaps between rows because it depends on font metrics and line height.
  try {
    terminal.loadAddon(new WebglAddon());
  } catch {
    // No WebGL here; the DOM renderer still draws everything, just seamed.
  }

  const metrics = (): CellMetrics | null => {
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

  return {
    terminal,
    fit() {
      fitAddon.fit();
      // The addon holds back 14px of width for a scrollbar, whatever is on
      // screen. The app is full-screen — it runs in the alternate screen, where
      // there is nothing to scroll back through — so those cells were showing
      // as a strip of page down the right edge. Measure the cell the addon just
      // settled on and take the width back; the shell keeps its scrollback, and
      // its scrollbar simply overlays a column it does not write to.
      const cell = metrics();
      if (cell != null && cell.width > 0 && cell.height > 0) {
        const cols = Math.max(2, Math.floor(container.clientWidth / cell.width));
        const rows = Math.max(2, Math.floor(container.clientHeight / cell.height));
        if (cols !== terminal.cols || rows !== terminal.rows) {
          terminal.resize(cols, rows);
        }
        // A whole number of cells almost never divides the window exactly. Left
        // alone the remainder all lands at the right and the bottom, which is a
        // margin the app cannot see and did not ask for — so split it, and the
        // app's own even margins stay even on the glass. The element's box is
        // still the container's, so this shifts the grid without changing what
        // the next fit measures.
        const element = terminal.element;
        if (element != null) {
          element.style.marginLeft = `${Math.floor((container.clientWidth - terminal.cols * cell.width) / 2)}px`;
          element.style.marginTop = `${Math.floor((container.clientHeight - terminal.rows * cell.height) / 2)}px`;
        }
      }
      return { cols: terminal.cols, rows: terminal.rows };
    },
    metrics,
    cellAt(x, y) {
      const cell = metrics();
      if (cell == null) {
        return null;
      }
      const col = Math.floor((x - cell.rect.left) / cell.width);
      const row = Math.floor((y - cell.rect.top) / cell.height);
      if (col < 0 || row < 0 || col >= terminal.cols || row >= terminal.rows) {
        return null;
      }
      return { col: col + 1, row: row + 1 };
    },
    dispose() {
      viewportStyle.remove();
      // oxlint-disable-next-line no-underscore-dangle -- test hook
      delete window.__samTerminal;
      terminal.dispose();
    },
  };
}
