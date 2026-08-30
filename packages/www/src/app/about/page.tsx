import type { Metadata } from "next";
import TerminalPage from "../../lib/TerminalPage";

export const metadata: Metadata = {
  title: "About | Developer Sam",
};

export default function AboutPage(): React.JSX.Element {
  return <TerminalPage />;
}
