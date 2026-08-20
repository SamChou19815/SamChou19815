#!/bin/sh
# Builds the sam-tui wasm artifact for the website into
# packages/www/public/wasm/. Works whether or not a Rust toolchain is
# preinstalled: Cloudflare Pages build images ship without one, so a minimal
# stable toolchain is bootstrapped via rustup when cargo is missing.
set -eu

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found; installing a minimal Rust toolchain"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable
  . "$HOME/.cargo/env"
fi

# A no-op when the target is already installed; tolerated when the toolchain
# is not rustup-managed (cargo will fail with its own clear error if the
# wasm32 standard library is genuinely unavailable).
rustup target add wasm32-unknown-unknown 2>/dev/null || true

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
cd "$REPO_ROOT"
# Materialize the patched crossterm/iocraft sources [patch.crates-io] needs.
./scripts/prepare-patched-deps.sh
cargo build --release -p sam-tui --target wasm32-unknown-unknown
mkdir -p packages/www/public/wasm
cp target/wasm32-unknown-unknown/release/sam_tui.wasm packages/www/public/wasm/sam-tui.wasm
