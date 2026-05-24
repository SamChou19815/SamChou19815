"use client";

import { Suspense } from "react";
import AuthGate from "../../lib/AuthGate";
import NavBar from "../../lib/NavBar";
import BudgetApp from "./BudgetApp";
import { useWhitelist } from "./useWhitelist";

export default function BudgetPage(): React.JSX.Element {
  return (
    <>
      <NavBar title="Developer Sam" titleLink="/" navItems={[{ name: "Blog", link: "/blog" }]} />
      <Suspense fallback={<LoadingShell />}>
        <AuthGate
          title="Budget"
          signedOutPrompt="Sign in to view your dashboard."
          deniedMessage="Access denied — your account is not on the whitelist."
          allowSignUp
          signUpRedirectPath="/budget"
          useAccessCheck={useWhitelist}
        >
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
