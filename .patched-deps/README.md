This directory holds materialized copies of patched third-party crates
(crossterm, iocraft). Its contents are generated and gitignored.

Run `scripts/prepare-patched-deps.sh` before any cargo command on a fresh
clone, or cargo will fail to resolve the [patch.crates-io] paths that
point here. CI and scripts/build-wasm.sh do this automatically.

See patches/README.md for what the patches change.
