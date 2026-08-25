import { parseLocalDate } from "../budget/utils";

// Counting starts on this date, matching `START_DATE` in the Rust CLI.
export const START_DATE = parseLocalDate("2025-01-24");

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
  /** `total` as a share of `TARGET_DAYS`, 0–100. */
  percent: number;
};

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

/**
 * The first day the requirement is met, assuming presence in Canada every day
 * from `asOf` onward. `null` when it stays out of reach within the horizon —
 * notably with no PR date, where the credit is capped below the target.
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
