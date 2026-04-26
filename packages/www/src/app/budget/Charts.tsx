"use client";

import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  ComposedChart,
  Legend,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { Expense, Income, Investment, InvestmentSnapshot } from "./types";
import {
  cadValue,
  formatCAD,
  latestSnapshotByMonth,
  monthBucket,
  monthsBetween,
  snapshotCadValue,
} from "./utils";

const PALETTE = [
  "#3b82f6",
  "#ef4444",
  "#10b981",
  "#f59e0b",
  "#8b5cf6",
  "#ec4899",
  "#14b8a6",
  "#f97316",
  "#6366f1",
  "#84cc16",
];

const INCOME_PALETTE = ["#10b981", "#14b8a6", "#22c55e", "#06b6d4", "#84cc16", "#0ea5e9"];
const EXPENSE_PALETTE = ["#ef4444", "#f97316", "#f59e0b", "#ec4899", "#dc2626", "#a855f7"];

type Range = { start: Date; end: Date };

const NO_SOURCE = "(no source)";

export type MonthlyTotal = {
  month: string;
  income: number;
  expense: number;
  net: number;
};

export function computeMonthlyTotals(
  range: Range,
  incomes: ReadonlyArray<Income>,
  expenses: ReadonlyArray<Expense>,
): MonthlyTotal[] {
  const months = monthsBetween(range.start, range.end);
  const incomeBy = new Map<string, number>();
  const expenseBy = new Map<string, number>();
  for (const i of incomes) {
    const k = monthBucket(i.date);
    incomeBy.set(k, (incomeBy.get(k) ?? 0) + Number(i.amount));
  }
  for (const e of expenses) {
    const k = monthBucket(e.date);
    expenseBy.set(k, (expenseBy.get(k) ?? 0) + Number(e.amount));
  }
  return months.map((m) => {
    const income = incomeBy.get(m) ?? 0;
    const expense = expenseBy.get(m) ?? 0;
    return { month: m, income, expense, net: income - expense };
  });
}

export function MonthlyTotalsChart({
  range,
  incomes,
  expenses,
}: {
  range: Range;
  incomes: ReadonlyArray<Income>;
  expenses: ReadonlyArray<Expense>;
}): React.JSX.Element {
  const data = computeMonthlyTotals(range, incomes, expenses);
  return (
    <ResponsiveContainer width="100%" height={300}>
      <ComposedChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
        <XAxis dataKey="month" tick={{ fontSize: 12 }} />
        <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => `$${Math.round(v / 1000)}k`} />
        <Tooltip formatter={(v: number) => formatCAD(v)} />
        <Legend />
        <Bar dataKey="income" fill="#10b981" name="Income" />
        <Bar dataKey="expense" fill="#ef4444" name="Expenses" />
        <Line type="monotone" dataKey="net" stroke="#3b82f6" name="Net" dot={false} />
      </ComposedChart>
    </ResponsiveContainer>
  );
}

export function MonthlyBar({
  range,
  incomes,
  expenses,
}: {
  range: Range;
  incomes: ReadonlyArray<Income>;
  expenses: ReadonlyArray<Expense>;
}): React.JSX.Element {
  const months = monthsBetween(range.start, range.end);
  const monthSet = new Set(months);

  const incomeCategories = Array.from(
    new Set(
      incomes.filter((i) => monthSet.has(monthBucket(i.date))).map((i) => i.source ?? NO_SOURCE),
    ),
  ).sort();
  const expenseCategories = Array.from(
    new Set(expenses.filter((e) => monthSet.has(monthBucket(e.date))).map((e) => e.category)),
  ).sort();

  const data = months.map((m) => {
    const row: Record<string, number | string> = { month: m };
    for (const cat of incomeCategories) {
      row[`income:${cat}`] = incomes
        .filter((i) => monthBucket(i.date) === m && (i.source ?? NO_SOURCE) === cat)
        .reduce((s, i) => s + Number(i.amount), 0);
    }
    for (const cat of expenseCategories) {
      row[`expense:${cat}`] = expenses
        .filter((e) => monthBucket(e.date) === m && e.category === cat)
        .reduce((s, e) => s + Number(e.amount), 0);
    }
    return row;
  });

  return (
    <ResponsiveContainer width="100%" height={300}>
      <BarChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
        <XAxis dataKey="month" tick={{ fontSize: 12 }} />
        <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => `$${Math.round(v / 1000)}k`} />
        <Tooltip formatter={(v: number) => formatCAD(v)} />
        <Legend />
        {incomeCategories.map((cat, idx) => (
          <Bar
            key={`income:${cat}`}
            dataKey={`income:${cat}`}
            stackId="income"
            fill={INCOME_PALETTE[idx % INCOME_PALETTE.length]}
            name={cat}
          />
        ))}
        {expenseCategories.map((cat, idx) => (
          <Bar
            key={`expense:${cat}`}
            dataKey={`expense:${cat}`}
            stackId="expense"
            fill={EXPENSE_PALETTE[idx % EXPENSE_PALETTE.length]}
            name={cat}
          />
        ))}
      </BarChart>
    </ResponsiveContainer>
  );
}

export function CashFlowLine({
  range,
  incomes,
  expenses,
}: {
  range: Range;
  incomes: ReadonlyArray<Income>;
  expenses: ReadonlyArray<Expense>;
}): React.JSX.Element {
  const months = monthsBetween(range.start, range.end);
  const incomeBy = bucketSum(incomes, (i) => Number(i.amount));
  const expenseBy = bucketSum(expenses, (e) => Number(e.amount));
  const allMonths = monthsBetween(new Date(2000, 0, 1), range.end);
  let runningIncome = 0;
  let runningExpense = 0;
  const cashFlowMap = new Map<string, number>();
  for (const m of allMonths) {
    runningIncome += incomeBy.get(m) ?? 0;
    runningExpense += expenseBy.get(m) ?? 0;
    cashFlowMap.set(m, runningIncome - runningExpense);
  }

  const data = months.map((m) => ({ month: m, cashFlow: cashFlowMap.get(m) ?? 0 }));

  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
        <XAxis dataKey="month" tick={{ fontSize: 12 }} />
        <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => `$${Math.round(v / 1000)}k`} />
        <Tooltip formatter={(v: number) => formatCAD(v)} />
        <Legend />
        <Line type="monotone" dataKey="cashFlow" stroke={PALETTE[0]} name="Cash flow" dot={false} />
      </LineChart>
    </ResponsiveContainer>
  );
}

export function InvestmentValueLine({
  range,
  snapshots,
  investments,
}: {
  range: Range;
  snapshots: ReadonlyArray<InvestmentSnapshot>;
  investments: ReadonlyArray<Investment>;
}): React.JSX.Element {
  const months = monthsBetween(range.start, range.end);
  const snapsByInvestment = new Map<string, InvestmentSnapshot[]>();
  for (const s of snapshots) {
    const arr = snapsByInvestment.get(s.investment_id) ?? [];
    arr.push(s);
    snapsByInvestment.set(s.investment_id, arr);
  }
  const data = months.map((m) => {
    let total = 0;
    for (const inv of investments) {
      const arr = snapsByInvestment.get(inv.id) ?? [];
      const map = latestSnapshotByMonth(arr, [m]);
      const snap = map.get(m);
      if (snap != null) total += snapshotCadValue(snap);
    }
    return { month: m, value: total };
  });
  return (
    <ResponsiveContainer width="100%" height={300}>
      <LineChart data={data}>
        <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
        <XAxis dataKey="month" tick={{ fontSize: 12 }} />
        <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => `$${Math.round(v / 1000)}k`} />
        <Tooltip formatter={(v: number) => formatCAD(v)} />
        <Line
          type="monotone"
          dataKey="value"
          stroke={PALETTE[6]}
          name="Market value (CAD)"
          dot={false}
        />
      </LineChart>
    </ResponsiveContainer>
  );
}

export function IncomeByCategoryPie({
  range,
  incomes,
}: {
  range: Range;
  incomes: ReadonlyArray<Income>;
}): React.JSX.Element {
  const months = new Set(monthsBetween(range.start, range.end));
  const byCat = new Map<string, number>();
  for (const i of incomes) {
    if (!months.has(monthBucket(i.date))) continue;
    const k = i.source ?? NO_SOURCE;
    byCat.set(k, (byCat.get(k) ?? 0) + Number(i.amount));
  }
  const data = Array.from(byCat, ([name, value]) => ({ name, value })).filter((d) => d.value > 0);
  if (data.length === 0) {
    return (
      <div className="flex h-[300px] items-center justify-center text-sm text-gray-500 dark:text-gray-400">
        No income in range.
      </div>
    );
  }
  return (
    <ResponsiveContainer width="100%" height={300}>
      <PieChart>
        <Pie data={data} dataKey="value" nameKey="name" innerRadius={60} outerRadius={100} label>
          {data.map((entry, idx) => (
            <Cell key={entry.name} fill={INCOME_PALETTE[idx % INCOME_PALETTE.length]} />
          ))}
        </Pie>
        <Tooltip formatter={(v: number) => formatCAD(v)} />
        <Legend />
      </PieChart>
    </ResponsiveContainer>
  );
}

export function ExpenseByCategoryPie({
  range,
  expenses,
}: {
  range: Range;
  expenses: ReadonlyArray<Expense>;
}): React.JSX.Element {
  const months = new Set(monthsBetween(range.start, range.end));
  const byCat = new Map<string, number>();
  for (const e of expenses) {
    if (!months.has(monthBucket(e.date))) continue;
    byCat.set(e.category, (byCat.get(e.category) ?? 0) + Number(e.amount));
  }
  const data = Array.from(byCat, ([name, value]) => ({ name, value })).filter((d) => d.value > 0);
  if (data.length === 0) {
    return (
      <div className="flex h-[300px] items-center justify-center text-sm text-gray-500 dark:text-gray-400">
        No expenses in range.
      </div>
    );
  }
  return (
    <ResponsiveContainer width="100%" height={300}>
      <PieChart>
        <Pie data={data} dataKey="value" nameKey="name" innerRadius={60} outerRadius={100} label>
          {data.map((entry, idx) => (
            <Cell key={entry.name} fill={EXPENSE_PALETTE[idx % EXPENSE_PALETTE.length]} />
          ))}
        </Pie>
        <Tooltip formatter={(v: number) => formatCAD(v)} />
        <Legend />
      </PieChart>
    </ResponsiveContainer>
  );
}

export function AllocationDonut({
  investments,
}: {
  investments: ReadonlyArray<Investment>;
}): React.JSX.Element {
  const byType = new Map<string, number>();
  for (const inv of investments) {
    byType.set(inv.type, (byType.get(inv.type) ?? 0) + cadValue(inv));
  }
  const data = Array.from(byType, ([name, value]) => ({ name, value })).filter((d) => d.value > 0);
  if (data.length === 0) {
    return (
      <div className="flex h-[300px] items-center justify-center text-sm text-gray-500 dark:text-gray-400">
        No investments yet.
      </div>
    );
  }
  return (
    <ResponsiveContainer width="100%" height={300}>
      <PieChart>
        <Pie data={data} dataKey="value" nameKey="name" innerRadius={60} outerRadius={100} label>
          {data.map((entry, idx) => (
            <Cell key={entry.name} fill={PALETTE[idx % PALETTE.length]} />
          ))}
        </Pie>
        <Tooltip formatter={(v: number) => formatCAD(v)} />
        <Legend />
      </PieChart>
    </ResponsiveContainer>
  );
}

function bucketSum<T extends { date: string }>(
  rows: ReadonlyArray<T>,
  pick: (r: T) => number,
): Map<string, number> {
  const out = new Map<string, number>();
  for (const r of rows) {
    const key = monthBucket(r.date);
    out.set(key, (out.get(key) ?? 0) + pick(r));
  }
  return out;
}
