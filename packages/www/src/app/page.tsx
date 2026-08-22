import type { Metadata } from "next";
import TerminalShell from "./TerminalShell";

export const metadata: Metadata = {
  title: "Developer Sam — Terminal",
  description:
    "Explore Developer Sam's portfolio as a full-screen terminal app: powered by xterm.js and a Rust backend compiled to WebAssembly.",
  openGraph: {
    images: "https://developersam.com/sam-by-megan-3-square.webp",
    type: "website",
  },
};

export default function IndexPage(): React.JSX.Element {
  return (
    <main className="fixed inset-0 bg-[#f7f7f7]">
      <TerminalShell />
    </main>
  );
}
