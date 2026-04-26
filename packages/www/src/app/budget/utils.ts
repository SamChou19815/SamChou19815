import type { Investment, InvestmentSnapshot } from "./types";

const cadFormatter = new Intl.NumberFormat("en-CA", {
  style: "currency",
  currency: "CAD",
  maximumFractionDigits: 2,
});

export function formatCAD(value: number): string {
  if (!Number.isFinite(value)) return "—";
  return cadFormatter.format(value);
}

export function todayISO(): string {
  const d = new Date();
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}`;
}

export function nowLocalInputValue(): string {
  const d = new Date();
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const hh = String(d.getHours()).padStart(2, "0");
  const mi = String(d.getMinutes()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}T${hh}:${mi}`;
}

export function isoToLocalInputValue(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return nowLocalInputValue();
  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const hh = String(d.getHours()).padStart(2, "0");
  const mi = String(d.getMinutes()).padStart(2, "0");
  return `${yyyy}-${mm}-${dd}T${hh}:${mi}`;
}

export function localInputValueToISO(value: string): string {
  return new Date(value).toISOString();
}

// e.g. "2026-04" key for month bucketing, takes a date string.
export function monthBucket(date: string): string {
  return date.slice(0, 7);
}

export function monthsBetween(start: Date, end: Date): string[] {
  const out: string[] = [];
  const cur = new Date(start.getFullYear(), start.getMonth(), 1);
  const last = new Date(end.getFullYear(), end.getMonth(), 1);
  // oxlint-disable-next-line no-unmodified-loop-condition
  while (cur <= last) {
    const yyyy = cur.getFullYear();
    const mm = String(cur.getMonth() + 1).padStart(2, "0");
    out.push(`${yyyy}-${mm}`);
    cur.setMonth(cur.getMonth() + 1);
  }
  return out;
}

export function cadValue(
  investment: Pick<Investment, "total_market_value" | "exchange_rate">,
): number {
  return investment.total_market_value * investment.exchange_rate;
}

export function snapshotCadValue(
  snapshot: Pick<InvestmentSnapshot, "total_market_value" | "exchange_rate">,
): number {
  return snapshot.total_market_value * snapshot.exchange_rate;
}

// For each month-bucket "YYYY-MM", returns the latest snapshot whose recorded_at is <= last day of that month.
export function latestSnapshotByMonth(
  snapshots: ReadonlyArray<InvestmentSnapshot>,
  monthKeys: ReadonlyArray<string>,
): Map<string, InvestmentSnapshot | null> {
  const sorted = snapshots
    .slice()
    .sort((a, b) => new Date(a.recorded_at).getTime() - new Date(b.recorded_at).getTime());
  const out = new Map<string, InvestmentSnapshot | null>();
  for (const key of monthKeys) {
    const [yStr, mStr] = key.split("-");
    const year = Number(yStr);
    const month = Number(mStr) - 1;
    const monthEnd = new Date(year, month + 1, 0, 23, 59, 59, 999);
    let found: InvestmentSnapshot | null = null;
    for (const snap of sorted) {
      if (new Date(snap.recorded_at).getTime() <= monthEnd.getTime()) {
        found = snap;
      } else {
        break;
      }
    }
    out.set(key, found);
  }
  return out;
}

export function endOfCurrentMonth(): Date {
  const d = new Date();
  return new Date(d.getFullYear(), d.getMonth() + 1, 0, 23, 59, 59, 999);
}

export function parseNonNegFloat(value: string): number | null {
  const n = Number(value);
  if (!Number.isFinite(n) || n < 0) return null;
  return n;
}

export function parsePosFloat(value: string): number | null {
  const n = Number(value);
  if (!Number.isFinite(n) || n <= 0) return null;
  return n;
}

// Parse "YYYY-MM-DD" from <input type="date"> as a local-time Date,
// avoiding the UTC midnight shift you'd get from `new Date("2024-01-01")`.
export function parseLocalDate(yyyymmdd: string): Date {
  const parts = yyyymmdd.split("-").map(Number);
  const y = parts[0] ?? 1970;
  const m = parts[1] ?? 1;
  const d = parts[2] ?? 1;
  return new Date(y, m - 1, d);
}

export function friendlyMutationError(err: { code?: string; message: string }): string {
  if (err.code === "42501" || /row-level security|policy/i.test(err.message)) {
    return "Access denied — your account is not on the whitelist.";
  }
  return err.message;
}

// Splits pasted spreadsheet text into a 2D array. Tab-separated when any tab
// is present on a line (default for spreadsheet copy), otherwise comma-separated.
// Empty lines are skipped.
export function splitPastedRows(text: string): string[][] {
  return text
    .split(/\r?\n/)
    .map((line) => line.replace(/\r$/, ""))
    .filter((line) => line.trim() !== "")
    .map((line) => {
      const cells = line.includes("\t") ? line.split("\t") : line.split(",");
      return cells.map((c) => c.trim());
    });
}
