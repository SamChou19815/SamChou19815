import { createFileRoute } from "@tanstack/react-router";
import { SITE_ICON_LINKS } from "../lib/appIcons";
import TerminalPage from "../lib/TerminalPage";

/**
 * Every terminal URL shares this one layout, so the wasm app stays mounted
 * while it moves between views: its router pushes history entries the
 * TanStack router also follows, and only the child match (which carries just
 * the page's head) changes. See `TerminalPage`.
 */
export const Route = createFileRoute("/_terminal")({
  head: () => ({ links: SITE_ICON_LINKS }),
  component: TerminalPage,
});
