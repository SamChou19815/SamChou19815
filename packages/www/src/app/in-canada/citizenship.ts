import { parseLocalDate } from "../budget/utils";

// Counting starts on this date, matching `START_DATE` in the Rust CLI.
export const START_DATE = parseLocalDate("2025-01-23");

/** Days of physical presence needed to apply for citizenship. */
export const TARGET_DAYS = 1095;
/** Only presence within this many years of the application date counts. */
export const WINDOW_YEARS = 5;
/** Days before permanent residency count half, up to this much credit in total. */
export const PRE_PR_CREDIT_CAP = 365;
/** How far `projectEligibility` looks ahead before giving up. */
const PROJECTION_HORIZON_DAYS = 3650;

export type CitizenshipProgress = {
  /** Days present in Canada before the PR date, within the window. */
  prePrDays: number;
  /** `prePrDays` at the half-day rate, capped at `PRE_PR_CREDIT_CAP`. */
  prePrCredit: number;
  /** Days present on or after the PR date, within the window; these count fully. */
  prDays: number;
  total: number;
  remaining: number;
  /** `total` as a share of `TARGET_DAYS`, 0-100. */
  percent: number;
};

/** `YYYY-MM-DD`, optionally followed by a `(D)` departure or `(A)` arrival marker. */
const ENTRY_PATTERN = /^(\d{4}-\d{2}-\d{2})\s*(?:\(([DA])\))?$/i;

export type TravelDays = {
  /** Days spent entirely outside Canada. */
  away: ReadonlySet<string>;
  /**
   * Departure and arrival days. Any part of a day spent in Canada counts as a
   * full day of physical presence, so these count as in Canada; they're kept
   * only so the calendar can mark them.
   */
  travel: ReadonlySet<string>;
};

/**
 * Parse the stored one-entry-per-line text. A date listed both bare and with a
 * marker counts as a travel day, since part of it was spent in Canada. Lines
 * that don't parse are ignored.
 *
 * Mirrors `parse_entry` in `crates/sam-cli/src/commands/in_canada.rs`.
 */
export function parseTravelDays(text: string): TravelDays {
  const away = new Set<string>();
  const travel = new Set<string>();
  for (const line of text.split("\n")) {
    const match = ENTRY_PATTERN.exec(line.trim());
    if (match == null) continue;
    const date = match[1] ?? "";
    if (match[2] != null) {
      travel.add(date);
    } else {
      away.add(date);
    }
  }
  for (const date of travel) away.delete(date);
  return { away, travel };
}

function addDays(date: Date, days: number): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
}

/** A local-time `Date` back to the `YYYY-MM-DD` form the app stores. */
export function formatLocalDate(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(
    date.getDate(),
  ).padStart(2, "0")}`;
}

/** First day of the trailing `WINDOW_YEARS` window ending on `asOf`, inclusive. */
export function windowStart(asOf: Date): Date {
  const back = new Date(asOf.getFullYear() - WINDOW_YEARS, asOf.getMonth(), asOf.getDate());
  return addDays(back, 1);
}

/**
 * Credit earned toward citizenship as of `asOf`.
 *
 * Days before `prDate` were spent as a temporary resident, so each counts as
 * half a day and they contribute at most `PRE_PR_CREDIT_CAP` in total; days
 * from `prDate` onward count in full. Only the trailing `WINDOW_YEARS` count,
 * and nothing is known before `START_DATE`, so the walk begins at the later of
 * the two. With no `prDate` recorded every day is a pre-PR day, which caps the
 * total below the target — the honest answer, not a placeholder.
 *
 * Mirrors `progress` in `crates/sam-cli/src/commands/in_canada.rs`.
 */
export function computeCitizenshipProgress(
  asOf: Date,
  prDate: Date | null,
  awayDays: ReadonlySet<string>,
): CitizenshipProgress {
  const start = windowStart(asOf);
  let day = start > START_DATE ? start : START_DATE;
  let prePrDays = 0;
  let prDays = 0;
  while (day <= asOf) {
    if (!awayDays.has(formatLocalDate(day))) {
      if (prDate != null && day >= prDate) {
        prDays++;
      } else {
        prePrDays++;
      }
    }
    day = addDays(day, 1);
  }
  const prePrCredit = Math.min(Math.floor(prePrDays / 2), PRE_PR_CREDIT_CAP);
  const total = prePrCredit + prDays;
  return {
    prePrDays,
    prePrCredit,
    prDays,
    total,
    remaining: Math.max(0, TARGET_DAYS - total),
    percent: Math.min(100, (total / TARGET_DAYS) * 100),
  };
}

export type Milestone = {
  key: string;
  label: string;
  /** Credited days needed to reach it. */
  threshold: number;
};

/**
 * Progress milestones in ascending order. Thresholds are rounded up, so on the
 * milestone's date the credited share is at least the stated fraction.
 */
export const MILESTONES: readonly Milestone[] = (
  [
    ["10", "10%", 1, 10],
    ["20", "20%", 2, 10],
    ["25", "25%", 1, 4],
    ["30", "30%", 3, 10],
    ["33", "⅓", 1, 3],
    ["40", "40%", 4, 10],
    ["50", "50%", 1, 2],
    ["60", "60%", 6, 10],
    ["67", "⅔", 2, 3],
    ["70", "70%", 7, 10],
    ["75", "75%", 3, 4],
    ["80", "80%", 8, 10],
    ["90", "90%", 9, 10],
    ["100", "Done", 1, 1],
  ] as const
).map(([key, label, numerator, denominator]) => ({
  key,
  label,
  // Multiply before dividing so exact fractions like ⅓ of 1095 stay integral.
  threshold: Math.ceil((TARGET_DAYS * numerator) / denominator),
}));

/**
 * The first day each of `MILESTONES` is reached, keyed by `Milestone.key`, or
 * `null` when it stays out of reach within the projection horizon past `asOf`.
 * Days after `asOf` are projected the same way as `projectEligibility`.
 *
 * Equivalent to calling `computeCitizenshipProgress` for every day from
 * `START_DATE`, but slides the window along instead of recounting it.
 */
export function findMilestoneDates(
  asOf: Date,
  prDate: Date | null,
  awayDays: ReadonlySet<string>,
): ReadonlyMap<string, Date | null> {
  const dates = new Map<string, Date | null>(MILESTONES.map(({ key }) => [key, null]));
  let prePrDays = 0;
  let prDays = 0;
  const count = (day: Date, delta: number) => {
    if (awayDays.has(formatLocalDate(day))) return;
    if (prDate != null && day >= prDate) {
      prDays += delta;
    } else {
      prePrDays += delta;
    }
  };

  const end = addDays(asOf, PROJECTION_HORIZON_DAYS);
  // The oldest day still inside the window.
  let tail = START_DATE;
  let next = 0;
  for (let day = START_DATE; day <= end && next < MILESTONES.length; day = addDays(day, 1)) {
    count(day, 1);
    const start = windowStart(day);
    while (tail < start) {
      count(tail, -1);
      tail = addDays(tail, 1);
    }
    const total = Math.min(Math.floor(prePrDays / 2), PRE_PR_CREDIT_CAP) + prDays;
    for (let milestone = MILESTONES[next]; milestone != null; milestone = MILESTONES[next]) {
      if (total < milestone.threshold) break;
      dates.set(milestone.key, day);
      next++;
    }
  }
  return dates;
}

/**
 * The first day the requirement is met, assuming presence in Canada from `asOf`
 * onward except on future days already in `awayDays` (planned trips). `null`
 * when it stays out of reach within the horizon — notably with no PR date,
 * where the credit is capped below the target.
 */
export function projectEligibility(
  asOf: Date,
  prDate: Date | null,
  awayDays: ReadonlySet<string>,
): Date | null {
  let day = asOf;
  for (let i = 0; i <= PROJECTION_HORIZON_DAYS; i++) {
    if (computeCitizenshipProgress(day, prDate, awayDays).total >= TARGET_DAYS) return day;
    day = addDays(day, 1);
  }
  return null;
}
