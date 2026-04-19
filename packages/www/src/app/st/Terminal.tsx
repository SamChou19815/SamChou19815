"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  type OutputLine,
  type TerminalState,
  executeCommand,
  getCompletions,
} from "./terminal-commands";

const WELCOME = `welcome to sam terminal
type "help" for available commands, or "cat README.md" to get started.
`;

const PROMPT_PREFIX = "sam";

const URL_REGEX = /https?:\/\/[^\s]+/g;

function linkify(text: string): React.ReactNode {
  const parts: React.ReactNode[] = [];
  let lastIndex = 0;
  for (const match of text.matchAll(URL_REGEX)) {
    const url = match[0];
    const idx = match.index;
    if (idx > lastIndex) parts.push(text.slice(lastIndex, idx));
    parts.push(
      <a
        key={idx}
        href={url}
        target="_blank"
        rel="noopener noreferrer"
        className="text-[#57c7ff] underline hover:text-[#5af78e]"
        onClick={(e) => e.stopPropagation()}
      >
        {url}
      </a>,
    );
    lastIndex = idx + url.length;
  }
  if (parts.length === 0) return text;
  if (lastIndex < text.length) parts.push(text.slice(lastIndex));
  return parts;
}

function TerminalLine({ line }: { line: OutputLine }) {
  if (line.type === "input") {
    const colonIdx = line.text.indexOf(":");
    const dollarIdx = line.text.indexOf("$", colonIdx);
    if (colonIdx !== -1 && dollarIdx !== -1) {
      const user = line.text.slice(0, colonIdx);
      const path = line.text.slice(colonIdx + 1, dollarIdx);
      const cmd = line.text.slice(dollarIdx + 1);
      return (
        <div className="whitespace-pre-wrap break-all">
          <span className="text-[#5af78e]">{user}</span>
          <span className="text-white">:</span>
          <span className="text-[#57c7ff]">{path}</span>
          <span className="text-white">$</span>
          {cmd}
        </div>
      );
    }
    return <div className="whitespace-pre-wrap break-all text-white">{line.text}</div>;
  }
  if (line.type === "error") {
    return <div className="whitespace-pre-wrap break-all text-[#ff5c57]">{line.text}</div>;
  }
  return <div className="whitespace-pre-wrap break-all text-[#e4e4e4]">{linkify(line.text)}</div>;
}

export default function Terminal() {
  const [state, setState] = useState<TerminalState>({
    cwd: "/home/sam",
    history: [{ type: "output", text: WELCOME }],
    inputHistory: [],
  });
  const [input, setInput] = useState("");
  const [historyIndex, setHistoryIndex] = useState(-1);
  const inputRef = useRef<HTMLInputElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [cursorVisible, setCursorVisible] = useState(true);

  // Cursor blink
  useEffect(() => {
    const interval = setInterval(() => setCursorVisible((v) => !v), 530);
    return () => clearInterval(interval);
  }, []);

  // Auto-scroll after state updates
  const scrollToBottom = useCallback(() => {
    requestAnimationFrame(() => {
      if (scrollRef.current) {
        scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
      }
    });
  }, []);

  // Focus input on click
  const handleClick = useCallback(() => {
    inputRef.current?.focus();
  }, []);

  const handleSubmit = useCallback(() => {
    const trimmed = input.trim();
    const inputLine: OutputLine = {
      type: "input",
      text: `${PROMPT_PREFIX}:${state.cwd}$ ${input}`,
    };

    if (!trimmed) {
      setState((prev) => ({
        ...prev,
        history: [...prev.history, inputLine],
      }));
      scrollToBottom();
      setInput("");
      return;
    }

    const result = executeCommand(trimmed, state);
    const newHistory = [...state.history, inputLine];

    let shouldClear = false;
    let urlToOpen: string | null = null;

    for (const line of result.output) {
      if (line.type === "output" && line.text === "\x1BCLEAR") {
        shouldClear = true;
      } else if (line.type === "output" && line.text.startsWith("\x1BOPEN:")) {
        urlToOpen = line.text.slice(5);
      } else {
        newHistory.push(line);
      }
    }

    if (urlToOpen) {
      window.open(urlToOpen, "_blank");
    }

    setState({
      cwd: result.newCwd,
      history: shouldClear ? [] : newHistory,
      inputHistory: [...state.inputHistory, trimmed],
    });
    scrollToBottom();
    setInput("");
    setHistoryIndex(-1);
  }, [input, state, scrollToBottom]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleSubmit();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        const history = state.inputHistory;
        if (history.length === 0) return;
        const newIndex = historyIndex === -1 ? history.length - 1 : Math.max(0, historyIndex - 1);
        setHistoryIndex(newIndex);
        setInput(history[newIndex] ?? "");
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        const history = state.inputHistory;
        if (historyIndex === -1) return;
        const newIndex = historyIndex + 1;
        if (newIndex >= history.length) {
          setHistoryIndex(-1);
          setInput("");
        } else {
          setHistoryIndex(newIndex);
          setInput(history[newIndex] ?? "");
        }
      } else if (e.key === "Tab") {
        e.preventDefault();
        const completions = getCompletions(input, state.cwd);
        if (completions.length === 1) {
          const tokens = input.split(/\s+/);
          tokens[tokens.length - 1] = completions[0] ?? "";
          setInput(tokens.join(" "));
        } else if (completions.length > 1) {
          // Show completions
          const inputLine: OutputLine = {
            type: "input",
            text: `${PROMPT_PREFIX}:${state.cwd}$ ${input}`,
          };
          setState((prev) => ({
            ...prev,
            history: [...prev.history, inputLine, { type: "output", text: completions.join("  ") }],
          }));
          scrollToBottom();
        }
      } else if (e.key === "c" && e.ctrlKey) {
        e.preventDefault();
        const inputLine: OutputLine = {
          type: "input",
          text: `${PROMPT_PREFIX}:${state.cwd}$ ${input}^C`,
        };
        setState((prev) => ({
          ...prev,
          history: [...prev.history, inputLine],
        }));
        scrollToBottom();
        setInput("");
      } else if (e.key === "l" && e.ctrlKey) {
        e.preventDefault();
        setState((prev) => ({ ...prev, history: [] }));
      }
    },
    [handleSubmit, historyIndex, input, state.cwd, state.inputHistory, scrollToBottom],
  );

  const shortCwd = state.cwd === "/home/sam" ? "~" : state.cwd.replace("/home/sam", "~");

  return (
    <div
      className="flex h-screen w-screen flex-col bg-[#1a1b26] font-mono text-sm"
      onClick={handleClick}
      onKeyDown={handleKeyDown}
    >
      {/* Title bar */}
      <div className="flex h-8 shrink-0 items-center bg-[#24283b] px-4">
        <div className="flex gap-2">
          <div className="h-3 w-3 rounded-full bg-[#ff5c57]" />
          <div className="h-3 w-3 rounded-full bg-[#f3f99d]" />
          <div className="h-3 w-3 rounded-full bg-[#5af78e]" />
        </div>
        <div className="ml-4 text-xs text-[#787c99]">sam: {shortCwd}</div>
      </div>

      {/* Terminal body */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-2">
        {state.history.map((line, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: terminal output lines
          <TerminalLine key={i} line={line} />
        ))}

        {/* Current input line */}
        <div className="flex whitespace-pre-wrap break-all">
          <span className="text-[#5af78e]">{PROMPT_PREFIX}</span>
          <span className="text-white">:</span>
          <span className="text-[#57c7ff]">{shortCwd}</span>
          <span className="text-white">$ </span>
          <span className="text-[#e4e4e4]">{input}</span>
          <span
            className={`inline-block h-[1.2em] w-[0.6em] align-middle ${cursorVisible ? "bg-[#e4e4e4]" : "bg-transparent"}`}
          />
        </div>
      </div>

      {/* Hidden input */}
      <input
        ref={inputRef}
        className="absolute opacity-0 pointer-events-none"
        value={input}
        onChange={(e) => {
          setInput(e.target.value);
          setHistoryIndex(-1);
        }}
        onKeyDown={handleKeyDown}
        spellCheck={false}
        autoComplete="off"
        autoCapitalize="none"
      />
    </div>
  );
}
