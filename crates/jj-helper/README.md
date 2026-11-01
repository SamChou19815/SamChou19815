# jj-helper

A terminal take on [VisualJJ](https://www.visualjj.com/): shape Jujutsu commits visually and drive
GitHub stacked PRs, including GitHub's native PR stacks.

Run `./jj-helper` from the repo root (a `sam-run` launcher, auto-deployed like `sam-cli`), or
`cargo run -p jj-helper` inside any jj repo.

## Workflow

Select a commit (`j`/`k`, `@`), press a command key (or `Enter` for the menu). Anything that changes
the graph opens a **Before / After** view; nothing runs until you press `Enter`. Commands with a
destination — rebase, squash into, revert onto, move bookmark — let you move a target cursor while
the After pane updates live, so you never type revsets.

| Key       | Command                                                                | Key             | Command                        |
| --------- | ---------------------------------------------------------------------- | --------------- | ------------------------------ |
| `r`       | rebase… (`Tab`: onto / insert after / before, `s`: `-s` / `-r` / `-b`) | `e`             | edit                           |
| `t`       | rebase stack onto trunk                                                | `n` / `N` / `I` | new / insert before / after    |
| `s` / `S` | squash into parent / into…                                             | `d`             | describe (`Ctrl-E`: `$EDITOR`) |
| `x`       | split (interactive)                                                    | `b`             | set / move bookmark            |
| `a` / `A` | abandon / with descendants                                             | `y` / `v`       | duplicate / revert onto…       |
| `P`       | create / update stacked PRs                                            | `M`             | ship PR (+ everything below)   |
| `i`       | interdiff since push / review                                          | `c`             | clean up shipped commits       |
| `f` / `p` | fetch / push stack                                                     | `u` / `U`       | undo / redo                    |
| `Space`   | diff                                                                   | `?`             | help                           |

## GitHub

Uses `GITHUB_TOKEN` / `GH_TOKEN`, falling back to `gh auth token`; the repo comes from the
`origin` remote (override with `--remote` or `JJ_HELPER_GITHUB_REPO=owner/name`).

- **Submit (`P`)** — one PR per commit from trunk to the selection, each based on the one below
  (`push-<change id>` bookmarks, like `jj git push -c`). PRs are linked as a
  [native GitHub stack](https://docs.github.com/en/rest/pulls/stacks) (stacks that no longer match
  are dissolved, retargeted and rebuilt) and get a stack list in their description.
  `--no-native-stack` / `--no-stack-footer` opt out.
- **Ship (`M`)** — merges through the asynchronous merge API (required for stacked PRs; merges all
  PRs below too), polls until done, then fetches.
- **Interdiff (`i`)** — what changed since the last push, since a reviewer's last review, or since
  the previous local version — re-review only new changes.
- **Auto-fetch** every 5 minutes (`--auto-fetch N`, 0 disables); with `--auto-rebase` (toggle: `T`)
  your stacks are rebased onto the new trunk, undone automatically if that would add conflicts.

Non-interactive: `jj-helper log`, `jj-helper submit [REV] [--dry-run]`, `jj-helper ship [REV]`,
`jj-helper sync [--rebase]`.
