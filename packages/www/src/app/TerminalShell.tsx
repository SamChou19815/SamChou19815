"use client";

import dynamic from "next/dynamic";

// xterm.js touches the DOM at import time, so it must never run on the server.
const TerminalApp = dynamic(() => import("./terminal/TerminalApp"), {
  ssr: false,
  loading: () => (
    <div className="flex h-full w-full items-center justify-center bg-[#f7f7f7] text-sm text-gray-600">
      Loading terminal…
    </div>
  ),
});

export default function TerminalShell(): React.JSX.Element {
  return <TerminalApp />;
}
