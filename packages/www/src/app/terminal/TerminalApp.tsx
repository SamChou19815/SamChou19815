"use client";

import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { useEffect, useRef } from "react";
import { loadSamTui, type SamTui } from "./sam-tui";

// The page is a true terminal: xterm.js relays raw bytes to and from the
// iocraft backend, which speaks full ANSI (alternate screen, mouse capture,
// synchronized updates). No frame protocol, no key mapping.

const PROMPT = "\x1b[1;32msam@developersam\x1b[0m\x1b[90m:\x1b[0m\x1b[34m~\x1b[0m\x1b[90m$\x1b[0m ";

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
    terminal.open(container);
    fitAddon.fit();
    // xterm.css hardcodes a black viewport; the site body is #f7f7f7.
    const viewportStyle = document.createElement("style");
    viewportStyle.textContent = ".xterm .xterm-viewport { background-color: #f7f7f7 !important; }";
    document.head.appendChild(viewportStyle);
    // oxlint-disable-next-line no-underscore-dangle -- test hook
    window.__samTerminal = terminal;

    let disposed = false;
    let backend: SamTui | null = null;
    let appRunning = false;

    /** Pump everything the backend produced into xterm, plus side effects. */
    const pump = (): void => {
      if (backend == null) {
        return;
      }
      const output = backend.drain();
      if (output !== "") {
        terminal.write(output);
      }
      for (let url = backend.pollAction(); url != null; url = backend.pollAction()) {
        window.open(url, "_blank", "noopener");
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
