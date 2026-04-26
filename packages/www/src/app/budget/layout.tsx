import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Budget | Developer Sam",
  robots: { index: false, follow: false },
};

export default function BudgetLayout({ children }: { children: React.ReactNode }) {
  return <>{children}</>;
}
