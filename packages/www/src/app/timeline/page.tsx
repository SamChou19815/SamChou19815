import type { Metadata } from "next";
import TerminalPage from "../../lib/TerminalPage";

export const metadata: Metadata = {
  title: "Timeline | Developer Sam",
  description: "What Developer Sam has built and shipped, year by year.",
  openGraph: {
    images: "https://developersam.com/sam-by-megan-3-square.webp",
    type: "profile",
  },
};

export default function TimelinePage(): React.JSX.Element {
  return <TerminalPage />;
}
