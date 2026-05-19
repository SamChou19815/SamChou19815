"use client";

import { useEffect, useMemo, useState, type ReactNode } from "react";
import { getSupabase } from "../../lib/supabase";
import { useAuth } from "../../lib/useAuth";

const START_DATE = new Date("2025-01-24");

export function Card({ children }: { children: ReactNode }): React.JSX.Element {
  return (
    <div className="flex flex-col bg-white rounded filter drop-shadow hover:drop-shadow-lg transition-all duration-300 ease-out p-6 dark:bg-[#242424] dark:drop-shadow-[0_1px_3px_rgba(0,0,0,0.3)] dark:hover:drop-shadow-[0_4px_12px_rgba(0,0,0,0.4)]">
      {children}
    </div>
  );
}

function parseMissingDays(text: string): string[] {
  return text
    .split("\n")
    .map((d) => d.trim())
    .filter((d) => d !== "");
}

function getDaysBetween(start: Date, end: Date): number {
  const msPerDay = 24 * 60 * 60 * 1000;
  return Math.floor((end.getTime() - start.getTime()) / msPerDay) + 1;
}

function countDaysInCanada(
  today: Date,
  missingDays: ReadonlyArray<string>,
): { totalDays: number; daysInCanada: number; missingDaysCount: number } {
  const totalDays = getDaysBetween(START_DATE, today);
  const missingDaysInRange = missingDays.filter((dateStr) => {
    const date = new Date(dateStr);
    return date >= START_DATE && date <= today;
  });
  const missingDaysCount = missingDaysInRange.length;
  return { totalDays, daysInCanada: totalDays - missingDaysCount, missingDaysCount };
}

function StatBox({ label, value }: { label: string; value: number | string }): React.JSX.Element {
  return (
    <div className="text-center">
      <div className="text-4xl font-bold text-blue-500 dark:text-blue-400">{value}</div>
      <div className="text-gray-500 mt-1 dark:text-gray-400">{label}</div>
    </div>
  );
}

function getMonthsInRange(start: Date, end: Date): { year: number; month: number }[] {
  const months: { year: number; month: number }[] = [];
  const current = new Date(start.getFullYear(), start.getMonth(), 1);
  const endMonth = new Date(end.getFullYear(), end.getMonth(), 1);

  // oxlint-disable-next-line no-unmodified-loop-condition
  while (current <= endMonth) {
    months.push({ year: current.getFullYear(), month: current.getMonth() });
    current.setMonth(current.getMonth() + 1);
  }
  return months;
}

function getDaysInMonth(year: number, month: number): number {
  return new Date(year, month + 1, 0).getDate();
}

function getFirstDayOfMonth(year: number, month: number): number {
  return new Date(year, month, 1).getDay();
}

function formatDateString(year: number, month: number, day: number): string {
  return `${year}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`;
}

function MonthCalendar({
  year,
  month,
  today,
  missingDaysSet,
}: {
  year: number;
  month: number;
  today: Date;
  missingDaysSet: ReadonlySet<string>;
}): React.JSX.Element {
  const daysInMonth = getDaysInMonth(year, month);
  const firstDay = getFirstDayOfMonth(year, month);
  const monthName = new Date(year, month).toLocaleDateString("en-US", { month: "short" });

  const days: React.JSX.Element[] = [];

  // Empty cells for days before the first day of the month
  for (let i = 0; i < firstDay; i++) {
    days.push(<div key={`empty-${i}`} className="w-5 h-5" />);
  }

  // Days of the month
  for (let day = 1; day <= daysInMonth; day++) {
    const dateStr = formatDateString(year, month, day);
    const date = new Date(year, month, day);
    const isMissing = missingDaysSet.has(dateStr);
    const isOutOfRange = date < START_DATE || date > today;

    let className = "w-5 h-5 text-xs flex items-center justify-center rounded ";
    if (isOutOfRange) {
      className += "text-gray-300 dark:text-gray-600";
    } else if (isMissing) {
      className += "bg-red-400 text-white font-medium";
    } else {
      className += "bg-green-400 text-white";
    }

    days.push(
      <div key={day} className={className}>
        {day}
      </div>,
    );
  }

  return (
    <div className="flex flex-col items-center">
      <div className="text-xs font-medium text-gray-600 mb-1 dark:text-gray-400">
        {monthName} {year}
      </div>
      <div className="grid grid-cols-7 gap-0.5">{days}</div>
    </div>
  );
}

function Calendar({
  today,
  missingDaysSet,
}: {
  today: Date;
  missingDaysSet: ReadonlySet<string>;
}): React.JSX.Element {
  const months = getMonthsInRange(START_DATE, today);

  return (
    <div className="border-t pt-4 mt-4 dark:border-t-gray-600">
      <h4 className="text-gray-700 mb-4 dark:text-gray-300">Calendar View</h4>
      <div className="flex items-center gap-4 mb-4 text-xs">
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-green-400" />
          <span className="text-gray-500 dark:text-gray-400">In Canada</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded bg-red-400" />
          <span className="text-gray-500 dark:text-gray-400">Outside Canada</span>
        </div>
      </div>
      <div className="flex flex-wrap gap-4 justify-center">
        {months.map(({ year, month }) => (
          <MonthCalendar
            key={`${year}-${month}`}
            year={year}
            month={month}
            today={today}
            missingDaysSet={missingDaysSet}
          />
        ))}
      </div>
    </div>
  );
}

type LoadState = "loading" | "ready" | "error";

export default function InCanadaApp(): React.JSX.Element {
  const { session } = useAuth();
  const userId = session?.user.id ?? null;

  const [today, setToday] = useState<Date | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  // The persisted value, and the (possibly edited) textarea value.
  const [savedText, setSavedText] = useState("");
  const [draftText, setDraftText] = useState("");
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<{ kind: "ok" | "error"; text: string } | null>(null);

  useEffect(() => {
    setToday(new Date());
  }, []);

  useEffect(() => {
    if (userId == null) return;
    let cancelled = false;
    setLoadState("loading");
    getSupabase()
      .from("in_canada")
      .select("missing_days")
      .eq("user_id", userId)
      .maybeSingle()
      .then(({ data, error }) => {
        if (cancelled) return;
        if (error != null) {
          setLoadState("error");
          setMessage({ kind: "error", text: error.message });
          return;
        }
        const text = data?.missing_days ?? "";
        setSavedText(text);
        setDraftText(text);
        setLoadState("ready");
      });
    return () => {
      cancelled = true;
    };
  }, [userId]);

  useEffect(() => {
    if (message?.kind !== "ok") return;
    const t = setTimeout(() => setMessage(null), 4000);
    return () => clearTimeout(t);
  }, [message]);

  // Stats reflect the saved value, not in-progress edits.
  const missingDays = useMemo(() => parseMissingDays(savedText), [savedText]);
  const missingDaysSet = useMemo(() => new Set(missingDays), [missingDays]);

  const stats = today != null ? countDaysInCanada(today, missingDays) : null;
  const startDateFormatted = START_DATE.toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });

  const dirty = draftText !== savedText;

  const save = async () => {
    if (userId == null) return;
    setSaving(true);
    setMessage(null);
    const normalized = parseMissingDays(draftText).join("\n");
    const { error } = await getSupabase()
      .from("in_canada")
      .upsert(
        { user_id: userId, missing_days: normalized, updated_at: new Date().toISOString() },
        { onConflict: "user_id" },
      );
    setSaving(false);
    if (error != null) {
      setMessage({ kind: "error", text: error.message });
      return;
    }
    setSavedText(normalized);
    setDraftText(normalized);
    setMessage({ kind: "ok", text: "Saved." });
  };

  return (
    <div className="mx-auto max-w-4xl px-4 py-12 flex flex-col gap-6">
      <Card>
        <h1 className="text-center mb-2">In-Canada Days Counter</h1>
        <p className="text-center text-gray-500 mb-8 dark:text-gray-400">
          Counting since {startDateFormatted}
        </p>

        <div className="flex flex-row justify-around mb-8">
          <StatBox label="Days in Canada" value={stats?.daysInCanada ?? "—"} />
          <StatBox label="Days Away" value={stats?.missingDaysCount ?? "—"} />
          <StatBox label="Total Days" value={stats?.totalDays ?? "—"} />
        </div>

        {today != null && <Calendar today={today} missingDaysSet={missingDaysSet} />}
      </Card>

      <Card>
        <h3 className="mb-2">Days outside Canada</h3>
        <p className="text-sm text-gray-500 mb-4 dark:text-gray-400">
          One date per line in <code>YYYY-MM-DD</code> format. These days are subtracted from the
          counter.
        </p>
        {message != null && (
          <div
            className={`mb-3 rounded border px-3 py-2 text-sm ${
              message.kind === "ok"
                ? "border-blue-400 bg-blue-50 text-blue-700 dark:bg-blue-900/20 dark:text-blue-300"
                : "border-red-400 bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-300"
            }`}
          >
            {message.text}
          </div>
        )}
        <textarea
          value={draftText}
          onChange={(e) => setDraftText(e.target.value)}
          disabled={loadState === "loading"}
          rows={10}
          spellCheck={false}
          placeholder={"2025-03-14\n2025-03-15\n2025-07-02"}
          className="w-full rounded border border-gray-300 bg-white px-3 py-2 font-mono text-sm focus:border-blue-500 focus:outline-none disabled:opacity-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
        />
        <div className="mt-4 flex items-center justify-end gap-3">
          {dirty && loadState === "ready" && (
            <span className="text-sm text-gray-500 dark:text-gray-400">Unsaved changes</span>
          )}
          <button
            type="button"
            onClick={save}
            disabled={saving || loadState !== "ready" || !dirty}
            className="rounded bg-blue-500 px-4 py-2 text-sm font-bold text-white transition-colors duration-200 hover:bg-blue-600 disabled:opacity-50 dark:bg-blue-400 dark:text-gray-900 dark:hover:bg-blue-300"
          >
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </Card>
    </div>
  );
}
