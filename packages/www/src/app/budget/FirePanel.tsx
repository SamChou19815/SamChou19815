"use client";

import { useDeferredValue, useMemo, useState } from "react";
import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { Card } from "./BudgetApp";
import type { Expense, Income, Investment } from "./types";
import { cadValue, formatCAD, formatCADCompact, parseLocalDate } from "./utils";

type Props = {
  expenses: ReadonlyArray<Expense>;
  incomes: ReadonlyArray<Income>;
  investments: ReadonlyArray<Investment>;
  loading: boolean;
};

function lastCalendarYear(): number {
  return new Date().getFullYear() - 1;
}

function sumForYear<T extends { date: string; amount: number }>(
  rows: ReadonlyArray<T>,
  year: number,
): number {
  return rows
    .filter((r) => parseLocalDate(r.date).getFullYear() === year)
    .reduce((s, r) => s + Number(r.amount), 0);
}

function sumRollingYear<T extends { date: string; amount: number }>(
  rows: ReadonlyArray<T>,
): number {
  const today = new Date();
  const cutoff = new Date(today.getFullYear() - 1, today.getMonth(), today.getDate());
  return rows
    .filter((r) => parseLocalDate(r.date).getTime() > cutoff.getTime())
    .reduce((s, r) => s + Number(r.amount), 0);
}

type SharedInputs = {
  annualExpenses: string;
  withdrawalRate: string;
  portfolio: string;
  annualSavings: string;
  expectedReturn: string;
  inflation: string;
};

type SharedSetters = {
  setAnnualExpenses: (v: string) => void;
  setWithdrawalRate: (v: string) => void;
  setPortfolio: (v: string) => void;
  setAnnualSavings: (v: string) => void;
  setExpectedReturn: (v: string) => void;
  setInflation: (v: string) => void;
};

export default function FirePanel({
  expenses,
  incomes,
  investments,
  loading,
}: Props): React.JSX.Element {
  const year = lastCalendarYear();

  const defaults = useMemo(() => {
    const annualExpenses = sumRollingYear(expenses);
    const annualIncomeYear = sumForYear(incomes, year);
    const annualExpensesYear = sumForYear(expenses, year);
    const annualSavings = Math.max(0, annualIncomeYear - annualExpensesYear);
    const portfolio = investments.reduce((s, inv) => s + cadValue(inv), 0);
    return { annualExpenses, annualIncome: annualIncomeYear, annualSavings, portfolio };
  }, [expenses, incomes, investments, year]);

  if (loading) {
    return (
      <Card>
        <div className="text-center text-gray-500 dark:text-gray-400">Loading…</div>
      </Card>
    );
  }

  return <FirePlanner year={year} defaults={defaults} />;
}

type Defaults = {
  annualExpenses: number;
  annualIncome: number;
  annualSavings: number;
  portfolio: number;
};

type FlavorKey = "classic" | "coast";

type Flavor = {
  key: FlavorKey;
  label: string;
  color: string;
  target: number;
  sim: SimYear[];
  switchYear: number | null;
};

function FirePlanner({ year, defaults }: { year: number; defaults: Defaults }): React.JSX.Element {
  const [annualExpenses, setAnnualExpenses] = useState(() =>
    String(round2(defaults.annualExpenses)),
  );
  const [withdrawalRate, setWithdrawalRate] = useState("2");
  const [portfolio, setPortfolio] = useState(() => String(round2(defaults.portfolio)));
  const [annualSavings, setAnnualSavings] = useState(() => String(round2(defaults.annualSavings)));
  const [expectedReturn, setExpectedReturn] = useState("7");
  const [inflation, setInflation] = useState("2.5");
  const [currentAge, setCurrentAge] = useState(String(new Date().getFullYear() - 1998));
  const [retireAge, setRetireAge] = useState("45");
  const [coastStartAge, setCoastStartAge] = useState("30");
  const [endAge, setEndAge] = useState("95");
  const [mcStdDev, setMcStdDev] = useState("12");
  const [mcNumSims, setMcNumSims] = useState("10000");

  const hasLastYearData = defaults.annualExpenses > 0 || defaults.annualIncome > 0;
  const shared: SharedInputs = {
    annualExpenses,
    withdrawalRate,
    portfolio,
    annualSavings,
    expectedReturn,
    inflation,
  };
  const setters: SharedSetters = {
    setAnnualExpenses,
    setWithdrawalRate,
    setPortfolio,
    setAnnualSavings,
    setExpectedReturn,
    setInflation,
  };

  const exp = numOr(annualExpenses, 0);
  const wr = numOr(withdrawalRate, 0) / 100;
  const pv = numOr(portfolio, 0);
  const sav = numOr(annualSavings, 0);
  const r = realReturn(expectedReturn, inflation);
  const age = numOr(currentAge, 0);
  const retire = numOr(retireAge, 0);
  const coastStart = numOr(coastStartAge, 0);
  const end = numOr(endAge, 0);
  // The four ages are the single source of truth; horizon and phase lengths derive from them.
  const horizonYears = clampHorizon(end - age);
  const accumulationYears = Math.max(0, Math.round(coastStart - age));
  const coastYears = Math.max(0, Math.round(retire - Math.max(age, coastStart)));

  const fireNumber = wr > 0 ? exp / wr : Number.POSITIVE_INFINITY;
  const yearsToRetire = Math.max(0, retire - age);
  const growth = Math.pow(1 + r, yearsToRetire);
  const coastTarget = growth > 0 ? fireNumber / growth : Number.POSITIVE_INFINITY;
  const coastHorizon = clampHorizon(Math.max(yearsToRetire + 1, end - age));

  const classicSim = useMemo(
    () =>
      simulate({
        pv,
        r,
        horizonYears,
        contribution: sav,
        withdrawal: exp,
        switchAt: (_y, p) => p >= fireNumber,
      }),
    [pv, r, horizonYears, sav, exp, fireNumber],
  );
  const coastSim = useMemo(
    () =>
      simulate({
        pv,
        r,
        horizonYears: coastHorizon,
        contribution: 0,
        withdrawal: exp,
        switchAt: (y) => y >= yearsToRetire,
      }),
    [pv, r, coastHorizon, exp, yearsToRetire],
  );

  const flavors: Flavor[] = [
    {
      key: "classic",
      label: "FIRE",
      color: "#3b82f6",
      target: fireNumber,
      sim: classicSim,
      switchYear: firstWithdrawYear(classicSim),
    },
    {
      key: "coast",
      label: "Coast FIRE",
      color: "#8b5cf6",
      target: fireNumber,
      sim: coastSim,
      switchYear: yearsToRetire,
    },
  ];

  const deferredExp = useDeferredValue(exp);
  const deferredSav = useDeferredValue(sav);
  const deferredR = useDeferredValue(r);
  const deferredPv = useDeferredValue(pv);
  const deferredAccumulationYears = useDeferredValue(accumulationYears);
  const deferredCoastYears = useDeferredValue(coastYears);
  const deferredHorizonYears = useDeferredValue(horizonYears);
  const deferredMcStdDev = useDeferredValue(mcStdDev);
  const deferredMcNumSims = useDeferredValue(mcNumSims);
  const mcResult = useMemo(() => {
    if (deferredPv <= 0 || deferredExp <= 0) return null;
    const sd = numOr(deferredMcStdDev, 12) / 100;
    const ns = Math.min(50000, Math.max(100, Math.round(numOr(deferredMcNumSims, 10000))));
    return monteCarloSimulate({
      startPortfolio: deferredPv,
      accumulationYears: deferredAccumulationYears,
      annualContribution: deferredSav,
      coastYears: deferredCoastYears,
      annualWithdrawal: deferredExp,
      realReturn: deferredR,
      realStdDev: sd,
      horizonYears: deferredHorizonYears,
      numSims: ns,
    });
  }, [
    deferredPv,
    deferredAccumulationYears,
    deferredSav,
    deferredCoastYears,
    deferredExp,
    deferredR,
    deferredMcStdDev,
    deferredHorizonYears,
    deferredMcNumSims,
  ]);

  return (
    <div className="flex flex-col gap-6">
      <Card>
        <div className="flex flex-col gap-1">
          <h3 className="m-0">FIRE planning</h3>
          <p className="m-0 text-sm text-gray-600 dark:text-gray-400">
            Numbers below are pre-filled from your {year} totals (income, expenses) and current
            investment value, but you can edit any field to explore scenarios. All amounts are in
            CAD.
          </p>
          {!hasLastYearData && (
            <p className="m-0 mt-2 rounded bg-amber-50 px-3 py-2 text-sm text-amber-800 dark:bg-amber-900/20 dark:text-amber-200">
              No income or expense rows found for {year}. Defaults are zeroed — fill in the fields
              manually.
            </p>
          )}
        </div>
      </Card>

      <SharedInputsCard year={year} shared={shared} setters={setters} />

      <ClassicFire pv={pv} sav={sav} r={r} exp={exp} target={fireNumber} wrPct={withdrawalRate} />
      <CoastFire
        pv={pv}
        r={r}
        retire={retire}
        currentAge={currentAge}
        setCurrentAge={setCurrentAge}
        retireAge={retireAge}
        setRetireAge={setRetireAge}
        coastStartAge={coastStartAge}
        setCoastStartAge={setCoastStartAge}
        endAge={endAge}
        setEndAge={setEndAge}
        target={coastTarget}
        fireNumber={fireNumber}
      />
      <MonteCarloCard
        result={mcResult}
        stdDev={mcStdDev}
        setStdDev={setMcStdDev}
        numSims={mcNumSims}
        setNumSims={setMcNumSims}
        startPortfolio={pv}
        accumulationYears={accumulationYears}
        coastYears={coastYears}
        horizonYears={horizonYears}
      />

      <Card>
        <CalculatorHeader
          title="Combined projection"
          subtitle="Portfolio trajectories under each FIRE flavor, in today's dollars. Vertical markers show when each strategy switches from accumulation to withdrawal."
        />
        <CombinedProjectionChart flavors={flavors} />
      </Card>
    </div>
  );
}

function SharedInputsCard({
  year,
  shared,
  setters,
}: {
  year: number;
  shared: SharedInputs;
  setters: SharedSetters;
}): React.JSX.Element {
  return (
    <Card>
      <CalculatorHeader
        title="Shared inputs"
        subtitle="Edit once — these values feed every calculator below."
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <NumberField
          label="Annual expenses (last 12 mo)"
          value={shared.annualExpenses}
          onChange={setters.setAnnualExpenses}
        />
        <NumberField
          label="Withdrawal rate (%)"
          value={shared.withdrawalRate}
          onChange={setters.setWithdrawalRate}
          step="0.1"
        />
        <NumberField
          label="Current investments"
          value={shared.portfolio}
          onChange={setters.setPortfolio}
        />
        <NumberField
          label={`Annual savings (${year})`}
          value={shared.annualSavings}
          onChange={setters.setAnnualSavings}
        />
        <NumberField
          label="Nominal return (% / yr)"
          value={shared.expectedReturn}
          onChange={setters.setExpectedReturn}
          step="0.1"
        />
        <NumberField
          label="Inflation (% / yr)"
          value={shared.inflation}
          onChange={setters.setInflation}
          step="0.1"
        />
      </div>
      <p className="mt-3 text-xs text-gray-500 dark:text-gray-400">
        All amounts (FIRE number, gap, projection chart) are shown in today&rsquo;s dollars.
        Calculations use the real return derived from your nominal return and inflation: (1 +
        nominal) / (1 + inflation) − 1.
      </p>
    </Card>
  );
}

function ClassicFire({
  pv,
  sav,
  r,
  exp,
  target,
  wrPct,
}: {
  pv: number;
  sav: number;
  r: number;
  exp: number;
  target: number;
  wrPct: string;
}): React.JSX.Element {
  const gap = Math.max(0, target - pv);
  const years = yearsToReach(pv, target, sav, r);
  return (
    <Card>
      <CalculatorHeader
        title="FIRE"
        subtitle={`Portfolio big enough to fund your annual expenses (${formatCAD(exp)}) indefinitely at your ${wrPct}% withdrawal rate.`}
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <Result label="FIRE number" value={formatCAD(target)} />
        <Result label="Gap from today" value={formatCAD(gap)} />
        <Result label="Years to FIRE" value={formatYears(years)} />
      </div>
    </Card>
  );
}

function CoastFire({
  pv,
  r,
  retire,
  currentAge,
  setCurrentAge,
  retireAge,
  setRetireAge,
  coastStartAge,
  setCoastStartAge,
  endAge,
  setEndAge,
  target,
  fireNumber,
}: {
  pv: number;
  r: number;
  retire: number;
  currentAge: string;
  setCurrentAge: (v: string) => void;
  retireAge: string;
  setRetireAge: (v: string) => void;
  coastStartAge: string;
  setCoastStartAge: (v: string) => void;
  endAge: string;
  setEndAge: (v: string) => void;
  target: number;
  fireNumber: number;
}): React.JSX.Element {
  const age = numOr(currentAge, 0);
  const yearsToRetire = Math.max(0, retire - age);
  const growth = Math.pow(1 + r, yearsToRetire);
  const projected = pv * growth;
  const gap = Math.max(0, target - pv);
  const surplus = Math.max(0, pv - target);

  return (
    <Card>
      <CalculatorHeader
        title="Coast FIRE"
        subtitle="The lump sum you need invested today so it grows into your full FIRE number by retirement, even if you stop contributing. These four ages drive every timeline below: contributions continue until the coast-start age, the portfolio coasts (no contributions, no withdrawals) until the retirement age, then withdrawals run until the plan-until age."
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <NumberField label="Current age" value={currentAge} onChange={setCurrentAge} />
        <NumberField label="Coast start age" value={coastStartAge} onChange={setCoastStartAge} />
        <NumberField
          label="Retirement age (withdrawals begin)"
          value={retireAge}
          onChange={setRetireAge}
        />
        <NumberField label="Plan-until age" value={endAge} onChange={setEndAge} />
      </div>
      <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Result label="FIRE number (at retirement)" value={formatCAD(fireNumber)} />
        <Result label="Coast FIRE target (today)" value={formatCAD(target)} />
        <Result
          label={surplus > 0 ? "Surplus over target" : "Gap to target"}
          value={formatCAD(surplus > 0 ? surplus : gap)}
        />
        <Result label={`Projected at age ${Math.round(retire)}`} value={formatCAD(projected)} />
      </div>
    </Card>
  );
}

type SimYear = {
  year: number;
  portfolio: number;
  withdrawn: number;
  phase: "accum" | "withdraw";
};

function simulate({
  pv,
  r,
  horizonYears,
  contribution,
  withdrawal,
  switchAt,
}: {
  pv: number;
  r: number;
  horizonYears: number;
  contribution: number;
  withdrawal: number;
  switchAt: (year: number, portfolio: number) => boolean;
}): SimYear[] {
  const out: SimYear[] = [];
  let cur = Math.max(0, pv);
  let cum = 0;
  let phase: "accum" | "withdraw" = switchAt(0, cur) ? "withdraw" : "accum";
  out.push({ year: 0, portfolio: cur, withdrawn: 0, phase });
  for (let y = 1; y <= horizonYears; y++) {
    if (phase === "accum") {
      cur = cur * (1 + r) + contribution;
      if (switchAt(y, cur)) phase = "withdraw";
    } else {
      const took = Math.min(withdrawal, cur * (1 + r));
      cur = cur * (1 + r) - took;
      cum += took;
      if (cur < 0) cur = 0;
    }
    out.push({ year: y, portfolio: cur, withdrawn: cum, phase });
  }
  return out;
}

function firstWithdrawYear(sim: ReadonlyArray<SimYear>): number | null {
  const found = sim.find((s, i) => i > 0 && s.phase === "withdraw");
  return found ? found.year : null;
}

function CombinedProjectionChart({
  flavors,
}: {
  flavors: ReadonlyArray<Flavor>;
}): React.JSX.Element {
  const maxYear = flavors.reduce((m, f) => {
    const last = f.sim[f.sim.length - 1];
    return Math.max(m, last ? last.year : 0);
  }, 0);
  const data: Array<Record<string, number>> = [];
  for (let y = 0; y <= maxYear; y++) {
    const row: Record<string, number> = { year: y };
    for (const f of flavors) {
      const point = f.sim.find((s) => s.year === y);
      if (point) row[f.key] = Math.round(point.portfolio);
    }
    data.push(row);
  }
  return (
    <div className="mt-2">
      <ResponsiveContainer width="100%" height={600}>
        <LineChart data={data} margin={{ top: 8, right: 16, left: 0, bottom: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
          <XAxis
            dataKey="year"
            tick={{ fontSize: 12 }}
            label={{ value: "Years from now", position: "insideBottom", offset: -4, fontSize: 12 }}
          />
          <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => formatCADCompact(v)} width={60} />
          <Tooltip
            formatter={(v: number, name: string) => [formatCAD(v), name]}
            labelFormatter={(l) => `Year ${l}`}
          />
          <Legend />
          {flavors.map(
            (f) =>
              f.switchYear != null &&
              f.switchYear > 0 &&
              f.switchYear <= maxYear && (
                <ReferenceLine
                  key={`switch-${f.key}`}
                  x={f.switchYear}
                  stroke={f.color}
                  strokeOpacity={0.5}
                  strokeDasharray="3 3"
                />
              ),
          )}
          {flavors.map((f) => (
            <Line
              key={f.key}
              type="monotone"
              dataKey={f.key}
              stroke={f.color}
              name={f.label}
              dot={false}
              strokeWidth={2}
              connectNulls
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

function CalculatorHeader({
  title,
  subtitle,
}: {
  title: string;
  subtitle: string;
}): React.JSX.Element {
  return (
    <div className="mb-4">
      <h3 className="m-0">{title}</h3>
      <p className="mt-1 mb-0 text-sm text-gray-600 dark:text-gray-400">{subtitle}</p>
    </div>
  );
}

function NumberField({
  label,
  value,
  onChange,
  step,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  step?: string;
}): React.JSX.Element {
  return (
    <label className="flex flex-col text-sm">
      <span className="mb-1 text-gray-600 dark:text-gray-400">{label}</span>
      <input
        type="text"
        inputMode="decimal"
        step={step ?? "1"}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full rounded border border-gray-300 bg-white px-2 py-1 text-sm focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
      />
    </label>
  );
}

function Result({ label, value }: { label: string; value: string }): React.JSX.Element {
  return (
    <div className="rounded border border-gray-200 bg-gray-50 px-3 py-2 dark:border-gray-700 dark:bg-gray-900/40">
      <div className="text-xs text-gray-500 dark:text-gray-400">{label}</div>
      <div className="text-lg font-semibold text-blue-600 dark:text-blue-400">{value}</div>
    </div>
  );
}

function round2(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.round(value * 100) / 100;
}

function numOr(value: string, fallback: number): number {
  const n = Number(value.replace(/,/g, ""));
  return Number.isFinite(n) ? n : fallback;
}

function clampHorizon(n: number): number {
  if (!Number.isFinite(n) || n <= 0) return 1;
  return Math.min(100, Math.max(1, Math.round(n)));
}

// Fisher: real = (1 + nominal) / (1 + inflation) - 1.
function realReturn(nominalPct: string, inflationPct: string): number {
  const nominal = numOr(nominalPct, 0) / 100;
  const inflation = numOr(inflationPct, 0) / 100;
  if (inflation <= -1) return nominal;
  return (1 + nominal) / (1 + inflation) - 1;
}

// Solves for n in: fv = pv·(1+r)^n + sav·((1+r)^n - 1)/r
// Returns Infinity if savings + growth can't reach the goal.
function yearsToReach(pv: number, fv: number, sav: number, r: number): number {
  if (pv >= fv) return 0;
  if (!Number.isFinite(fv)) return Number.POSITIVE_INFINITY;
  if (r === 0) {
    if (sav <= 0) return Number.POSITIVE_INFINITY;
    return (fv - pv) / sav;
  }
  const num = fv * r + sav;
  const den = pv * r + sav;
  if (den <= 0 || num <= 0) return Number.POSITIVE_INFINITY;
  const ratio = num / den;
  if (ratio <= 1) return Number.POSITIVE_INFINITY;
  return Math.log(ratio) / Math.log(1 + r);
}

function formatYears(years: number): string {
  if (!Number.isFinite(years)) return "Never (at this savings rate)";
  if (years <= 0) return "Already there";
  if (years < 1) {
    const months = Math.max(1, Math.round(years * 12));
    return `${months} mo`;
  }
  return `${years.toFixed(1)} yrs`;
}

// ── Monte Carlo simulation ──────────────────────────────────────────────────

type McPercentile = {
  year: number;
  p1: number;
  p2: number;
  p3: number;
  p4: number;
  p5: number;
  p10: number;
  p25: number;
  p50: number;
};

type MonteCarloResult = {
  successRate: number;
  medianEnd: number;
  p10End: number;
  p5End: number;
  p4End: number;
  p3End: number;
  p2End: number;
  p1End: number;
  medianRetirePortfolio: number;
  effectiveWithdrawalRate: number;
  percentiles: McPercentile[];
};

function mulberry32(seed: number): () => number {
  return () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function normalRandom(rng: () => number): number {
  const u1 = rng();
  const u2 = rng();
  return Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
}

function pctValue(sorted: number[], p: number): number {
  if (sorted.length === 0) return 0;
  const idx = p * (sorted.length - 1);
  const lo = Math.floor(idx);
  const hi = Math.ceil(idx);
  const loVal = sorted[lo] ?? 0;
  if (lo === hi) return loVal;
  return loVal + ((sorted[hi] ?? 0) - loVal) * (idx - lo);
}

function monteCarloSimulate({
  startPortfolio,
  accumulationYears,
  annualContribution,
  coastYears,
  annualWithdrawal,
  realReturn: annualReturn,
  realStdDev,
  horizonYears,
  numSims,
}: {
  startPortfolio: number;
  accumulationYears: number;
  annualContribution: number;
  coastYears: number;
  annualWithdrawal: number;
  realReturn: number;
  realStdDev: number;
  horizonYears: number;
  numSims: number;
}): MonteCarloResult | null {
  if (startPortfolio <= 0 || annualWithdrawal <= 0 || horizonYears <= 0 || numSims <= 0) {
    return null;
  }

  const accumEnd = Math.max(0, Math.min(horizonYears, accumulationYears));
  const coastEnd = Math.min(horizonYears, accumEnd + Math.max(0, coastYears));

  const rng = mulberry32(42);
  const allPaths: number[][] = [];
  let successes = 0;

  for (let s = 0; s < numSims; s++) {
    const path = [startPortfolio];
    let portfolio = startPortfolio;
    let survived = true;

    for (let y = 1; y <= horizonYears; y++) {
      const ret = annualReturn + realStdDev * normalRandom(rng);
      if (y <= accumEnd) {
        portfolio = portfolio * (1 + ret) + annualContribution;
        if (portfolio < 0) portfolio = 0;
      } else if (y <= coastEnd) {
        portfolio = portfolio * (1 + ret);
        if (portfolio < 0) portfolio = 0;
      } else {
        portfolio = portfolio * (1 + ret) - annualWithdrawal;
        if (portfolio <= 0) {
          portfolio = 0;
          survived = false;
        }
      }
      path.push(portfolio);
    }

    if (survived) successes++;
    allPaths.push(path);
  }

  const percentiles: McPercentile[] = [];
  for (let y = 0; y <= horizonYears; y++) {
    const vals = allPaths.map((p) => p[y] as number).sort((a, b) => a - b);
    percentiles.push({
      year: y,
      p1: pctValue(vals, 0.01),
      p2: pctValue(vals, 0.02),
      p3: pctValue(vals, 0.03),
      p4: pctValue(vals, 0.04),
      p5: pctValue(vals, 0.05),
      p10: pctValue(vals, 0.1),
      p25: pctValue(vals, 0.25),
      p50: pctValue(vals, 0.5),
    });
  }

  const endVals = allPaths.map((p) => p[horizonYears] as number).sort((a, b) => a - b);

  const retireVals = allPaths.map((p) => p[coastEnd] as number).sort((a, b) => a - b);
  const medianRetirePortfolio = pctValue(retireVals, 0.5);
  const effectiveWithdrawalRate =
    medianRetirePortfolio > 0 ? annualWithdrawal / medianRetirePortfolio : 0;

  return {
    successRate: successes / numSims,
    medianEnd: pctValue(endVals, 0.5),
    p10End: pctValue(endVals, 0.1),
    p5End: pctValue(endVals, 0.05),
    p4End: pctValue(endVals, 0.04),
    p3End: pctValue(endVals, 0.03),
    p2End: pctValue(endVals, 0.02),
    p1End: pctValue(endVals, 0.01),
    medianRetirePortfolio,
    effectiveWithdrawalRate,
    percentiles,
  };
}

function MonteCarloCard({
  result,
  stdDev,
  setStdDev,
  numSims,
  setNumSims,
  startPortfolio,
  accumulationYears,
  coastYears,
  horizonYears,
}: {
  result: MonteCarloResult | null;
  stdDev: string;
  setStdDev: (v: string) => void;
  numSims: string;
  setNumSims: (v: string) => void;
  startPortfolio: number;
  accumulationYears: number;
  coastYears: number;
  horizonYears: number;
}): React.JSX.Element {
  const successColor =
    result && result.successRate >= 0.9
      ? "text-emerald-600 dark:text-emerald-400"
      : result && result.successRate >= 0.7
        ? "text-amber-600 dark:text-amber-400"
        : "text-red-600 dark:text-red-400";
  const effectiveAccum = Math.max(0, Math.min(horizonYears, accumulationYears));
  const effectiveCoast = Math.max(0, Math.min(horizonYears - effectiveAccum, coastYears));
  const withdrawYears = Math.max(0, horizonYears - effectiveAccum - effectiveCoast);

  return (
    <Card>
      <CalculatorHeader
        title="Monte Carlo Simulation"
        subtitle="Estimates the probability your portfolio survives retirement withdrawals by running thousands of simulations with random annual returns. Starts from your current portfolio, contributes annual savings until the coast-start age, then coasts (no contributions, no withdrawals) until the retirement age, then withdraws your annual expenses each year. The withdrawal rate set in the shared inputs above is not used here — instead, the effective withdrawal rate (annual expenses ÷ median portfolio at retirement) is reported below. Based on the Trinity Study (Cooley, Hubbard & Walz, 1998) which established the 4% safe withdrawal rate using historical US market data (1926–1997), and Bengen (1994) who first proposed the rule."
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Result label="Starting portfolio (today)" value={formatCAD(startPortfolio)} />
        <Result
          label="Accumulate / coast / withdraw years"
          value={`${effectiveAccum} / ${effectiveCoast} / ${withdrawYears} yr`}
        />
        <NumberField
          label="Real return std dev (%)"
          value={stdDev}
          onChange={setStdDev}
          step="0.1"
        />
        <NumberField label="Number of simulations" value={numSims} onChange={setNumSims} />
      </div>
      {result ? (
        <>
          <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            <div className="rounded border border-gray-200 bg-gray-50 px-3 py-2 dark:border-gray-700 dark:bg-gray-900/40">
              <div className="text-xs text-gray-500 dark:text-gray-400">Success rate</div>
              <div className={`text-lg font-semibold ${successColor}`}>
                {(result.successRate * 100).toFixed(1)}%
              </div>
            </div>
            <Result
              label="Effective withdrawal rate"
              value={`${(result.effectiveWithdrawalRate * 100).toFixed(2)}%`}
            />
            <Result
              label="Median portfolio at retirement"
              value={formatCAD(result.medianRetirePortfolio)}
            />
            <Result label="Median ending portfolio" value={formatCAD(result.medianEnd)} />
            <Result label="10th percentile ending" value={formatCAD(result.p10End)} />
            <Result label="5th percentile ending" value={formatCAD(result.p5End)} />
            <Result label="4th percentile ending" value={formatCAD(result.p4End)} />
            <Result label="3rd percentile ending" value={formatCAD(result.p3End)} />
            <Result label="2nd percentile ending" value={formatCAD(result.p2End)} />
            <Result label="1st percentile ending" value={formatCAD(result.p1End)} />
          </div>
          <p className="mt-3 text-xs text-gray-500 dark:text-gray-400">
            Success rate = percentage of simulations where the portfolio survived the full horizon.
            Std dev of ~15–20% approximates historical equity volatility; ~10–12% for a balanced
            portfolio. All values are in today&apos;s dollars.
          </p>
          <MonteCarloChart percentiles={result.percentiles} />
        </>
      ) : (
        <p className="mt-3 text-sm text-amber-700 dark:text-amber-300">
          Set a current portfolio (&gt;$0) and annual expenses (&gt;$0) to run the simulation.
        </p>
      )}
    </Card>
  );
}

function MonteCarloChart({ percentiles }: { percentiles: McPercentile[] }): React.JSX.Element {
  if (percentiles.length === 0) return <></>;

  const data = percentiles.map((p) => ({
    year: p.year,
    p1: Math.round(p.p1),
    p2: Math.round(p.p2),
    p3: Math.round(p.p3),
    p4: Math.round(p.p4),
    p5: Math.round(p.p5),
    p10: Math.round(p.p10),
    p25: Math.round(p.p25),
    p50: Math.round(p.p50),
  }));

  return (
    <div className="mt-2">
      <ResponsiveContainer width="100%" height={400}>
        <LineChart data={data} margin={{ top: 8, right: 16, left: 0, bottom: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#e5e7eb" />
          <XAxis
            dataKey="year"
            tick={{ fontSize: 12 }}
            label={{
              value: "Years in retirement",
              position: "insideBottom",
              offset: -4,
              fontSize: 12,
            }}
          />
          <YAxis tick={{ fontSize: 12 }} tickFormatter={(v) => formatCADCompact(v)} width={60} />
          <Tooltip
            formatter={(v: number, name: string) => [formatCAD(v), name]}
            labelFormatter={(l) => `Year ${l}`}
          />
          <Legend />
          <Line
            type="monotone"
            dataKey="p50"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={false}
            name="Median"
          />
          <Line
            type="monotone"
            dataKey="p25"
            stroke="#60a5fa"
            strokeWidth={1}
            dot={false}
            name="25th percentile"
          />
          <Line
            type="monotone"
            dataKey="p10"
            stroke="#93c5fd"
            strokeWidth={1}
            dot={false}
            name="10th percentile"
          />
          <Line
            type="monotone"
            dataKey="p5"
            stroke="#fecaca"
            strokeWidth={1}
            dot={false}
            name="5th percentile"
          />
          <Line
            type="monotone"
            dataKey="p4"
            stroke="#fca5a5"
            strokeWidth={1}
            dot={false}
            name="4th percentile"
          />
          <Line
            type="monotone"
            dataKey="p3"
            stroke="#f87171"
            strokeWidth={1}
            dot={false}
            name="3rd percentile"
          />
          <Line
            type="monotone"
            dataKey="p2"
            stroke="#ef4444"
            strokeWidth={1}
            dot={false}
            name="2nd percentile"
          />
          <Line
            type="monotone"
            dataKey="p1"
            stroke="#b91c1c"
            strokeWidth={1}
            dot={false}
            name="1st percentile"
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
