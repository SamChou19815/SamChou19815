import TerminalShell from "../app/TerminalShell";

/**
 * The site is one app: every route renders this same shell, and the terminal
 * inside it opens on the view the URL names (see `App::go_to` in
 * `crates/sam-tui/src/lib.rs`). Navigating inside the terminal rewrites the URL
 * through the History API rather than loading another document.
 */
export default function TerminalPage(): React.JSX.Element {
  return (
    <main className="fixed inset-0 bg-[#f7f7f7]">
      <TerminalShell />
    </main>
  );
}
