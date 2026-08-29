/* tslint:disable */
/* eslint-disable */

/**
 * Takes the ANSI bytes written since the last drain.
 */
export function drain(): string;

/**
 * Where the current frame drew its artwork, one row per image, as
 * `"x y cols rows top right bottom left url"` in canvas cells. The app owns
 * the alternate screen, so cell (0, 0) is the top left of the viewport and the
 * host can place an `<img>` straight onto it.
 *
 * The four sides are what the pane's clipping took off: a card scrolled half
 * off the bottom paints only some of its artwork, and the overlay has to crop
 * to match. Images with nothing on screen are omitted. The rectangle is the
 * whole picture, so its origin goes negative for one scrolled part-way off the
 * top of a pane.
 *
 * Space-separated rather than JSON: no asset path contains a space, and a
 * serializer would cost the wasm binary more than the artwork itself does.
 */
export function imageRegions(): string[];

export function input(bytes: Uint8Array): void;

export function navigate(path: string): void;

/**
 * Activates a link the terminal printed — an OSC 8 hyperlink, or a bare URL
 * the host's link addon spotted. One naming a view of this app is followed
 * here; everything else comes back as [`crate::HostEvent::Open`].
 */
export function openLink(url: string): void;

/**
 * Takes the next thing only the browser can do, as
 * `open <url>` or `route push|replace <path>\t<title>`.
 */
export function pollEvent(): string | undefined;

export function resize(cols: number, rows: number): void;

export function start(cols: number, rows: number, path: string, touch: boolean): void;

/**
 * Rows one wheel notch scrolls. A host with no wheel — a phone metering a
 * drag — has to know the distance the app moves for one to keep the content
 * under the finger.
 */
export function wheelRows(): number;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly drain: (a: number) => void;
    readonly imageRegions: (a: number) => void;
    readonly input: (a: number, b: number) => void;
    readonly navigate: (a: number, b: number) => void;
    readonly openLink: (a: number, b: number) => void;
    readonly pollEvent: (a: number) => void;
    readonly resize: (a: number, b: number) => void;
    readonly start: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly wheelRows: () => number;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
