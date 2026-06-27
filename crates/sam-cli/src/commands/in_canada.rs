//! `sam in-canada` — mirror of the In-Canada Days Counter web app.

use std::collections::BTreeSet;

use anyhow::{bail, Result};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use serde_json::json;

use crate::charts::{bold, paint, Color};
use crate::cli::InCanadaCommand;
use crate::supabase::Supabase;

/// Counting starts on this date, matching `START_DATE` in the web app.
const START_DATE: &str = "2025-01-24";

#[derive(Deserialize)]
struct Row {
    missing_days: Option<String>,
}

pub fn run(sb: &Supabase, command: Option<InCanadaCommand>) -> Result<()> {
    match command.unwrap_or(InCanadaCommand::Status) {
        InCanadaCommand::Status => status(sb),
        InCanadaCommand::List => list(sb),
        InCanadaCommand::Add { dates } => add(sb, &dates),
        InCanadaCommand::Remove { dates } => remove(sb, &dates),
    }
}

fn start_date() -> NaiveDate {
    NaiveDate::parse_from_str(START_DATE, "%Y-%m-%d").expect("START_DATE is valid")
}

/// Fetch the persisted set of days outside Canada, sorted and de-duplicated.
fn fetch_missing_days(sb: &Supabase) -> Result<BTreeSet<NaiveDate>> {
    let query = format!("select=missing_days&user_id=eq.{}", sb.user_id());
    let rows: Vec<Row> = sb.select("in_canada", &query)?;
    let mut days = BTreeSet::new();
    if let Some(text) = rows.into_iter().next().and_then(|r| r.missing_days) {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(date) = NaiveDate::parse_from_str(line, "%Y-%m-%d") {
                days.insert(date);
            }
        }
    }
    Ok(days)
}

/// Persist the set back, one ISO date per line (matching the web app's format).
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

fn status(sb: &Supabase) -> Result<()> {
    let days = fetch_missing_days(sb)?;
    let start = start_date();
    let today = Utc::now().date_naive();

    // Inclusive day count, matching the web app's getDaysBetween.
    let total_days = (today - start).num_days() + 1;
    let away = days.iter().filter(|d| **d >= start && **d <= today).count() as i64;
    let in_canada = total_days - away;

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
        bold(&paint(&in_canada.to_string(), Color::Green))
    );
    println!(
        "  Days away      : {}",
        paint(&away.to_string(), Color::Red)
    );
    println!(
        "  Total days     : {}",
        paint(&total_days.to_string(), Color::Cyan)
    );
    Ok(())
}

fn list(sb: &Supabase) -> Result<()> {
    let days = fetch_missing_days(sb)?;
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
    let mut days = fetch_missing_days(sb)?;
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
    let mut days = fetch_missing_days(sb)?;
    let removed = parsed.iter().filter(|d| days.remove(*d)).count();
    save_missing_days(sb, &days)?;
    println!(
        "{} day(s) removed; {} total.",
        paint(&removed.to_string(), Color::Red),
        days.len()
    );
    Ok(())
}

fn parse_dates(dates: &[String]) -> Result<Vec<NaiveDate>> {
    dates
        .iter()
        .map(|d| match NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d") {
            Ok(date) => Ok(date),
            Err(_) => bail!("invalid date '{d}' (expected YYYY-MM-DD)"),
        })
        .collect()
}
