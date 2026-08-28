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
// synchronized updates). No frame protocol, no key mapping.
//
// Card artwork is the one thing layered on top. The backend already draws it
// in-band as truecolor half-blocks — that is what the native `dev-sam` binary
// shows — and reports where each one landed. Here we cover those cells with the
// real asset, so the web gets full resolution and the half-blocks become the
// fallback if an image fails to load.

const PROMPT = "\x1b[1;32msam@developersam\x1b[0m\x1b[90m:\x1b[0m\x1b[34m~\x1b[0m\x1b[90m$\x1b[0m ";

declare global {
  interface Window {
    /** Test hook: the xterm instance driving this page, for cell-exact automation. */
    __samTerminal?: Terminal;
  }
}

/**
 * Opens a link the terminal printed. Only http(s): the page renders whatever
 * bytes reach it, and a `javascript:` URL would run in this document.
 */
function openLink(url: string): void {
  if (/^https?:\/\//i.test(url)) {
    window.open(url, "_blank", "noopener");
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
      // The shell writes its URLs as OSC 8 hyperlinks — the `@` tags of
      // `cat about.txt`, the contact list, resume.pdf. xterm draws those as
      // plain text unless it is told what activating one means.
      linkHandler: { activate: (_event, uri) => openLink(uri) },
      scrollback: 1000,
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
    let appRunning = false;

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
      const screen = container.querySelector(".xterm-screen");
      if (screen == null) {
        return;
      }
      const screenRect = screen.getBoundingClientRect();
      const containerRect = container.getBoundingClientRect();
      const cellWidth = screenRect.width / terminal.cols;
      const cellHeight = screenRect.height / terminal.rows;
      const offsetLeft = screenRect.left - containerRect.left;
      const offsetTop = screenRect.top - containerRect.top;

      const used = new Map<string, number>();
      for (const region of regions) {
        // "x y cols rows visibleX visibleY visibleCols visibleRows url" in
        // canvas cells; the app owns the alternate screen, so cell (0, 0) is
        // the top left of the viewport.
        const [x, y, cols, rows, vx, vy, vcols, vrows, ...rest] = region.split(" ");
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
        // clipped to the part the pane actually painted — otherwise a card
        // half off the bottom would spill its artwork over the status bar. The
        // full rectangle can start above the viewport, at a negative y, when a
        // post is scrolled part-way through one of its images.
        const inset = [
          (Number(vy) - Number(y)) * cellHeight,
          (Number(x) + Number(cols) - Number(vx) - Number(vcols)) * cellWidth,
          (Number(y) + Number(rows) - Number(vy) - Number(vrows)) * cellHeight,
          (Number(vx) - Number(x)) * cellWidth,
        ];
        Object.assign(element.style, {
          left: `${offsetLeft + Number(x) * cellWidth}px`,
          top: `${offsetTop + Number(y) * cellHeight}px`,
          width: `${Number(cols) * cellWidth}px`,
          height: `${Number(rows) * cellHeight}px`,
          clipPath: inset.every((side) => side === 0)
            ? "none"
            : `inset(${inset.map((side) => `${side}px`).join(" ")})`,
        });
      }
      // Retire whatever this frame did not place — a card that scrolled off,
      // or a duplicate that is no longer doubled up.
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

    const clearImages = (): void => {
      imageLayer.replaceChildren();
      pool.clear();
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
      if (appRunning) {
        syncImages();
      }
      for (let url = backend.pollAction(); url != null; url = backend.pollAction()) {
        openLink(url);
      }
      if (appRunning && backend.shouldQuit()) {
        exitToShell();
      }
    };

    const runDevSam = (): void => {
      if (backend == null) {
        return;
      }
      appRunning = true;
      backend.start(terminal.cols, terminal.rows);
      pump();
      terminal.focus();
    };

    const exitToShell = (): void => {
      appRunning = false;
      pump();
      clearImages();
      terminal.write("\x1b[90mdev-sam exited — type dev-sam to run it again, or help\x1b[0m\r\n");
      terminal.write(PROMPT);
      terminal.focus();
    };

    // --- The shell (dev-sam-sh) ---------------------------------------------
    let shellLine = "";
    const shellHistory: string[] = [];
    let shellHistoryIndex = 0;

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
        terminal.write("\r\n");
        runDevSam();
        return;
      }
      if (command === "clear") {
        terminal.write(`\x1b[2J\x1b[H${PROMPT}`);
        return;
      }
      const output = backend?.shellExecute(raw) ?? "";
      const normalized = output.replace(/\r?\n/g, "\r\n").replace(/(?:\r\n)+$/, "");
      terminal.write(`\r\n${normalized}`);
      newShellPrompt();
    };

    const renderShellLine = (): void => {
      terminal.write(`\r\x1b[K${PROMPT}${shellLine}`);
    };

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

    // --- Input routing --------------------------------------------------------
    // While the app runs, every byte xterm produces is forwarded verbatim
    // (keys AND mouse reports — the backend parses real ANSI). At the shell
    // prompt, the line editor handles keys directly.
    terminal.attachCustomKeyEventHandler((event) => {
      if (event.type !== "keydown" || backend == null || appRunning) {
        return true;
      }
      return !handleShellKey(event);
    });

    const encoder = new TextEncoder();
    terminal.onData((data) => {
      if (backend == null || !appRunning) {
        return;
      }
      backend.input(encoder.encode(data));
      pump();
    });

    const refocus = (): void => terminal.focus();
    container.addEventListener("click", refocus);

    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
      if (backend != null && appRunning) {
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
      // Boot into the shell; the app is one `dev-sam` away.
      terminal.write(
        "\x1b[90mdev-sam-sh 1.0 — developer sam's terminal\x1b[0m\r\n" +
          "\x1b[90mtype help for commands, or run dev-sam\x1b[0m\r\n\r\n",
      );
      // Pre-type the headline command so a visitor only has to press Enter.
      // Seeded into the line editor's buffer, not just painted, so backspace
      // and the rest of the editing keys see it as text they typed.
      shellLine = "dev-sam";
      renderShellLine();
      terminal.focus();
    };
    void boot();

    return () => {
      disposed = true;
      container.removeEventListener("click", refocus);
      resizeObserver.disconnect();
      viewportStyle.remove();
      imageLayer.remove();
      // oxlint-disable-next-line no-underscore-dangle -- test hook
      delete window.__samTerminal;
      terminal.dispose();
    };
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
