import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "In-Canada Days Counter | Developer Sam",
  description: "Track days spent in Canada",
};

export default function Layout({ children }: { children: React.ReactNode }): React.JSX.Element {
  return <>{children}</>;
}
