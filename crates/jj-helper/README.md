# jj-helper

A terminal take on [VisualJJ](https://www.visualjj.com/): shape Jujutsu commits visually and drive
GitHub stacked PRs, including GitHub's native PR stacks. Run `./jj-helper` from the repo root (a
`sam-run` launcher, auto-deployed like `sam-cli`), or `cargo run -p jj-helper` inside any jj repo;
press `?` in the TUI for key bindings. Uses `GITHUB_TOKEN` / `GH_TOKEN`, falling back to
`gh auth token`; non-interactive: `jj-helper log | submit | ship | sync`.
