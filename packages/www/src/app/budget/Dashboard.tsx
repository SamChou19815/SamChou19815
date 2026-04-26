"use client";

import { useMemo, useState } from "react";
import { Card } from "./BudgetApp";
import {
  AllocationDonut,
  CashFlowLine,
  ExpenseByCategoryPie,
  IncomeByCategoryPie,
  InvestmentValueLine,
  MonthlyBar,
} from "./Charts";
import MonthlySummary from "./MonthlySummary";
import TimeRangeSelector, {
  presetLabel,
  rangeFromPreset,
  type RangePreset,
  type TimeRange,
} from "./TimeRangeSelector";
import type { Expense, Income, Investment, InvestmentSnapshot } from "./types";
import { cadValue, formatCAD, parseLocalDate } from "./utils";

const PRESETS: ReadonlyArray<RangePreset> = ["MONTH", "3M", "6M", "12M", "YTD", "ALL", "CUSTOM"];

function dateInRange(dateStr: string, range: TimeRange): boolean {
  const d = parseLocalDate(dateStr);
  return d >= range.start && d <= range.end;
}

type Props = {
  expenses: ReadonlyArray<Expense>;
  incomes: ReadonlyArray<Income>;
  investments: ReadonlyArray<Investment>;
  snapshots: ReadonlyArray<InvestmentSnapshot>;
  loading: boolean;
};

function StatTile({ label, value }: { label: string; value: string }): React.JSX.Element {
  return (
    <Card>
      <div className="text-center">
        <div className="text-2xl font-bold text-blue-500 sm:text-3xl dark:text-blue-400">
          {value}
        </div>
        <div className="mt-1 text-sm text-gray-500 dark:text-gray-400">{label}</div>
      </div>
    </Card>
  );
}

export default function Dashboard({
  expenses,
  incomes,
  investments,
  snapshots,
  loading,
}: Props): React.JSX.Element {
  const [range, setRange] = useState<TimeRange>(() => rangeFromPreset("12M"));

  const stats = useMemo(() => {
    const rangeIncome = incomes
      .filter((i) => dateInRange(i.date, range))
      .reduce((s, i) => s + Number(i.amount), 0);
    const rangeExpense = expenses
      .filter((e) => dateInRange(e.date, range))
      .reduce((s, e) => s + Number(e.amount), 0);
    const rangeNet = rangeIncome - rangeExpense;
    const investTotal = investments.reduce((s, inv) => s + cadValue(inv), 0);
    return { rangeIncome, rangeExpense, rangeNet, investTotal };
  }, [expenses, incomes, investments, range]);

  if (loading) {
    return (
      <Card>
        <div className="text-center text-gray-500 dark:text-gray-400">Loading…</div>
      </Card>
    );
  }

  const tileLabel = presetLabel(range.preset);

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center justify-end gap-2">
          <TimeRangeSelector value={range} onChange={setRange} presets={PRESETS} />
        </div>
        <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
          <StatTile label={`Income (${tileLabel})`} value={formatCAD(stats.rangeIncome)} />
          <StatTile label={`Expenses (${tileLabel})`} value={formatCAD(stats.rangeExpense)} />
          <StatTile label={`Net (${tileLabel})`} value={formatCAD(stats.rangeNet)} />
          <StatTile label="Investments (CAD)" value={formatCAD(stats.investTotal)} />
        </div>
      </div>

      <MonthlySummary incomes={incomes} expenses={expenses} range={range} />

      <Card>
        <h3 className="m-0 mb-4">Charts</h3>
        <div className="grid grid-cols-1 gap-8">
          <ChartSection title="Income vs Expenses (monthly)">
            <MonthlyBar range={range} incomes={incomes} expenses={expenses} />
          </ChartSection>
          <ChartSection title="Cumulative cash flow">
            <CashFlowLine range={range} incomes={incomes} expenses={expenses} />
          </ChartSection>
          <ChartSection title="Investment market value over time">
            <InvestmentValueLine range={range} snapshots={snapshots} investments={investments} />
          </ChartSection>
          <div className="grid grid-cols-1 gap-8 md:grid-cols-3">
            <ChartSection title="Income by category">
              <IncomeByCategoryPie range={range} incomes={incomes} />
            </ChartSection>
            <ChartSection title="Expenses by category">
              <ExpenseByCategoryPie range={range} expenses={expenses} />
            </ChartSection>
            <ChartSection title="Current allocation by type">
              <AllocationDonut investments={investments} />
            </ChartSection>
          </div>
        </div>
      </Card>
    </div>
  );
}

function ChartSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <div>
      <h4 className="mb-2 text-sm font-medium text-gray-700 dark:text-gray-200">{title}</h4>
      {children}
    </div>
  );
}
