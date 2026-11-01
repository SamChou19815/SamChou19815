import { createFileRoute } from "@tanstack/react-router";
import { appIconHead } from "../lib/appIcons";
import { Suspense } from "react";
import BudgetApp from "../app/budget/BudgetApp";
import { useWhitelist } from "../app/budget/useWhitelist";
import AuthGate, { signedInEmail } from "../lib/AuthGate";
import { useAuth } from "../lib/useAuth";

export const Route = createFileRoute("/budget")({
  head: () => {
    const icon = appIconHead("budget", "Budget");
    return {
      meta: [
        { title: "Budget | Developer Sam" },
        { name: "robots", content: "noindex, nofollow" },
        ...icon.meta,
      ],
      links: icon.links,
    };
  },
  validateSearch: (search: Record<string, unknown>): { tab?: string } => ({
    tab: typeof search.tab === "string" ? search.tab : undefined,
  }),
  component: BudgetPage,
});

function BudgetPage(): React.JSX.Element {
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
