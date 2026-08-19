"use client";

import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { useEffect, useRef } from "react";
import { loadSamTui, type LinkRegion, type SamTui } from "./sam-tui";

// Key discriminants matching crates/sam-tui/src/ffi.rs.
const KEY = {
  UP: 0,
  DOWN: 1,
  LEFT: 2,
  RIGHT: 3,
  ENTER: 4,
  ESC: 5,
  TAB: 6,
  BACKTAB: 7,
  BACKSPACE: 8,
  PAGE_UP: 9,
  PAGE_DOWN: 10,
  HOME: 11,
  END: 12,
  DELETE: 13,
  CHAR: 14,
} as const;

const MOD_CTRL = 1;
const MOD_SHIFT = 2;
const MOD_ALT = 4;

// Mouse kinds matching crates/sam-tui/src/ffi.rs.
const MOUSE_PRESS = 0;
const MOUSE_RELEASE = 1;
const MOUSE_SCROLL_UP = 2;
const MOUSE_SCROLL_DOWN = 3;

// The page reads as a shell session: one prompt line runs `dev-sam`, and the
// TUI renders below it as the command's output.
const PROMPT = "\x1b[1;32msam@developersam\x1b[0m\x1b[90m:\x1b[0m\x1b[34m~\x1b[0m\x1b[90m$\x1b[0m ";
const PROMPT_ROWS = 1;

/** Shifts every absolute cursor move in a frame down past the prompt line. */
function shiftFrame(frame: string): string {
  return frame.replace(
    // oxlint-disable-next-line no-control-regex -- frames position cells with ESC CSI sequences
    /\x1b\[(\d+);(\d+)H/g,
    (_match, row: string, col: string) => `\x1b[${Number.parseInt(row, 10) + PROMPT_ROWS};${col}H`,
  );
}

function mapKeyboardEvent(event: KeyboardEvent): { key: number; codepoint: number } | null {
  switch (event.key) {
    case "ArrowUp":
      return { key: KEY.UP, codepoint: 0 };
    case "ArrowDown":
      return { key: KEY.DOWN, codepoint: 0 };
    case "ArrowLeft":
      return { key: KEY.LEFT, codepoint: 0 };
    case "ArrowRight":
      return { key: KEY.RIGHT, codepoint: 0 };
    case "Enter":
      return { key: KEY.ENTER, codepoint: 0 };
    case "Escape":
      return { key: KEY.ESC, codepoint: 0 };
    case "Tab":
      return { key: event.shiftKey ? KEY.BACKTAB : KEY.TAB, codepoint: 0 };
    case "Backspace":
      return { key: KEY.BACKSPACE, codepoint: 0 };
    case "PageUp":
      return { key: KEY.PAGE_UP, codepoint: 0 };
    case "PageDown":
      return { key: KEY.PAGE_DOWN, codepoint: 0 };
    case "Home":
      return { key: KEY.HOME, codepoint: 0 };
    case "End":
      return { key: KEY.END, codepoint: 0 };
    case "Delete":
      return { key: KEY.DELETE, codepoint: 0 };
    default:
      if (event.key.length === 1) {
        return { key: KEY.CHAR, codepoint: event.key.codePointAt(0) ?? 0 };
      }
      return null;
  }
}

/**
 * Fallback input path: feeds keys that reach xterm through `onData` instead of
 * `keydown` — IME composition, paste, mobile keyboards and other environments
 * that synthesize input events. Keydown handling suppresses xterm's `onData`
 * for the same key, so this never double-fires on desktop browsers.
 */
function feedFallbackInput(data: string, key: (key: number, codepoint: number) => void): boolean {
  const sequences: Record<string, number> = {
    "\r": KEY.ENTER,
    "\t": KEY.TAB,
    "\x7f": KEY.BACKSPACE,
    "\x1b": KEY.ESC,
    "\x1b[A": KEY.UP,
    "\x1b[B": KEY.DOWN,
    "\x1b[C": KEY.RIGHT,
    "\x1b[D": KEY.LEFT,
    "\x1bOA": KEY.UP,
    "\x1bOB": KEY.DOWN,
    "\x1bOC": KEY.RIGHT,
    "\x1bOD": KEY.LEFT,
    "\x1b[5~": KEY.PAGE_UP,
    "\x1b[6~": KEY.PAGE_DOWN,
    "\x1b[H": KEY.HOME,
    "\x1b[F": KEY.END,
    "\x1bOH": KEY.HOME,
    "\x1bOF": KEY.END,
    "\x1b[1~": KEY.HOME,
    "\x1b[4~": KEY.END,
    "\x1b[3~": KEY.DELETE,
    "\x1b[Z": KEY.BACKTAB,
  };
  const special = sequences[data];
  if (special != null) {
    key(special, 0);
    return true;
  }
  if (data.startsWith("\x1b")) {
    // Unrecognized escape sequence: leave it alone.
    return false;
  }
  let handled = false;
  for (const character of data) {
    const codepoint = character.codePointAt(0) ?? 0;
    if (codepoint >= 0x20 && codepoint !== 0x7f) {
      key(KEY.CHAR, codepoint);
      handled = true;
    }
  }
  return handled;
}

/**
 * Splits every SGR mouse report (`\x1b[<b;x;yM/m`) out of an onData payload.
 * xterm can coalesce several reports (press+release, wheel bursts) into one
 * string, so they must be handled one by one; the remainder is returned for
 * the keyboard fallback path.
 */
function extractMouseReports(data: string): { reports: string[]; remainder: string } {
  const reports: string[] = [];
  let remainder = "";
  let last = 0;
  // oxlint-disable-next-line no-control-regex -- xterm mouse reports start with ESC
  const pattern = /\x1b\[<\d+;\d+;\d+[Mm]/g;
  for (const match of data.matchAll(pattern)) {
    const index = match.index ?? 0;
    reports.push(match[0]);
    remainder += data.slice(last, index);
    last = index + match[0].length;
  }
  remainder += data.slice(last);
  return { reports, remainder };
}

/** Dispatches one parsed SGR mouse report. Hover motion never repaints. */
function dispatchMouseReport(
  report_: string,
  report: (kind: number, button: number, col: number, row: number) => void,
  hover: (col: number, row: number) => void,
): boolean {
  // oxlint-disable-next-line no-control-regex -- xterm mouse reports start with ESC
  const match = /^\x1b\[<(\d+);(\d+);(\d+)([Mm])$/.exec(report_);
  if (match == null) {
    return false;
  }
  const raw = Number.parseInt(match[1] ?? "0", 10);
  const col = Number.parseInt(match[2] ?? "1", 10);
  const row = Number.parseInt(match[3] ?? "1", 10);
  const release = (match[4] ?? "M") === "m";
  if (raw >= 64) {
    report(raw === 64 ? MOUSE_SCROLL_UP : MOUSE_SCROLL_DOWN, 0, col, row);
    return true;
  }
  if (raw & 32) {
    hover(col - 1, row - 1);
    return false;
  }
  const button = raw & 3;
  report(release ? MOUSE_RELEASE : MOUSE_PRESS, button, col, row);
  return true;
}

declare global {
  interface Window {
    /** Test hook: the xterm instance driving this page, for cell-exact automation. */
    __samTerminal?: Terminal;
  }
}

export default function TerminalApp(): React.JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    if (container == null) {
      return;
    }

    const terminal = new Terminal({
      scrollback: 0,
      cursorBlink: true,
      fontSize: 15,
      convertEol: false,
      drawBoldTextInBrightColors: false,
      fontFamily:
        'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
      theme: {
        // The site's light mode: body #f7f7f7, white cards, blue-500 accent.
        background: "#f7f7f7",
        foreground: "#1c1e21",
        // A solid dark block cursor with inverted text, like a real terminal.
        cursor: "#1c1e21",
        cursorAccent: "#f7f7f7",
        black: "#1c1e21",
        red: "#c33b30",
        green: "#1a8f52",
        yellow: "#b45309",
        blue: "#3e7ae2",
        magenta: "#9a30ad",
        cyan: "#0e7490",
        white: "#f7f7f7",
        brightBlack: "#6b7280",
        brightRed: "#c33b30",
        brightGreen: "#1a8f52",
        brightYellow: "#b45309",
        brightBlue: "#3e7ae2",
        brightMagenta: "#9a30ad",
        brightCyan: "#0e7490",
        brightWhite: "#ffffff",
      },
    });
    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.open(container);
    fitAddon.fit();
    // oxlint-disable-next-line no-underscore-dangle -- test hook
    window.__samTerminal = terminal;
    // xterm.css hardcodes a black viewport; the site body is #f7f7f7.
    const viewportStyle = document.createElement("style");
    viewportStyle.textContent = ".xterm .xterm-viewport { background-color: #f7f7f7 !important; }";
    document.head.appendChild(viewportStyle);

    let disposed = false;
    let backend: SamTui | null = null;
    let links: LinkRegion[] = [];

    // The terminal boots into the shell; the app is one `dev-sam` away.
    let shellMode = true;
    let shellLine = "";
    const shellHistory: string[] = [];
    let shellHistoryIndex = 0;

    const runDevSam = (): void => {
      if (backend == null) {
        return;
      }
      shellMode = false;
      shellLine = "";
      terminal.options.cursorBlink = false;
      terminal.write(`\x1b[2J\x1b[H${PROMPT}dev-sam\r\n`);
      terminal.write("\x1b[?25l\x1b[?1003h\x1b[?1006h");
      backend.reset();
      backend.resize(terminal.cols, terminal.rows - PROMPT_ROWS);
      drainAndPaint();
      terminal.focus();
    };

    // Exits the app the way a real command would: release the mouse, drop
    // back to the shell prompt, and leave the transcript selectable.
    const exitToShell = (): void => {
      shellMode = true;
      shellLine = "";
      shellHistoryIndex = shellHistory.length;
      links = [];
      terminal.options.cursorBlink = true;
      terminal.write("\x1b[?1003l\x1b[?1006l\x1b[?25h");
      terminal.write("\r\n\x1b[90mdev-sam exited\x1b[0m\r\n");
      terminal.write(PROMPT);
      terminal.focus();
    };

    const newShellPrompt = (): void => {
      terminal.write(`\r\n${PROMPT}`);
    };

    const submitShellCommand = (raw: string): void => {
      const command = raw.trim();
      if (command === "") {
        newShellPrompt();
        return;
      }
      shellHistory.push(command);
      shellHistoryIndex = shellHistory.length;
      if (command === "dev-sam") {
        runDevSam();
        return;
      }
      if (command === "clear") {
        terminal.write(`\x1b[2J\x1b[H${PROMPT}`);
        return;
      }
      // Everything else (help, ls, cat, cd, …) runs in the wasm backend.
      // The backend speaks \n; the terminal needs \r\n (convertEol is off).
      const output = (backend?.shellExecute(raw) ?? "").replace(/\r?\n/g, "\r\n");
      const normalized = output.replace(/(?:\r\n)+$/, "");
      terminal.write(`\r\n${normalized}`);
      newShellPrompt();
    };

    const renderShellLine = (): void => {
      terminal.write(`\r\x1b[K${PROMPT}${shellLine}`);
    };

    /** Handles one keypress at the shell prompt; returns true when consumed. */
    const handleShellKey = (event: KeyboardEvent): boolean => {
      if (event.metaKey) {
        return false;
      }
      if (event.ctrlKey && event.key === "c") {
        event.preventDefault();
        terminal.write("^C");
        shellLine = "";
        newShellPrompt();
        return true;
      }
      if (event.ctrlKey && event.key === "l") {
        event.preventDefault();
        terminal.write(`\x1b[2J\x1b[H${PROMPT}${shellLine}`);
        return true;
      }
      if (event.ctrlKey || event.altKey) {
        return false;
      }
      switch (event.key) {
        case "Enter":
          event.preventDefault();
          submitShellCommand(shellLine);
          shellLine = "";
          return true;
        case "Backspace":
          event.preventDefault();
          if (shellLine.length > 0) {
            shellLine = shellLine.slice(0, -1);
            terminal.write("\b \b");
          }
          return true;
        case "Tab": {
          event.preventDefault();
          const candidates = backend?.shellComplete(shellLine) ?? [];
          if (candidates.length === 1) {
            const candidate = candidates[0] ?? "";
            const wordStart = shellLine.lastIndexOf(" ") + 1;
            // Keep the slash so `cd proj<TAB>` becomes `cd projects/`.
            const suffix = candidate.endsWith("/") ? "" : " ";
            shellLine = `${shellLine.slice(0, wordStart)}${candidate}${suffix}`;
            renderShellLine();
          } else if (candidates.length > 1) {
            terminal.write(`\r\n${candidates.join("   ")}`);
            newShellPrompt();
            renderShellLine();
          }
          return true;
        }
        case "ArrowUp":
          event.preventDefault();
          if (shellHistoryIndex > 0) {
            shellHistoryIndex--;
            shellLine = shellHistory[shellHistoryIndex] ?? "";
            renderShellLine();
          }
          return true;
        case "ArrowDown":
          event.preventDefault();
          if (shellHistoryIndex < shellHistory.length) {
            shellHistoryIndex++;
            shellLine =
              shellHistoryIndex === shellHistory.length
                ? ""
                : (shellHistory[shellHistoryIndex] ?? "");
            renderShellLine();
          }
          return true;
        default:
          if (event.key.length === 1) {
            event.preventDefault();
            shellLine += event.key;
            terminal.write(event.key);
            return true;
          }
          return false;
      }
    };

    const drainAndPaint = (): void => {
      if (backend == null || shellMode) {
        return;
      }
      if (backend.shouldQuit()) {
        exitToShell();
        return;
      }
      for (let url = backend.pollAction(); url != null; url = backend.pollAction()) {
        window.open(url, "_blank", "noopener");
      }
      const { frame, links: renderedLinks } = backend.render();
      links = renderedLinks;
      terminal.write(shiftFrame(frame));
    };

    terminal.attachCustomKeyEventHandler((event) => {
      if (event.type !== "keydown" || backend == null) {
        return true;
      }
      if (shellMode) {
        return !handleShellKey(event);
      }
      // Leave browser and function keys to the browser itself.
      if (event.metaKey || event.ctrlKey || event.altKey) {
        // Ctrl+C copies when there is a selection; Ctrl+C/Ctrl+D quit otherwise.
        const plain = event.key === "c" || event.key === "d";
        if (!(event.ctrlKey && !event.altKey && plain)) {
          return true;
        }
        if (event.key === "c" && terminal.hasSelection()) {
          return true;
        }
      }
      const mapped = mapKeyboardEvent(event);
      if (mapped == null) {
        return true;
      }
      event.preventDefault();
      const mods =
        (event.ctrlKey ? MOD_CTRL : 0) |
        (event.shiftKey ? MOD_SHIFT : 0) |
        (event.altKey ? MOD_ALT : 0);
      backend.key(mapped.key, mapped.codepoint, mods);
      drainAndPaint();
      return false;
    });

    terminal.onData((data) => {
      if (backend == null || shellMode) {
        return;
      }
      const hover = (col: number, row: number): void => {
        const overLink = links.some(
          (link) =>
            col >= link.x && col < link.x + link.w && row >= link.y && row < link.y + link.h,
        );
        container.style.cursor = overLink ? "pointer" : "default";
      };
      // Mouse reports are 1-based screen coordinates; the app lives below
      // the prompt line, so translate rows (and ignore the prompt itself).
      const { reports, remainder } = extractMouseReports(data);
      for (const mouseReport of reports) {
        dispatchMouseReport(
          mouseReport,
          (kind, button, col, row) => {
            if (row > PROMPT_ROWS) {
              backend?.mouse(kind, button, col, row - PROMPT_ROWS);
            }
          },
          (col, row) => hover(col, row - PROMPT_ROWS),
        );
      }
      if (reports.length > 0) {
        drainAndPaint();
      }
      if (
        remainder !== "" &&
        feedFallbackInput(remainder, (key, codepoint) => backend?.key(key, codepoint, 0))
      ) {
        drainAndPaint();
      }
    });

    // Taps anywhere in the terminal (re)focus the hidden input.
    const refocus = (): void => terminal.focus();
    container.addEventListener("click", refocus);

    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
      if (backend == null || shellMode) {
        return;
      }
      backend.resize(terminal.cols, terminal.rows - PROMPT_ROWS);
      drainAndPaint();
    });
    resizeObserver.observe(container);

    const boot = async (): Promise<void> => {
      try {
        backend = await loadSamTui();
      } catch (error) {
        terminal.write(
          `\x1b[31mfailed to load sam-tui.wasm:\x1b[0m ${String(error)}\r\n` +
            "\x1b[2mthe backend is built with `pnpm build:wasm` into public/wasm/.\x1b[0m\r\n",
        );
        return;
      }
      if (disposed) {
        return;
      }
      // Boot into the shell; the app is one `dev-sam` away.
      terminal.write(
        "\x1b[90mdev-sam-sh 1.0 — developer sam's terminal\x1b[0m\r\n" +
          "\x1b[90mtype help for commands, or run dev-sam\x1b[0m\r\n\r\n",
      );
      terminal.write(PROMPT);
      terminal.focus();
    };
    void boot();

    return () => {
      disposed = true;
      container.removeEventListener("click", refocus);
      resizeObserver.disconnect();
      viewportStyle.remove();
      // oxlint-disable-next-line no-underscore-dangle -- test hook
      delete window.__samTerminal;
      terminal.dispose();
    };
  }, []);

  return (
    <div className="fixed inset-0 overflow-hidden bg-[#f7f7f7]">
      <div
        ref={containerRef}
        className="h-full w-full p-1"
        aria-label="Developer Sam's portfolio as a full-screen terminal app"
        role="application"
      />
    </div>
  );
}
