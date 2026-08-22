#!/bin/sh
# Builds the sam-tui wasm bundle for the website — the binary plus the
# wasm-bindgen JS glue and .d.ts — into packages/www/src/app/generated/,
# where Next bundles it as a static asset. Works whether or not a Rust toolchain
# is preinstalled: CI sets one up before calling this, but a minimal stable
# toolchain and wasm-pack are bootstrapped when missing.
set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found; installing a minimal Rust toolchain"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable
  . "$HOME/.cargo/env"
fi

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "wasm-pack not found; installing the prebuilt binary"
  curl --proto '=https' --tlsv1.2 -sSf \
    https://rustwasm.github.io/wasm-pack/installer/init.sh | sh
fi

# A no-op when the target is already installed; tolerated when the toolchain
# is not rustup-managed (cargo will fail with its own clear error if the
# wasm32 standard library is genuinely unavailable).
rustup target add wasm32-unknown-unknown 2>/dev/null || true

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$REPO_ROOT"
# Materialize the patched crossterm/iocraft sources [patch.crates-io] needs.
./scripts/prepare-patched-deps.sh

OUT_DIR="$REPO_ROOT/packages/www/src/app/generated"
rm -rf "$OUT_DIR"
wasm-pack build crates/sam-tui --release --target web --no-pack --out-dir "$OUT_DIR"

rm -f "$OUT_DIR/.gitignore" "$OUT_DIR/sam_tui_bg.wasm.d.ts"
