# Dependency patches

The terminal TUI (`crates/sam-tui`) is built on iocraft, which needs two
small patches to run as wasm32-unknown-unknown inside the xterm.js bridge
on the website. The patched sources are _not_ checked in; they are
materialized into `.patched-deps/` (gitignored) by
`scripts/prepare-patched-deps.sh`, which downloads the pristine crates
from crates.io (checksum-pinned), normalizes them to LF, and applies
these patches. `[patch.crates-io]` in the workspace `Cargo.toml` points
at the materialized copies.

Run the script before any cargo command on a fresh clone. CI
(`rust.yml`) and the wasm build (`scripts/build-wasm.sh`, also used by
Cloudflare) invoke it automatically.

## crossterm 0.29.0 — `crossterm-0.29.0-wasm32.patch`

Upstream does not compile for wasm32-unknown-unknown (its event,
terminal, and cursor sys layers only ship unix and windows backends).
The patch adds a wasm32 backend: a byte-queue event source fed by the
host over the FFI (`push_input`/`push_event`/`set_size`), a cooperative
thread-free `EventStream`, a clock-free `PollTimeout` (`Instant::now`
panics on this target), no-op raw-mode/cursor stubs, and `is_tty() ==
true` on wasm. All changes are additive and cfg-gated on
`target_family = "wasm"`. Upstream feature request: crossterm-rs/crossterm#654.

## iocraft 0.8.4 — `iocraft-0.8.4-wasm-host.patch`

`StdTerminal` treats a non-tty stdin as "no input" and returns a
permanently pending event stream, so fullscreen components never receive
input when embedded. On wasm the host (the xterm.js bridge bridging raw
bytes) _is_ the terminal, so `input_is_terminal` is forced to `true`
under `target_family = "wasm"`. 4 lines.

When either patch lands upstream (PRs welcome), delete the
corresponding patch file, the `materialize` call in the prepare script,
and the `[patch.crates-io]` entry.
