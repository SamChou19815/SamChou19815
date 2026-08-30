import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "In-Canada Days Counter | Developer Sam",
  robots: { index: false, follow: false },
};

export default function Layout({ children }: { children: React.ReactNode }): React.JSX.Element {
  return <>{children}</>;
}
