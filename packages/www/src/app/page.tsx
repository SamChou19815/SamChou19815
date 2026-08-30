import type { Metadata } from "next";
import TerminalPage from "../lib/TerminalPage";

export const metadata: Metadata = {
  title: "Developer Sam — Terminal",
};

export default function IndexPage(): React.JSX.Element {
  return <TerminalPage />;
}
