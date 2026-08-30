//! `sam in-canada` — mirror of the In-Canada Days Counter web app.

use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::{Days, Months, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::charts::{bold, hbar, paint, term_width, Color};
use crate::cli::InCanadaCommand;
use crate::supabase::Supabase;

/// Counting starts on this date, matching `START_DATE` in the web app.
const START_DATE: &str = "2025-01-24";

/// Days of physical presence needed to apply for citizenship.
const TARGET_DAYS: i64 = 1095;
/// Only presence within this many years of the application date counts.
const WINDOW_YEARS: u32 = 5;
/// Days before permanent residency count half, up to this much credit in total.
const PRE_PR_CREDIT_CAP: i64 = 365;
/// How far `eligible_on` projects forward before giving up.
const PROJECTION_HORIZON_DAYS: usize = 3650;

#[derive(Deserialize)]
struct Row {
    missing_days: Option<String>,
    /// Absent from older rows (and from selects that don't ask for it).
    #[serde(default)]
    pr_date: Option<String>,
}

/// The persisted counter state: days spent outside Canada, plus the date
/// permanent residency began, once recorded.
struct State {
    away: BTreeSet<NaiveDate>,
    pr_date: Option<NaiveDate>,
}

pub fn run(sb: &Supabase, command: Option<InCanadaCommand>) -> Result<()> {
    match command.unwrap_or(InCanadaCommand::Status) {
        InCanadaCommand::Status => status(sb),
        InCanadaCommand::List => list(sb),
        InCanadaCommand::Add { dates } => add(sb, &dates),
        InCanadaCommand::Remove { dates } => remove(sb, &dates),
        InCanadaCommand::PrDate { date, clear } => pr_date(sb, date.as_deref(), clear),
    }
}

fn start_date() -> NaiveDate {
    NaiveDate::parse_from_str(START_DATE, "%Y-%m-%d").expect("START_DATE is valid")
}

/// Fetch the persisted state. Unparseable dates are silently skipped.
///
/// Selects every column rather than naming them: PostgREST rejects a select
/// that names a column the table doesn't have, and `pr_date` only exists once
/// the migration has been applied. It's a single row either way.
fn fetch(sb: &Supabase) -> Result<State> {
    let query = format!("select=*&user_id=eq.{}", sb.user_id());
    let rows: Vec<Row> = sb.select("in_canada", &query)?;
    let mut away = BTreeSet::new();
    let mut pr_date = None;
    if let Some(row) = rows.into_iter().next() {
        if let Some(text) = row.missing_days {
            for line in text.lines() {
                if let Some(date) = parse_date(line) {
                    away.insert(date);
                }
            }
        }
        pr_date = row.pr_date.as_deref().and_then(parse_date);
    }
    Ok(State { away, pr_date })
}

fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").ok()
}

/// Persist the set back, one ISO date per line (matching the web app's format).
/// `pr_date` is left out of the payload so the upsert doesn't disturb it.
fn save_missing_days(sb: &Supabase, days: &BTreeSet<NaiveDate>) -> Result<()> {
    let text = days
        .iter()
        .map(|d| d.format("%Y-%m-%d").to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let body = json!({
        "user_id": sb.user_id(),
        "missing_days": text,
        "updated_at": Utc::now().to_rfc3339(),
    });
    sb.insert("in_canada", &body, Some("user_id"))
}

/// Persist the PR date; `None` clears it. Leaves `missing_days` untouched.
fn save_pr_date(sb: &Supabase, date: Option<NaiveDate>) -> Result<()> {
    let body = json!({
        "user_id": sb.user_id(),
        "pr_date": date.map(|d| d.format("%Y-%m-%d").to_string()),
        "updated_at": Utc::now().to_rfc3339(),
    });
    sb.insert("in_canada", &body, Some("user_id"))
}

/// The headline counters, inclusive on both ends to match the web app's
/// `getDaysBetween`.
struct Counts {
    total: i64,
    away: i64,
    in_canada: i64,
}

fn counts(start: NaiveDate, today: NaiveDate, away: &BTreeSet<NaiveDate>) -> Counts {
    let total = (today - start).num_days() + 1;
    let away = away.iter().filter(|d| **d >= start && **d <= today).count() as i64;
    Counts {
        total,
        away,
        in_canada: total - away,
    }
}

/// Progress toward the citizenship physical-presence requirement.
#[derive(Debug, PartialEq, Eq)]
struct Progress {
    /// Days present in Canada before the PR date, within the window.
    pre_pr_days: i64,
    /// `pre_pr_days` at the half-day rate, capped at `PRE_PR_CREDIT_CAP`.
    pre_pr_credit: i64,
    /// Days present on or after the PR date, within the window; these count fully.
    pr_days: i64,
    total: i64,
    remaining: i64,
}

/// First day of the trailing `WINDOW_YEARS` window ending on `as_of`, inclusive.
fn window_start(as_of: NaiveDate) -> NaiveDate {
    as_of
        .checked_sub_months(Months::new(WINDOW_YEARS * 12))
        .and_then(|d| d.checked_add_days(Days::new(1)))
        .unwrap_or(NaiveDate::MIN)
}

/// Credit earned toward citizenship as of `as_of`.
///
/// Days before `pr_date` were spent as a temporary resident, so each counts as
/// half a day and they contribute at most `PRE_PR_CREDIT_CAP` in total; days
/// from `pr_date` onward count in full. Only the trailing `WINDOW_YEARS` count,
/// and nothing is known before `start`, so the walk begins at the later of the
/// two. With no `pr_date` recorded every day is a pre-PR day, which caps the
/// total below the target — the honest answer, not a placeholder.
fn progress(
    start: NaiveDate,
    as_of: NaiveDate,
    pr_date: Option<NaiveDate>,
    away: &BTreeSet<NaiveDate>,
) -> Progress {
    let mut pre_pr_days = 0;
    let mut pr_days = 0;
    let mut day = window_start(as_of).max(start);
    while day <= as_of {
        if !away.contains(&day) {
            match pr_date {
                Some(pr) if day >= pr => pr_days += 1,
                _ => pre_pr_days += 1,
            }
        }
        let Some(next) = day.succ_opt() else { break };
        day = next;
    }
    let pre_pr_credit = (pre_pr_days / 2).min(PRE_PR_CREDIT_CAP);
    let total = pre_pr_credit + pr_days;
    Progress {
        pre_pr_days,
        pre_pr_credit,
        pr_days,
        total,
        remaining: (TARGET_DAYS - total).max(0),
    }
}

/// The first day the requirement is met, assuming presence in Canada every day
/// from `as_of` onward. `None` when it stays out of reach within the horizon —
/// notably with no PR date, where the credit is capped below the target.
fn eligible_on(
    start: NaiveDate,
    as_of: NaiveDate,
    pr_date: Option<NaiveDate>,
    away: &BTreeSet<NaiveDate>,
) -> Option<NaiveDate> {
    let mut day = as_of;
    for _ in 0..=PROJECTION_HORIZON_DAYS {
        if progress(start, day, pr_date, away).total >= TARGET_DAYS {
            return Some(day);
        }
        day = day.succ_opt()?;
    }
    None
}

/// A two-tone bar `width` cells wide: pre-PR credit in blue, full-rate PR days
/// in green, the shortfall dimmed. The unfilled tail is drawn rather than
/// padded so the row keeps its width once the ANSI codes are stripped.
fn progress_bar(p: &Progress, width: usize) -> String {
    let bar = hbar(p.total as f64, TARGET_DAYS as f64, width);
    // Slice by chars: the eighth-block glyphs are multi-byte.
    let filled = bar.chars().count();
    let pre = (((p.pre_pr_credit as f64 / TARGET_DAYS as f64) * width as f64).round() as usize)
        .min(filled);
    let head: String = bar.chars().take(pre).collect();
    let tail: String = bar.chars().skip(pre).collect();
    let rest = "░".repeat(width.saturating_sub(filled));
    format!(
        "{}{}{}",
        paint(&head, Color::Blue),
        paint(&tail, Color::Green),
        paint(&rest, Color::Dim)
    )
}

fn status(sb: &Supabase) -> Result<()> {
    let state = fetch(sb)?;
    let start = start_date();
    let today = Utc::now().date_naive();
    let c = counts(start, today, &state.away);

    println!("{}", bold(&paint("In-Canada Days Counter", Color::Blue)));
    println!(
        "{}",
        paint(
            &format!("Counting since {}", start.format("%B %-d, %Y")),
            Color::Dim
        )
    );
    println!();
    println!(
        "  Days in Canada : {}",
        bold(&paint(&c.in_canada.to_string(), Color::Green))
    );
    println!(
        "  Days away      : {}",
        paint(&c.away.to_string(), Color::Red)
    );
    println!(
        "  Total days     : {}",
        paint(&c.total.to_string(), Color::Cyan)
    );

    print_citizenship(start, today, &state);
    Ok(())
}

fn print_citizenship(start: NaiveDate, today: NaiveDate, state: &State) {
    let p = progress(start, today, state.pr_date, &state.away);

    println!();
    println!("{}", bold(&paint("Progress to citizenship", Color::Blue)));
    println!(
        "{}",
        paint(
            &format!("{TARGET_DAYS} days of physical presence in the last {WINDOW_YEARS} years"),
            Color::Dim
        )
    );
    println!();
    match state.pr_date {
        Some(pr) => println!(
            "  PR since       : {}",
            paint(&pr.format("%Y-%m-%d").to_string(), Color::Cyan)
        ),
        None => println!("  PR since       : {}", paint("not set", Color::Dim)),
    }
    println!(
        "  Pre-PR credit  : {} {}",
        paint(&p.pre_pr_credit.to_string(), Color::Blue),
        paint(
            &format!("({} days at ½, cap {PRE_PR_CREDIT_CAP})", p.pre_pr_days),
            Color::Dim
        )
    );
    if state.pr_date.is_some() {
        println!(
            "  Days as PR     : {}",
            paint(&p.pr_days.to_string(), Color::Green)
        );
    }
    println!(
        "  Days remaining : {}",
        paint(&p.remaining.to_string(), Color::Cyan)
    );
    println!();

    // Non-bar columns: "  " + bar + "  " + summary + "  " + "100%".
    let summary = format!("{} / {TARGET_DAYS}", p.total);
    let bar_w = term_width()
        .saturating_sub(2 + 2 + summary.chars().count() + 2 + 4)
        .max(8);
    let pct = p.total as f64 / TARGET_DAYS as f64 * 100.0;
    println!("  {}  {}  {:>3.0}%", progress_bar(&p, bar_w), summary, pct);

    match eligible_on(start, today, state.pr_date, &state.away) {
        Some(day) if day <= today => println!(
            "  {}",
            paint("Requirement met — you can apply today.", Color::Green)
        ),
        Some(day) => println!(
            "  Eligible on    : {} {}",
            paint(&day.format("%Y-%m-%d").to_string(), Color::Cyan),
            paint("(if present every day)", Color::Dim)
        ),
        None => {}
    }
    if state.pr_date.is_none() {
        println!(
            "{}",
            paint(
                "  Set a PR date to count full days: sam in-canada pr-date <YYYY-MM-DD>",
                Color::Dim
            )
        );
    }
}

fn list(sb: &Supabase) -> Result<()> {
    let days = fetch(sb)?.away;
    if days.is_empty() {
        println!("No days outside Canada recorded.");
        return Ok(());
    }
    for day in &days {
        println!("{}", paint(&day.format("%Y-%m-%d").to_string(), Color::Red));
    }
    println!(
        "{}",
        paint(&format!("\n{} day(s) recorded.", days.len()), Color::Dim)
    );
    Ok(())
}

fn add(sb: &Supabase, dates: &[String]) -> Result<()> {
    let parsed = parse_dates(dates)?;
    let mut days = fetch(sb)?.away;
    let added = parsed.iter().filter(|d| days.insert(**d)).count();
    save_missing_days(sb, &days)?;
    println!(
        "{} day(s) added; {} total.",
        paint(&added.to_string(), Color::Green),
        days.len()
    );
    Ok(())
}

fn remove(sb: &Supabase, dates: &[String]) -> Result<()> {
    let parsed = parse_dates(dates)?;
    let mut days = fetch(sb)?.away;
    let removed = parsed.iter().filter(|d| days.remove(*d)).count();
    save_missing_days(sb, &days)?;
    println!(
        "{} day(s) removed; {} total.",
        paint(&removed.to_string(), Color::Red),
        days.len()
    );
    Ok(())
}

fn pr_date(sb: &Supabase, date: Option<&str>, clear: bool) -> Result<()> {
    if clear {
        save_pr_date(sb, None)?;
        println!("PR date cleared; every tracked day now counts as half a day.");
        return Ok(());
    }
    let Some(date) = date else {
        match fetch(sb)?.pr_date {
            Some(pr) => println!(
                "PR since {}.",
                paint(&pr.format("%Y-%m-%d").to_string(), Color::Cyan)
            ),
            None => println!("No PR date recorded."),
        }
        return Ok(());
    };
    let parsed = parse_date_arg(date)?;
    save_pr_date(sb, Some(parsed))?;
    println!(
        "PR date set to {}.",
        paint(&parsed.format("%Y-%m-%d").to_string(), Color::Cyan)
    );
    Ok(())
}

fn parse_date_arg(date: &str) -> Result<NaiveDate> {
    match parse_date(date) {
        Some(parsed) => Ok(parsed),
        None => bail!("invalid date '{date}' (expected YYYY-MM-DD)"),
    }
}

fn parse_dates(dates: &[String]) -> Result<Vec<NaiveDate>> {
    dates.iter().map(|d| parse_date_arg(d)).collect()
}
