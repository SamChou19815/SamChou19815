"use client";

import { Suspense } from "react";
import NavBar from "../../lib/NavBar";
import AuthGate from "./AuthGate";
import BudgetApp from "./BudgetApp";

export default function BudgetPage(): React.JSX.Element {
  return (
    <>
      <NavBar title="Developer Sam" titleLink="/" navItems={[{ name: "Blog", link: "/blog" }]} />
      <Suspense fallback={<LoadingShell />}>
        <AuthGate>
          <BudgetApp />
        </AuthGate>
      </Suspense>
    </>
  );
}

function LoadingShell(): React.JSX.Element {
  return (
    <div className="mx-auto max-w-6xl px-4 py-12">
      <div className="text-center text-gray-500 dark:text-gray-400">Loading…</div>
    </div>
  );
}
