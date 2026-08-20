/**
 * Loader for the Rust/iocraft backend (`crates/sam-tui`).
 *
 * The backend is a real terminal application: it writes ANSI (alternate
 * screen, mouse capture, synchronized updates) to an in-memory stream and
 * parses raw input bytes. JS relays bytes between it and xterm.js — no frame
 * protocol, no key mapping.
 */

interface SamTuiWasmExports {
  memory: WebAssembly.Memory;
  sam_start(cols: number, rows: number): void;
  sam_input_byte(index: number, byte: number): void;
  sam_input(length: number): void;
  sam_resize(cols: number, rows: number): void;
  sam_output_take(): number;
  sam_out_ptr(): number;
  sam_poll_action(): number;
  sam_should_quit(): number;
  sam_write_input_byte(index: number, byte: number): void;
  sam_shell_execute(length: number): number;
  sam_shell_complete(length: number): number;
  sam_shell_output_ptr(): number;
}

export interface SamTui {
  start(cols: number, rows: number): void;
  input(bytes: Uint8Array): void;
  resize(cols: number, rows: number): void;
  drain(): string;
  pollAction(): string | null;
  shouldQuit(): boolean;
  shellExecute(line: string): string;
  shellComplete(line: string): string[];
}

const decoder = new TextDecoder();
const encoder = new TextEncoder();

export async function loadSamTui(): Promise<SamTui> {
  const response = await fetch("/wasm/sam-tui.wasm");
  if (!response.ok) {
    throw new Error(`Failed to fetch sam-tui.wasm: ${response.status}`);
  }
  const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), {});
  const wasm = instance.exports as unknown as SamTuiWasmExports;

  const sendBytes = (write: (index: number, byte: number) => void, bytes: Uint8Array): number => {
    for (const [index, byte] of bytes.entries()) {
      write(index, byte);
    }
    return bytes.length;
  };

  const readApp = (length: number): string =>
    length === 0
      ? ""
      : decoder.decode(new Uint8Array(wasm.memory.buffer, wasm.sam_out_ptr(), length));

  const readShell = (length: number): string =>
    length === 0
      ? ""
      : decoder.decode(new Uint8Array(wasm.memory.buffer, wasm.sam_shell_output_ptr(), length));

  return {
    start: (cols, rows) => wasm.sam_start(cols, rows),
    input: (bytes) => {
      wasm.sam_input(sendBytes(wasm.sam_input_byte, bytes));
    },
    resize: (cols, rows) => wasm.sam_resize(cols, rows),
    drain: () => readApp(wasm.sam_output_take()),
    pollAction: () => {
      const length = wasm.sam_poll_action();
      return length === 0 ? null : readApp(length);
    },
    shouldQuit: () => wasm.sam_should_quit() === 1,
    shellExecute: (line) =>
      readShell(wasm.sam_shell_execute(sendBytes(wasm.sam_write_input_byte, encoder.encode(line)))),
    shellComplete: (line) => {
      const sent = sendBytes(wasm.sam_write_input_byte, encoder.encode(line));
      const candidates = readShell(wasm.sam_shell_complete(sent));
      return candidates === "" ? [] : candidates.split("\n");
    },
  };
}
