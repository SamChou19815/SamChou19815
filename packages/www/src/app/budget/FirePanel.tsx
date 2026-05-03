"use client";

import { useMemo, useState } from "react";
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
import { cadValue, formatCAD, parseLocalDate } from "./utils";

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

type SharedInputs = {
  annualExpenses: string;
  withdrawalRate: string;
  portfolio: string;
  annualSavings: string;
  expectedReturn: string;
  inflation: string;
  horizon: string;
};

type SharedSetters = {
  setAnnualExpenses: (v: string) => void;
  setWithdrawalRate: (v: string) => void;
  setPortfolio: (v: string) => void;
  setAnnualSavings: (v: string) => void;
  setExpectedReturn: (v: string) => void;
  setInflation: (v: string) => void;
  setHorizon: (v: string) => void;
};

export default function FirePanel({
  expenses,
  incomes,
  investments,
  loading,
}: Props): React.JSX.Element {
  const year = lastCalendarYear();

  const defaults = useMemo(() => {
    const annualExpenses = sumForYear(expenses, year);
    const annualIncome = sumForYear(incomes, year);
    const annualSavings = Math.max(0, annualIncome - annualExpenses);
    const portfolio = investments.reduce((s, inv) => s + cadValue(inv), 0);
    return { annualExpenses, annualIncome, annualSavings, portfolio };
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

const CHUBBY_DEFAULT_SPEND = 100000;
const FAT_DEFAULT_SPEND = 200000;

type FlavorKey = "classic" | "conservative" | "chubby" | "fat" | "coast" | "barista";

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
  const [withdrawalRate, setWithdrawalRate] = useState("4");
  const [portfolio, setPortfolio] = useState(() => String(round2(defaults.portfolio)));
  const [annualSavings, setAnnualSavings] = useState(() => String(round2(defaults.annualSavings)));
  const [expectedReturn, setExpectedReturn] = useState("7");
  const [inflation, setInflation] = useState("2.5");
  const [horizon, setHorizon] = useState("50");
  const [chubbySpend, setChubbySpend] = useState(String(CHUBBY_DEFAULT_SPEND));
  const [fatSpend, setFatSpend] = useState(String(FAT_DEFAULT_SPEND));
  const [baristaIncome, setBaristaIncome] = useState("25000");
  const [conservativeWr, setConservativeWr] = useState("3");
  const [currentAge, setCurrentAge] = useState(String(new Date().getFullYear() - 1998));
  const [retireAge, setRetireAge] = useState("65");
  const [endAge, setEndAge] = useState("95");

  const hasLastYearData = defaults.annualExpenses > 0 || defaults.annualIncome > 0;
  const shared: SharedInputs = {
    annualExpenses,
    withdrawalRate,
    portfolio,
    annualSavings,
    expectedReturn,
    inflation,
    horizon,
  };
  const setters: SharedSetters = {
    setAnnualExpenses,
    setWithdrawalRate,
    setPortfolio,
    setAnnualSavings,
    setExpectedReturn,
    setInflation,
    setHorizon,
  };

  const exp = numOr(annualExpenses, 0);
  const wr = numOr(withdrawalRate, 0) / 100;
  const pv = numOr(portfolio, 0);
  const sav = numOr(annualSavings, 0);
  const r = realReturn(expectedReturn, inflation);
  const horizonYears = clampHorizon(numOr(horizon, 50));
  const chubbyExp = numOr(chubbySpend, 0);
  const fatExp = numOr(fatSpend, 0);
  const barista = numOr(baristaIncome, 0);
  const age = numOr(currentAge, 0);
  const retire = numOr(retireAge, 0);
  const end = numOr(endAge, 0);

  const fireNumber = wr > 0 ? exp / wr : Number.POSITIVE_INFINITY;
  const chubbyTarget = wr > 0 ? chubbyExp / wr : Number.POSITIVE_INFINITY;
  const fatTarget = wr > 0 ? fatExp / wr : Number.POSITIVE_INFINITY;
  const baristaShortfall = Math.max(0, exp - barista);
  const baristaTarget = wr > 0 ? baristaShortfall / wr : Number.POSITIVE_INFINITY;
  const conservativeSpend = exp > 0 ? Math.ceil(exp / 10000) * 10000 : 0;
  const conservativeWrValue = numOr(conservativeWr, 0) / 100;
  const conservativeTarget =
    conservativeWrValue > 0 ? conservativeSpend / conservativeWrValue : Number.POSITIVE_INFINITY;
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
  const chubbySim = useMemo(
    () =>
      simulate({
        pv,
        r,
        horizonYears,
        contribution: sav,
        withdrawal: chubbyExp,
        switchAt: (_y, p) => p >= chubbyTarget,
      }),
    [pv, r, horizonYears, sav, chubbyExp, chubbyTarget],
  );
  const fatSim = useMemo(
    () =>
      simulate({
        pv,
        r,
        horizonYears,
        contribution: sav,
        withdrawal: fatExp,
        switchAt: (_y, p) => p >= fatTarget,
      }),
    [pv, r, horizonYears, sav, fatExp, fatTarget],
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
  const baristaSim = useMemo(
    () =>
      simulate({
        pv,
        r,
        horizonYears,
        contribution: sav,
        withdrawal: baristaShortfall,
        switchAt: (_y, p) => p >= baristaTarget,
      }),
    [pv, r, horizonYears, sav, baristaShortfall, baristaTarget],
  );
  const conservativeSim = useMemo(
    () =>
      simulate({
        pv,
        r,
        horizonYears,
        contribution: sav,
        withdrawal: conservativeSpend,
        switchAt: (_y, p) => p >= conservativeTarget,
      }),
    [pv, r, horizonYears, sav, conservativeSpend, conservativeTarget],
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
      key: "conservative",
      label: "Conservative FIRE",
      color: "#06b6d4",
      target: conservativeTarget,
      sim: conservativeSim,
      switchYear: firstWithdrawYear(conservativeSim),
    },
    {
      key: "chubby",
      label: "Chubby FIRE",
      color: "#f59e0b",
      target: chubbyTarget,
      sim: chubbySim,
      switchYear: firstWithdrawYear(chubbySim),
    },
    {
      key: "fat",
      label: "Fat FIRE",
      color: "#ef4444",
      target: fatTarget,
      sim: fatSim,
      switchYear: firstWithdrawYear(fatSim),
    },
    {
      key: "coast",
      label: "Coast FIRE",
      color: "#8b5cf6",
      target: fireNumber,
      sim: coastSim,
      switchYear: yearsToRetire,
    },
    {
      key: "barista",
      label: "Barista FIRE",
      color: "#10b981",
      target: baristaTarget,
      sim: baristaSim,
      switchYear: firstWithdrawYear(baristaSim),
    },
  ];

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
      <ConservativeFire
        pv={pv}
        sav={sav}
        r={r}
        exp={exp}
        conservativeSpend={conservativeSpend}
        conservativeWr={conservativeWr}
        setConservativeWr={setConservativeWr}
        target={conservativeTarget}
      />
      <ChubbyFire
        pv={pv}
        sav={sav}
        r={r}
        spend={chubbySpend}
        setSpend={setChubbySpend}
        target={chubbyTarget}
        spendValue={chubbyExp}
      />
      <FatFire
        pv={pv}
        sav={sav}
        r={r}
        spend={fatSpend}
        setSpend={setFatSpend}
        target={fatTarget}
        spendValue={fatExp}
      />
      <CoastFire
        pv={pv}
        r={r}
        retire={retire}
        currentAge={currentAge}
        setCurrentAge={setCurrentAge}
        retireAge={retireAge}
        setRetireAge={setRetireAge}
        endAge={endAge}
        setEndAge={setEndAge}
        target={coastTarget}
        fireNumber={fireNumber}
      />
      <BaristaFire
        pv={pv}
        baristaIncome={baristaIncome}
        setBaristaIncome={setBaristaIncome}
        exp={exp}
        shortfall={baristaShortfall}
        target={baristaTarget}
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
          label={`Annual expenses (${year})`}
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
        <NumberField label="Horizon (yrs)" value={shared.horizon} onChange={setters.setHorizon} />
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

function ConservativeFire({
  pv,
  sav,
  r,
  exp,
  conservativeSpend,
  conservativeWr,
  setConservativeWr,
  target,
}: {
  pv: number;
  sav: number;
  r: number;
  exp: number;
  conservativeSpend: number;
  conservativeWr: string;
  setConservativeWr: (v: string) => void;
  target: number;
}): React.JSX.Element {
  const gap = Math.max(0, target - pv);
  const years = yearsToReach(pv, target, sav, r);
  return (
    <Card>
      <CalculatorHeader
        title="Conservative FIRE"
        subtitle={`A safer margin: ${conservativeWr}% withdrawal rate with spending rounded up to the next $10,000 (${formatCAD(conservativeSpend)}).`}
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <NumberField
          label="Conservative withdrawal rate (%)"
          value={conservativeWr}
          onChange={setConservativeWr}
          step="0.1"
        />
        <Result label="Rounded-up annual spend" value={formatCAD(conservativeSpend)} />
      </div>
      <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-3">
        <Result label="Conservative FIRE number" value={formatCAD(target)} />
        <Result label="Gap from today" value={formatCAD(gap)} />
        <Result label="Years to conservative FIRE" value={formatYears(years)} />
      </div>
      {exp <= 0 && (
        <p className="mt-3 text-sm text-amber-700 dark:text-amber-300">
          Set annual expenses above $0 to see meaningful results.
        </p>
      )}
    </Card>
  );
}

function ChubbyFire({
  pv,
  sav,
  r,
  spend,
  setSpend,
  target,
  spendValue,
}: {
  pv: number;
  sav: number;
  r: number;
  spend: string;
  setSpend: (v: string) => void;
  target: number;
  spendValue: number;
}): React.JSX.Element {
  const gap = Math.max(0, target - pv);
  const years = yearsToReach(pv, target, sav, r);
  return (
    <Card>
      <CalculatorHeader
        title="Chubby FIRE"
        subtitle={`A more comfortable retirement. Default ${formatCAD(CHUBBY_DEFAULT_SPEND)} is the mid-point of that band.`}
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <NumberField label="Annual spend" value={spend} onChange={setSpend} />
      </div>
      <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-3">
        <Result label="Chubby FIRE number" value={formatCAD(target)} />
        <Result label="Gap from today" value={formatCAD(gap)} />
        <Result label="Years to chubby FIRE" value={formatYears(years)} />
      </div>
      {spendValue <= 0 && (
        <p className="mt-3 text-sm text-amber-700 dark:text-amber-300">
          Set an annual spend above $0 to see meaningful results.
        </p>
      )}
    </Card>
  );
}

function FatFire({
  pv,
  sav,
  r,
  spend,
  setSpend,
  target,
  spendValue,
}: {
  pv: number;
  sav: number;
  r: number;
  spend: string;
  setSpend: (v: string) => void;
  target: number;
  spendValue: number;
}): React.JSX.Element {
  const gap = Math.max(0, target - pv);
  const years = yearsToReach(pv, target, sav, r);
  return (
    <Card>
      <CalculatorHeader
        title="Fat FIRE"
        subtitle={`Luxury retirement. Default ${formatCAD(FAT_DEFAULT_SPEND)} is the mid-point of that band.`}
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <NumberField label="Annual spend" value={spend} onChange={setSpend} />
      </div>
      <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-3">
        <Result label="Fat FIRE number" value={formatCAD(target)} />
        <Result label="Gap from today" value={formatCAD(gap)} />
        <Result label="Years to fat FIRE" value={formatYears(years)} />
      </div>
      {spendValue <= 0 && (
        <p className="mt-3 text-sm text-amber-700 dark:text-amber-300">
          Set an annual spend above $0 to see meaningful results.
        </p>
      )}
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
        subtitle="The lump sum you need invested today so it grows into your full FIRE number by retirement, even if you stop contributing."
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <NumberField label="Current age" value={currentAge} onChange={setCurrentAge} />
        <NumberField label="Retirement age" value={retireAge} onChange={setRetireAge} />
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

function BaristaFire({
  pv,
  baristaIncome,
  setBaristaIncome,
  exp,
  shortfall,
  target,
}: {
  pv: number;
  baristaIncome: string;
  setBaristaIncome: (v: string) => void;
  exp: number;
  shortfall: number;
  target: number;
}): React.JSX.Element {
  const gap = Math.max(0, target - pv);
  const covered = shortfall <= 0 && exp > 0;

  return (
    <Card>
      <CalculatorHeader
        title="Barista FIRE"
        subtitle="Portfolio big enough that part-time / 'barista' income covers the rest of your annual expenses."
      />
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <NumberField
          label="Part-time income / yr"
          value={baristaIncome}
          onChange={setBaristaIncome}
        />
      </div>
      <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-3">
        <Result label="Annual shortfall" value={formatCAD(shortfall)} />
        <Result
          label="Barista FIRE target"
          value={covered ? "$0 (income covers)" : formatCAD(target)}
        />
        <Result label="Gap from today" value={formatCAD(gap)} />
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
          <YAxis
            tick={{ fontSize: 12 }}
            tickFormatter={(v) => `$${Math.round(v / 1000)}k`}
            width={60}
          />
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
