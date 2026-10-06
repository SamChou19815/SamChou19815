"use client";

import { Suspense } from "react";
import AuthGate, { signedInEmail } from "../../lib/AuthGate";
import { useAuth } from "../../lib/useAuth";
import BudgetApp from "./BudgetApp";
import { useWhitelist } from "./useWhitelist";

export default function BudgetPage(): React.JSX.Element {
  const auth = useAuth();
  const access = useWhitelist(signedInEmail(auth));
  return (
    <Suspense fallback={<LoadingShell />}>
      <AuthGate
        title="Budget"
        signedOutPrompt="Sign in to view your dashboard."
        deniedMessage="Access denied — your account is not on the whitelist."
        allowSignUp
        signUpRedirectPath="/budget"
        auth={auth}
        access={access}
      >
        <BudgetApp />
      </AuthGate>
    </Suspense>
  );
}

function LoadingShell(): React.JSX.Element {
  return (
    <div className="mx-auto max-w-6xl px-4 py-12">
      <div className="text-center text-gray-500 dark:text-gray-400">Loading…</div>
    </div>
  );
}
