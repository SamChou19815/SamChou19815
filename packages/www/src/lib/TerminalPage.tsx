import SamSite from "./SamSite";

/**
 * The site is one app: every route renders this same session, and it opens on
 * the view the URL names (see `App::go_to` in `crates/sam-web/src/lib.rs`).
 * Navigating inside the app rewrites the URL through the History API rather
 * than loading another document.
 */
export default function TerminalPage(): React.JSX.Element {
  return (
    <main className="fixed inset-0 bg-[#f7f7f7]">
      <SamSite />
    </main>
  );
}
