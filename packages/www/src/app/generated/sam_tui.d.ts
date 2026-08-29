/* tslint:disable */
/* eslint-disable */

/**
 * Takes the ANSI bytes written since the last drain.
 */
export function drain(): string;

/**
 * Whether this app has a view at `path`. The host asks before following a
 * link or a back button itself rather than leaving it to the browser.
 */
export function hasView(path: string): boolean;

/**
 * Where the current frame drew its artwork, one row per image, as
 * `"x y cols rows visibleX visibleY visibleCols visibleRows url"` in canvas
 * cells. The app owns the alternate screen, so cell (0, 0) is the top left of
 * the viewport and the host can place an `<img>` straight onto it.
 *
 * The second rectangle is the part that survived the pane's clipping: a card
 * scrolled half off the bottom paints only some of its artwork, and the
 * overlay has to crop to match. Images with nothing on screen are omitted.
 * The first rectangle is the whole picture, so its origin goes negative for
 * one scrolled part-way off the top of a pane.
 *
 * Space-separated rather than JSON: no asset path contains a space, and a
 * serializer would cost the wasm binary more than the artwork itself does.
 */
export function imageRegions(): string[];

export function input(bytes: Uint8Array): void;

/**
 * Shows the view `path` names: the URL the page was entered at, a link
 * followed inside a post, or wherever the back button just went.
 *
 * Before the app boots this only records the view, which `start` then opens
 * on. Afterwards the running render loop has to be woken to pick it up, and
 * iocraft forwards only key, mouse and resize events — so the wake is a
 * resize to the size the app already has, the same no-op `start` pushes.
 */
export function navigate(path: string): void;

/**
 * Drains the next pending URL action, if any.
 */
export function pollAction(): string | undefined;

export function resize(cols: number, rows: number): void;

/**
 * The view the last frame drew, as a site path, for the host to put in the
 * URL bar.
 */
export function route(): string;

export function shellComplete(line: string): string[];

export function shellExecute(line: string): string;

export function shouldQuit(): boolean;

export function start(cols: number, rows: number): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly drain: (a: number) => void;
    readonly hasView: (a: number, b: number) => number;
    readonly imageRegions: (a: number) => void;
    readonly input: (a: number, b: number) => void;
    readonly navigate: (a: number, b: number) => void;
    readonly pollAction: (a: number) => void;
    readonly resize: (a: number, b: number) => void;
    readonly route: (a: number) => void;
    readonly shellComplete: (a: number, b: number, c: number) => void;
    readonly shellExecute: (a: number, b: number, c: number) => void;
    readonly shouldQuit: () => number;
    readonly start: (a: number, b: number) => void;
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
