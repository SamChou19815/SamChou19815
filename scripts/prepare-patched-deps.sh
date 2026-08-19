#!/bin/sh
# Materializes patched third-party crates into .patched-deps/ so that
# [patch.crates-io] in the workspace Cargo.toml resolves. See patches/README.md
# for what the patches change and why they exist.
#
# The pristine sources are fetched from crates.io (checksum-pinned) rather
# than the local registry cache, because the active [patch.crates-io] means
# cargo itself never downloads the unpatched versions.
set -eu

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
DEPS_DIR="$REPO_ROOT/.patched-deps"
mkdir -p "$DEPS_DIR"

verify_sha256() {
  file=$1
  expected=$2
  if command -v sha256sum >/dev/null 2>&1; then
    echo "$expected  $file" | sha256sum -c - >/dev/null
  else
    echo "$expected  $file" | shasum -a 256 -c - >/dev/null
  fi
}

materialize() {
  name=$1
  version=$2
  patch_file=$3
  checksum=$4

  marker="$DEPS_DIR/.$name.marker"
  stamp="$checksum $(cat "$REPO_ROOT/$patch_file" | shasum -a 256 | cut -d' ' -f1)"
  if [ -d "$DEPS_DIR/$name" ] && [ -f "$marker" ] && [ "$(cat "$marker")" = "$stamp" ]; then
    return
  fi

  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  curl -fsSL -o "$tmp/pkg.crate" "https://static.crates.io/crates/$name/$name-$version.crate"
  verify_sha256 "$tmp/pkg.crate" "$checksum"
  mkdir "$tmp/src"
  tar -xzf "$tmp/pkg.crate" -C "$tmp/src" --strip-components 1

  # The published crates may ship CRLF (crossterm does); the patches are LF.
  find "$tmp/src" -type f \( -name '*.rs' -o -name 'Cargo.toml' \) \
    -exec perl -pi -e 's/\r$//' {} +

  rm -rf "$DEPS_DIR/$name"
  cp -R "$tmp/src" "$DEPS_DIR/$name"
  (cd "$DEPS_DIR/$name" && patch -s -p1 < "$REPO_ROOT/$patch_file")
  rm -rf "$tmp"
  trap - EXIT
  printf '%s' "$stamp" > "$marker"
  echo "materialized $name $version from $patch_file"
}

materialize crossterm 0.29.0 \
  patches/crossterm-0.29.0-wasm32.patch \
  d8b9f2e4c67f833b660cdb0a3523065869fb35570177239812ed4c905aeff87b

materialize iocraft 0.8.4 \
  patches/iocraft-0.8.4-wasm-host.patch \
  aba598b3bb6724ec114b885638d12105223c44d7cf19e5eb38b12baa9a28a29d
