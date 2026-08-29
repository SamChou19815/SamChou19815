"use client";

import { useEffect, useMemo, useState, type ReactNode } from "react";
import { getSupabase } from "../../lib/supabase";
import { useAuth } from "../../lib/useAuth";
import { parseLocalDate } from "../budget/utils";
import {
  computeCitizenshipProgress,
  formatLocalDate,
  PRE_PR_CREDIT_CAP,
  projectEligibility,
  START_DATE,
  TARGET_DAYS,
  WINDOW_YEARS,
  type CitizenshipProgress,
} from "./citizenship";

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
    const date = parseLocalDate(dateStr);
    return date >= START_DATE && date <= today;
  });
  const missingDaysCount = missingDaysInRange.length;
  return { totalDays, daysInCanada: totalDays - missingDaysCount, missingDaysCount };
}

// Deliberately smaller than the hero's headline number: these are the supporting
// counts, and matching the hero's scale would flatten the page's hierarchy.
function StatBox({ label, value }: { label: string; value: ReactNode }): React.JSX.Element {
  return (
    <div>
      <div className="text-3xl font-bold text-blue-500 dark:text-blue-400">{value}</div>
      <div className="text-sm text-gray-500 mt-1 dark:text-gray-400">{label}</div>
    </div>
  );
}

function Legend({ swatch, label }: { swatch: string; label: string }): React.JSX.Element {
  return (
    <div className="flex items-center gap-1">
      <div className={`w-3 h-3 rounded ${swatch}`} />
      <span className="text-gray-500 dark:text-gray-400">{label}</span>
    </div>
  );
}

function formatLongDate(date: Date): string {
  return date.toLocaleDateString("en-US", { year: "numeric", month: "long", day: "numeric" });
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

function MonthCalendar({
  year,
  month,
  today,
  missingDaysSet,
  prDateStr,
}: {
  year: number;
  month: number;
  today: Date;
  missingDaysSet: ReadonlySet<string>;
  prDateStr: string;
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
    const date = new Date(year, month, day);
    const dateStr = formatLocalDate(date);
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
    // The day permanent residency began: from here on days count in full.
    if (dateStr === prDateStr) {
      className += " ring-2 ring-blue-500";
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
  prDateStr,
}: {
  today: Date;
  missingDaysSet: ReadonlySet<string>;
  prDateStr: string;
}): React.JSX.Element {
  const months = getMonthsInRange(START_DATE, today);

  return (
    <div className="border-t pt-4 mt-4 dark:border-t-gray-600">
      <h4 className="text-gray-700 mb-4 dark:text-gray-300">Calendar View</h4>
      <div className="flex flex-wrap items-center gap-4 mb-4 text-xs">
        <Legend swatch="bg-green-400" label="In Canada" />
        <Legend swatch="bg-red-400" label="Outside Canada" />
        {prDateStr !== "" && <Legend swatch="ring-2 ring-blue-500" label="Became a PR" />}
      </div>
      <div className="flex flex-wrap gap-4 justify-center">
        {months.map(({ year, month }) => (
          <MonthCalendar
            key={`${year}-${month}`}
            year={year}
            month={month}
            today={today}
            missingDaysSet={missingDaysSet}
            prDateStr={prDateStr}
          />
        ))}
      </div>
    </div>
  );
}

/**
 * A stacked bar toward {@link TARGET_DAYS}: pre-PR credit (half rate) in blue,
 * days as a PR (full rate) in green. Keeping the two rates as separate segments
 * is the point — one flat bar would hide which part of the progress is capped.
 *
 * Spans the full screen width as the bottom edge of the hero, so it reads as
 * the base of the block that states its own numbers rather than as a stray
 * strip: a caption on the page's narrower measure could never line up with the
 * ends of a bar this wide.
 */
function ProgressBar({ progress }: { progress: CitizenshipProgress }): React.JSX.Element {
  const prePrPct = Math.min(100, (progress.prePrCredit / TARGET_DAYS) * 100);
  const prPct = Math.min(100 - prePrPct, (progress.prDays / TARGET_DAYS) * 100);
  return (
    <div
      className="flex h-3 w-full overflow-hidden bg-white/10"
      role="progressbar"
      aria-label="Progress to citizenship"
      aria-valuemin={0}
      aria-valuemax={TARGET_DAYS}
      aria-valuenow={progress.total}
    >
      <div className="h-full bg-blue-400" style={{ width: `${prePrPct}%` }} />
      <div className="h-full bg-emerald-400" style={{ width: `${prPct}%` }} />
    </div>
  );
}

/**
 * The one fact the page exists to answer, stated once and large. The stats that
 * used to sit in a card below duplicated every number here, so that card is
 * gone — only the rule it explained survives, as the lede beneath.
 */
function Hero({
  progress,
  eligibleOn,
  today,
  hasPrDate,
}: {
  progress: CitizenshipProgress | null;
  eligibleOn: Date | null;
  today: Date | null;
  hasPrDate: boolean;
}): React.JSX.Element {
  const alreadyEligible = eligibleOn != null && today != null && eligibleOn <= today;
  return (
    <section className="bg-slate-900 text-white">
      <div className="mx-auto max-w-6xl px-6 pt-12 pb-10">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">
          In-Canada Days Counter
        </p>

        <div className="mt-8 flex flex-wrap items-end justify-between gap-x-12 gap-y-6">
          <div>
            <div className="flex items-baseline gap-4">
              <span className="text-7xl font-bold leading-none tracking-tight">
                {progress != null ? `${Math.round(progress.percent)}%` : "—"}
              </span>
              <span className="text-lg text-slate-400">toward citizenship</span>
            </div>
            <p className="mt-4 text-slate-300">
              {progress != null
                ? `${progress.total.toLocaleString()} of ${TARGET_DAYS.toLocaleString()} days credited · ${progress.remaining.toLocaleString()} to go`
                : `${TARGET_DAYS.toLocaleString()} days needed`}
            </p>
          </div>

          <div className="sm:text-right">
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">
              {alreadyEligible ? "Status" : "Projected eligibility"}
            </p>
            <p className="mt-2 text-3xl font-semibold">
              {alreadyEligible ? "Eligible now" : (eligibleOn?.toLocaleDateString("en-CA") ?? "—")}
            </p>
            <p className="mt-2 text-sm text-slate-400">
              Counting since {formatLongDate(START_DATE)}
            </p>
          </div>
        </div>

        {progress != null && (
          <div className="mt-8 flex flex-wrap items-center gap-x-6 gap-y-2 text-xs text-slate-400">
            <span className="flex items-center gap-2">
              <span className="h-3 w-3 rounded bg-blue-400" />
              {`Pre-PR credit — ${progress.prePrDays.toLocaleString()} days at ½, capped at ${PRE_PR_CREDIT_CAP}`}
            </span>
            {hasPrDate && (
              <span className="flex items-center gap-2">
                <span className="h-3 w-3 rounded bg-emerald-400" />
                {`Days as a PR — ${progress.prDays.toLocaleString()} at full rate`}
              </span>
            )}
          </div>
        )}
      </div>

      {progress != null && <ProgressBar progress={progress} />}
    </section>
  );
}

type LoadState = "loading" | "ready" | "error";

export default function InCanadaApp(): React.JSX.Element {
  const { session } = useAuth();
  const userId = session?.user.id ?? null;

  const [today, setToday] = useState<Date | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("loading");
  // The persisted values, and the (possibly edited) form values.
  const [savedText, setSavedText] = useState("");
  const [draftText, setDraftText] = useState("");
  // "" means no PR date recorded, matching an empty <input type="date">.
  const [savedPrDate, setSavedPrDate] = useState("");
  const [draftPrDate, setDraftPrDate] = useState("");
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
      // Every column rather than a named list: PostgREST rejects a select that
      // names a column the table doesn't have, and `pr_date` only exists once
      // the migration has been applied. It's a single row either way.
      .select("*")
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
        const prDate = data?.pr_date ?? "";
        setSavedPrDate(prDate);
        setDraftPrDate(prDate);
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

  // Stats reflect the saved values, not in-progress edits.
  const missingDays = useMemo(() => parseMissingDays(savedText), [savedText]);
  const missingDaysSet = useMemo(() => new Set(missingDays), [missingDays]);
  const prDate = useMemo(
    () => (savedPrDate === "" ? null : parseLocalDate(savedPrDate)),
    [savedPrDate],
  );
  const progress = useMemo(
    () => (today == null ? null : computeCitizenshipProgress(today, prDate, missingDaysSet)),
    [today, prDate, missingDaysSet],
  );
  const eligibleOn = useMemo(
    () => (today == null ? null : projectEligibility(today, prDate, missingDaysSet)),
    [today, prDate, missingDaysSet],
  );

  const stats = today != null ? countDaysInCanada(today, missingDays) : null;

  const dirty = draftText !== savedText || draftPrDate !== savedPrDate;

  const save = async () => {
    if (userId == null) return;
    setSaving(true);
    setMessage(null);
    const normalized = parseMissingDays(draftText).join("\n");
    const { error } = await getSupabase()
      .from("in_canada")
      .upsert(
        {
          user_id: userId,
          missing_days: normalized,
          pr_date: draftPrDate === "" ? null : draftPrDate,
          updated_at: new Date().toISOString(),
        },
        { onConflict: "user_id" },
      );
    setSaving(false);
    if (error != null) {
      setMessage({ kind: "error", text: error.message });
      return;
    }
    setSavedText(normalized);
    setDraftText(normalized);
    setSavedPrDate(draftPrDate);
    setMessage({ kind: "ok", text: "Saved." });
  };

  return (
    // No max-width on the root: the hero and its progress bar span the whole
    // screen, so the page's measure is applied by the container below instead.
    <div>
      <Hero
        progress={progress}
        eligibleOn={eligibleOn}
        today={today}
        hasPrDate={savedPrDate !== ""}
      />

      <div className="mx-auto w-full max-w-6xl px-6 py-10 flex flex-col gap-6">
        <p className="max-w-3xl text-sm text-gray-500 dark:text-gray-400">
          {TARGET_DAYS.toLocaleString()} days of physical presence in the last {WINDOW_YEARS} years.
          Days before permanent residency count as half a day each, up to {PRE_PR_CREDIT_CAP} days
          of credit; days from the PR date onward count in full.{" "}
          {savedPrDate !== ""
            ? "The projected date assumes you stay in Canada every day from now on."
            : `Without a PR date every tracked day counts as half a day, so the total is capped at ${PRE_PR_CREDIT_CAP} — set the date below to count full days.`}
        </p>

        <Card>
          <div className="flex flex-wrap gap-x-16 gap-y-6">
            <StatBox label="Days in Canada" value={stats?.daysInCanada ?? "—"} />
            <StatBox label="Days away" value={stats?.missingDaysCount ?? "—"} />
            <StatBox label="Total days" value={stats?.totalDays ?? "—"} />
          </div>

          {today != null && (
            <Calendar today={today} missingDaysSet={missingDaysSet} prDateStr={savedPrDate} />
          )}
        </Card>

        <Card>
          <h3 className="mb-2">Permanent resident since</h3>
          <p className="text-sm text-gray-500 mb-4 dark:text-gray-400">
            The date you became a permanent resident. Days before it count as half a day toward
            citizenship; days from it onward count in full.
          </p>
          <div className="flex items-center gap-3">
            <input
              type="date"
              value={draftPrDate}
              onChange={(e) => setDraftPrDate(e.target.value)}
              disabled={loadState === "loading"}
              className="rounded border border-gray-300 bg-white px-2 py-1 text-sm disabled:opacity-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
            />
            {draftPrDate !== "" && (
              <button
                type="button"
                onClick={() => setDraftPrDate("")}
                className="text-sm text-gray-500 underline hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
              >
                Clear
              </button>
            )}
          </div>

          <h3 className="mt-8 mb-2">Days outside Canada</h3>
          <p className="text-sm text-gray-500 mb-4 dark:text-gray-400">
            One date per line in <code>YYYY-MM-DD</code> format. These days are subtracted from the
            counter.
          </p>
          <textarea
            value={draftText}
            onChange={(e) => setDraftText(e.target.value)}
            disabled={loadState === "loading"}
            rows={10}
            spellCheck={false}
            placeholder={"2025-03-14\n2025-03-15\n2025-07-02"}
            className="w-full rounded border border-gray-300 bg-white px-3 py-2 font-mono text-sm focus:border-blue-500 focus:outline-none disabled:opacity-50 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
          />

          {message != null && (
            <div
              className={`mt-4 rounded border px-3 py-2 text-sm ${
                message.kind === "ok"
                  ? "border-blue-400 bg-blue-50 text-blue-700 dark:bg-blue-900/20 dark:text-blue-300"
                  : "border-red-400 bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-300"
              }`}
            >
              {message.text}
            </div>
          )}
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
    </div>
  );
}
