"use client";

import { useState } from "react";
import { Card } from "./BudgetApp";
import { computeMonthlyTotals, MonthlyTotalsChart } from "./Charts";
import type { TimeRange } from "./TimeRangeSelector";
import type { Expense, Income } from "./types";
import { formatCAD } from "./utils";

type View = "table" | "chart";

export default function MonthlySummary({
  incomes,
  expenses,
  range,
}: {
  incomes: ReadonlyArray<Income>;
  expenses: ReadonlyArray<Expense>;
  range: TimeRange;
}): React.JSX.Element {
  const [view, setView] = useState<View>("table");

  return (
    <Card>
      <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
        <h3 className="m-0">Monthly summary</h3>
        <ViewToggle value={view} onChange={setView} />
      </div>
      {view === "table" ? (
        <MonthlyTotalsTable incomes={incomes} expenses={expenses} range={range} />
      ) : (
        <MonthlyTotalsChart range={range} incomes={incomes} expenses={expenses} />
      )}
    </Card>
  );
}

function ViewToggle({
  value,
  onChange,
}: {
  value: View;
  onChange: (v: View) => void;
}): React.JSX.Element {
  const options: ReadonlyArray<{ id: View; label: string }> = [
    { id: "table", label: "Table" },
    { id: "chart", label: "Chart" },
  ];
  return (
    <div className="flex flex-wrap items-center gap-2">
      {options.map((o) => {
        const active = o.id === value;
        const cls = active
          ? "rounded bg-blue-500 px-3 py-1 text-sm font-medium text-white dark:bg-blue-400 dark:text-gray-900"
          : "rounded border border-gray-300 px-3 py-1 text-sm font-medium text-gray-700 hover:bg-gray-100 dark:border-gray-600 dark:text-gray-200 dark:hover:bg-gray-800";
        return (
          <button key={o.id} type="button" onClick={() => onChange(o.id)} className={cls}>
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

function MonthlyTotalsTable({
  incomes,
  expenses,
  range,
}: {
  incomes: ReadonlyArray<Income>;
  expenses: ReadonlyArray<Expense>;
  range: TimeRange;
}): React.JSX.Element {
  const rows = computeMonthlyTotals(range, incomes, expenses);
  const totals = rows.reduce(
    (acc, r) => {
      acc.income += r.income;
      acc.expense += r.expense;
      acc.net += r.net;
      return acc;
    },
    { income: 0, expense: 0, net: 0 },
  );

  if (rows.length === 0) {
    return (
      <div className="flex h-[120px] items-center justify-center text-sm text-gray-500 dark:text-gray-400">
        No months in range.
      </div>
    );
  }

  const headerCls =
    "px-3 py-2 text-left text-xs font-medium uppercase tracking-wide text-gray-500 dark:text-gray-400";
  const cellCls = "px-3 py-2 text-sm text-gray-800 dark:text-gray-100";
  const numCls = `${cellCls} text-right tabular-nums`;

  return (
    <div className="overflow-x-auto">
      <table className="min-w-full border-collapse">
        <thead>
          <tr className="border-b border-gray-200 dark:border-gray-700">
            <th className={headerCls}>Month</th>
            <th className={`${headerCls} text-right`}>Income</th>
            <th className={`${headerCls} text-right`}>Expenses</th>
            <th className={`${headerCls} text-right`}>Net</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr
              key={r.month}
              className="border-b border-gray-100 last:border-0 hover:bg-gray-50 dark:border-gray-800 dark:hover:bg-gray-800/50"
            >
              <td className={cellCls}>{r.month}</td>
              <td className={`${numCls} text-emerald-600 dark:text-emerald-400`}>
                {formatCAD(r.income)}
              </td>
              <td className={`${numCls} text-red-600 dark:text-red-400`}>{formatCAD(r.expense)}</td>
              <td className={`${numCls} ${netColor(r.net)}`}>{formatCAD(r.net)}</td>
            </tr>
          ))}
        </tbody>
        <tfoot>
          <tr className="border-t-2 border-gray-300 dark:border-gray-600">
            <td className={`${cellCls} font-semibold`}>Total</td>
            <td className={`${numCls} font-semibold text-emerald-600 dark:text-emerald-400`}>
              {formatCAD(totals.income)}
            </td>
            <td className={`${numCls} font-semibold text-red-600 dark:text-red-400`}>
              {formatCAD(totals.expense)}
            </td>
            <td className={`${numCls} font-semibold ${netColor(totals.net)}`}>
              {formatCAD(totals.net)}
            </td>
          </tr>
        </tfoot>
      </table>
    </div>
  );
}

function netColor(net: number): string {
  if (net > 0) return "text-emerald-600 dark:text-emerald-400";
  if (net < 0) return "text-red-600 dark:text-red-400";
  return "text-gray-600 dark:text-gray-300";
}
