/**
 * Loader for the Rust/WASM TUI backend (`crates/sam-tui`).
 *
 * The backend owns all rendering: JS feeds it keyboard and mouse events,
 * then paints the ANSI frame it returns into xterm.js. Frames also carry a
 * table of clickable link regions (terminal cell coordinates).
 */

interface SamTuiWasmExports {
  memory: WebAssembly.Memory;
  sam_write_input_byte(index: number, byte: number): void;
  sam_shell_execute(length: number): number;
  sam_shell_complete(length: number): number;
  sam_shell_output_ptr(): number;
  sam_resize(cols: number, rows: number): void;
  sam_key(key: number, codepoint: number, mods: number): void;
  sam_mouse(kind: number, button: number, col: number, row: number): void;
  sam_render(): number;
  sam_poll_action(): number;
  sam_should_quit(): number;
  sam_reset(): void;
  sam_output_ptr(): number;
}

export interface LinkRegion {
  x: number;
  y: number;
  w: number;
  h: number;
  url: string;
}

export interface SamTui {
  resize(cols: number, rows: number): void;
  key(key: number, codepoint: number, mods: number): void;
  mouse(kind: number, button: number, col: number, row: number): void;
  render(): { frame: string; links: LinkRegion[] };
  pollAction(): string | null;
  shouldQuit(): boolean;
  reset(): void;
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

  const readOutput = (length: number): Uint8Array =>
    new Uint8Array(wasm.memory.buffer, wasm.sam_output_ptr(), length);

  const render = (): { frame: string; links: LinkRegion[] } => {
    const bytes = readOutput(wasm.sam_render());
    const separator = bytes.indexOf(0);
    const frame = decoder.decode(bytes.subarray(0, separator));
    const links: LinkRegion[] = [];
    const view = new DataView(bytes.buffer, bytes.byteOffset + separator + 1);
    const count = view.getUint32(0, true);
    let offset = 4;
    for (let index = 0; index < count; index++) {
      const x = view.getUint16(offset, true);
      const y = view.getUint16(offset + 2, true);
      const w = view.getUint16(offset + 4, true);
      const h = view.getUint16(offset + 6, true);
      const urlLength = view.getUint32(offset + 8, true);
      offset += 12;
      const url = decoder.decode(
        bytes.subarray(separator + 1 + offset, separator + 1 + offset + urlLength),
      );
      offset += urlLength;
      links.push({ x, y, w, h, url });
    }
    return { frame, links };
  };

  const pollAction = (): string | null => {
    const length = wasm.sam_poll_action();
    return length === 0 ? null : decoder.decode(readOutput(length));
  };

  // The shell shares the app's input buffer: bytes in, output length out.
  const sendLine = (line: string): number => {
    const bytes = encoder.encode(line.slice(0, 1024));
    for (const [index, byte] of bytes.entries()) {
      wasm.sam_write_input_byte(index, byte);
    }
    return bytes.length;
  };

  const shellOutput = (length: number): string =>
    length === 0
      ? ""
      : decoder.decode(new Uint8Array(wasm.memory.buffer, wasm.sam_shell_output_ptr(), length));

  return {
    resize: (cols, rows) => wasm.sam_resize(cols, rows),
    key: (key, codepoint, mods) => wasm.sam_key(key, codepoint, mods),
    mouse: (kind, button, col, row) => wasm.sam_mouse(kind, button, col, row),
    render,
    pollAction,
    shouldQuit: () => wasm.sam_should_quit() === 1,
    reset: () => wasm.sam_reset(),
    shellExecute: (line) => shellOutput(wasm.sam_shell_execute(sendLine(line))),
    shellComplete: (line) => {
      const candidates = shellOutput(wasm.sam_shell_complete(sendLine(line)));
      return candidates === "" ? [] : candidates.split("\n");
    },
  };
}
