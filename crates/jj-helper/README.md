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
| `t`       | rebase on main                                                         | `n` / `N` / `I` | new change here / insert       |
| `s` / `S` | squash into parent / into…                                             | `d`             | describe (`Ctrl-E`: `$EDITOR`) |
| `x`       | split…                                                                 | `b`             | set branch here…               |
| `a` / `A` | abandon… / with descendants                                            | `y` / `v`       | duplicate / revert onto…       |
| `P`       | Create Request (PR) / Update Request (PR)                              | `M`             | Ship Request (PR) (+ below)    |
| `i`       | interdiff since push / review                                          | `c`             | clean up shipped request (PR)  |
| `f` / `p` | fetch / push (all branches)                                            | `u` / `U`       | undo / redo                    |
| `Space`   | diff                                                                   | `?`             | help                           |

## GitHub

Uses `GITHUB_TOKEN` / `GH_TOKEN`, falling back to `gh auth token`; the repo comes from the
`origin` remote (override with `--remote` or `JJ_HELPER_GITHUB_REPO=owner/name`).

The PR commands run exactly what VisualJJ's `jj branching submit` / `push` / `ship-pr` run
(reverse engineered from the extension's bundled `jj`), through your own `jj`:

- **Create request (`P` on a change without a PR)** — `jj git push --change <change>`
  (bookmark `push-<change id>`, jj's own `templates.git_push_bookmark`), then one PR against the
  trunk branch via GraphQL `createPullRequest` (title = first description line, empty body), then
  `jj new` if the change was the working copy. `jj-helper submit` takes `--title`,
  `--new-bookmark`, `--draft` like VisualJJ's dialog; `--stack` submits every change from trunk,
  bottom up.
- **Update request (`P` on a change with a PR)** — a live read of the PRs the push can affect,
  refusing pushes GitHub would act on (a head becoming contained in its base), then
  `jj git push --bookmark <branch>`; dependents whose base moved sideways are parked on the
  nearest base below that still contains them.
- **Stacking** — follows the repo's VisualJJ settings, off by default like VisualJJ's:
  `visualjj.stacking-github-stacks` retargets each new PR onto the nearest ancestor with an open
  PR (`PATCH /pulls/{n}`) and rebuilds the [native stack](https://docs.github.com/en/rest/pulls/stacks)
  (`/stacks/{n}/unstack`, then `POST /stacks`); `visualjj.stacking-update-description` writes the
  `<!-- VISUALJJ_STACK_V1 -->` stack list into every PR of the stack. `--native-stack` /
  `--stack-info` (and `--no-…`) override per run.
- **Ship (`M`)** — `PUT /pulls/{n}/merge-async` with your GitHub default merge method
  (`--merge-method` overrides), polled until done, then fetches.
- **Interdiff (`i`)** — what changed since the last push, since a reviewer's last review, or since
  the previous local version — re-review only new changes.
- **Auto-fetch** every 5 minutes (`--auto-fetch N`, 0 disables); with `--auto-rebase` (toggle: `T`)
  your stacks are rebased onto the new trunk, undone automatically if that would add conflicts.

Non-interactive: `jj-helper log`, `jj-helper submit [REV] [--stack] [--dry-run]`,
`jj-helper ship [REV]`, `jj-helper sync [--rebase]`.
