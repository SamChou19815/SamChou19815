import type { Metadata } from "next";
import TerminalPage from "../../lib/TerminalPage";

export const metadata: Metadata = {
  title: "About | Developer Sam",
  description: "Who Developer Sam is, as the About tab of a full-screen terminal app.",
  openGraph: {
    images: "https://developersam.com/sam-by-megan-3-square.webp",
    type: "profile",
  },
};

export default function AboutPage(): React.JSX.Element {
  return <TerminalPage />;
}
