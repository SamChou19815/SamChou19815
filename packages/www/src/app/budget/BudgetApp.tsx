"use client";

import type { ReactNode } from "react";
import { getSupabase } from "../../lib/supabase";
import Dashboard from "./Dashboard";
import ExpensesPanel from "./ExpensesPanel";
import FirePanel from "./FirePanel";
import IncomesPanel from "./IncomesPanel";
import InvestmentsPanel from "./InvestmentsPanel";
import Tabs, { useActiveTab } from "./Tabs";
import { useAuth } from "./useAuth";
import { useRealtimeRows } from "./useRealtimeRows";
import type { Expense, Income, Investment, InvestmentSnapshot } from "./types";

export function Card({
  className,
  children,
}: {
  className?: string;
  children: ReactNode;
}): React.JSX.Element {
  const base =
    "flex flex-col bg-white rounded filter drop-shadow hover:drop-shadow-lg transition-all duration-300 ease-out p-6 dark:bg-[#242424] dark:drop-shadow-[0_1px_3px_rgba(0,0,0,0.3)] dark:hover:drop-shadow-[0_4px_12px_rgba(0,0,0,0.4)]";
  return <div className={className != null ? `${base} ${className}` : base}>{children}</div>;
}

export default function BudgetApp(): React.JSX.Element {
  const { session } = useAuth();
  const userId = session?.user.id ?? null;
  const email = session?.user.email ?? null;
  const active = useActiveTab();

  const expenses = useRealtimeRows<Expense>({
    table: "expenses",
    userId,
    orderBy: { column: "date", ascending: false },
  });
  const incomes = useRealtimeRows<Income>({
    table: "incomes",
    userId,
    orderBy: { column: "date", ascending: false },
  });
  const investments = useRealtimeRows<Investment>({
    table: "investments",
    userId,
    orderBy: { column: "updated_at", ascending: false },
  });
  const snapshots = useRealtimeRows<InvestmentSnapshot>({
    table: "investment_snapshots",
    userId,
    orderBy: { column: "recorded_at", ascending: false },
  });

  const onSignOut = async () => {
    await getSupabase().auth.signOut();
  };

  return (
    <div className="mx-auto max-w-6xl px-4 py-8">
      <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
        <h1 className="m-0">Budget</h1>
        <div className="flex items-center gap-3 text-sm">
          {email && (
            <span className="rounded bg-blue-50 px-2 py-1 font-medium text-blue-600 dark:bg-blue-900/30 dark:text-blue-300">
              {email}
            </span>
          )}
          <button
            type="button"
            onClick={onSignOut}
            className="rounded border border-gray-300 px-3 py-1 font-medium text-gray-700 transition-colors duration-200 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800"
          >
            Sign out
          </button>
        </div>
      </div>

      <div className="mb-6">
        <Tabs />
      </div>

      {active === "dashboard" && (
        <Dashboard
          expenses={expenses.rows}
          incomes={incomes.rows}
          investments={investments.rows}
          snapshots={snapshots.rows}
          loading={expenses.loading || incomes.loading || investments.loading || snapshots.loading}
        />
      )}
      {active === "expenses" && (
        <ExpensesPanel
          userId={userId}
          rows={expenses.rows}
          loading={expenses.loading}
          error={expenses.error}
        />
      )}
      {active === "income" && (
        <IncomesPanel
          userId={userId}
          rows={incomes.rows}
          loading={incomes.loading}
          error={incomes.error}
        />
      )}
      {active === "investments" && (
        <InvestmentsPanel
          userId={userId}
          investments={investments.rows}
          investmentsLoading={investments.loading}
          investmentsError={investments.error}
          snapshots={snapshots.rows}
          snapshotsLoading={snapshots.loading}
        />
      )}
      {active === "fire" && (
        <FirePanel
          expenses={expenses.rows}
          incomes={incomes.rows}
          investments={investments.rows}
          loading={expenses.loading || incomes.loading || investments.loading}
        />
      )}
    </div>
  );
}
