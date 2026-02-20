import "../lib/common.css";

import type { Metadata } from "next";

export const metadata: Metadata = {
  icons: ["/logo.png", { rel: "shortcut icon", url: "/favicon.ico" }],
  authors: { name: "Developer Sam" },
  keywords: [
    "Sam",
    "Sam Zhou",
    "Developer Sam",
    "developer",
    "web apps",
    "open source",
    "programming language",
    "rust",
  ],
  metadataBase: new URL("https://developersam.com"),
  manifest: "/manifest.json",
  alternates: {
    canonical: "https://developersam.com/",
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
