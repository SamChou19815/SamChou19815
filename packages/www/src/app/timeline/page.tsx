import type { Metadata } from "next";
import TerminalPage from "../../lib/TerminalPage";

export const metadata: Metadata = {
  title: "Timeline | Developer Sam",
};

export default function TimelinePage(): React.JSX.Element {
  return <TerminalPage />;
}
