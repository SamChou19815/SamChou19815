# sam-run

A sam-run script is a small text file that starts with a `sam-run` shebang followed by a JSON
manifest; running the script downloads the right binary from a GitHub release, caches it, and
`exec`s it with your arguments.

The file is **not pinned to a specific version**: sam-run resolves the newest release (or a named
tag) dynamically and caches the result — for 1 day by default.

## Installation

Download the binary for your platform from the rolling `sam-run-latest` release — the asset is
one of `sam-run-linux-x86_64`, `sam-run-linux-aarch64`, `sam-run-macos-x86_64`,
`sam-run-macos-aarch64`:

```sh
curl -fsSL "https://github.com/SamChou19815/SamChou19815/releases/download/sam-run-latest/sam-run-$(uname -s | tr '[:upper:]' '[:lower:]' | sed s/darwin/macos/)-$(uname -m | sed s/arm64/aarch64/)" \
  -o ~/.local/bin/sam-run
chmod +x ~/.local/bin/sam-run
```

(Any directory on your `PATH` works; in GitHub Actions use the `setup-sam-run` composite action
instead.)

## Updating

```sh
sam-run self-update           # replace this executable with the latest release
sam-run self-update --check   # only report whether an update is available
```

`sam-run --version` prints the commit the binary was built from; `self-update` is a no-op when
that commit matches the current release. Local dev builds have no embedded commit and always
update. The command replaces the file the running executable resolves to (symlinks are followed),
so an install in a root-owned directory like `/usr/local/bin` needs
`sudo -E sam-run self-update`.

## Script format

```bash
#!/usr/bin/env sam-run
{
  "repo": "BurntSushi/ripgrep",
  "platforms": {
    "linux-x86_64": {
      "asset": "ripgrep-{version}-x86_64-unknown-linux-musl.tar.gz",
      "path": "ripgrep-{version}/rg"
    },
    "macos-aarch64": {
      "asset": "ripgrep-{version}-aarch64-apple-darwin.tar.gz",
      "path": "ripgrep-{version}/rg"
    }
  }
}
```

Save it as `rg`, `chmod +x rg`, and `./rg --version` just works.

Manifest fields:

- `repo` (required): GitHub repository in `owner/name` form.
- `tag` (optional): release tag to use. Defaults to the repository's latest release.
- `ttl-hours` (optional, default `24`): how long a resolved release stays fresh before sam-run
  re-checks GitHub. `0` re-checks on every run. If the re-check fails (offline, rate limited),
  sam-run warns and falls back to the cached binary.
- `platforms` (required): map from platform key — one of `linux-x86_64`, `linux-aarch64`,
  `macos-x86_64`, `macos-aarch64` — to:
  - `asset` (required): the release asset name. `{tag}` expands to the release tag and
    `{version}` to the tag with one leading `v` stripped.
  - `path` (optional): path of the executable inside an archive asset (placeholders expand
    here too). May be omitted when the archive contains exactly one file.
  - `format` (optional): `binary`, `tar.gz`, `zip`, or `gz`. Inferred from the asset
    extension when omitted.

Authenticated requests (higher rate limits, private repos) use `GITHUB_TOKEN` or `GH_TOKEN`
when set.

## Cache

Downloads live under the per-user cache directory (`~/Library/Caches/sam-run` on macOS,
`~/.cache/sam-run` on Linux), keyed by repo, tag, and platform.

```sh
sam-run invalidate owner/name   # forget one repository
sam-run invalidate              # clear the whole cache
```

## Releases

Every commit to `main` that touches this crate republishes the four platform binaries to the
rolling `sam-run-latest` release via `.github/workflows/sam-run-release.yml`.
