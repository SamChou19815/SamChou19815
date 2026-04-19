import type { Metadata } from "next";
import Terminal from "./Terminal";

export const metadata: Metadata = {
  title: "Sam Terminal",
  description: "An interactive terminal interface for Developer Sam's website.",
};

export default function TerminalPage() {
  return <Terminal />;
}
