//! `sam budget` — mirror of the Budget web app's read views. Editing lives in
//! the interactive `sam budget tui` (see `budget_tui`).

use std::collections::BTreeMap;

use anyhow::Result;
use chrono::{DateTime, Datelike, Months, NaiveDate, Utc};
use serde::{Deserialize, Deserializer};

use crate::charts::{bold, chart, hbar, paint, paint_signed, term_width, Color};
use crate::cli::BudgetCommand;
use crate::supabase::Supabase;

// Several commands request only a subset of columns; fields not always
// selected are marked `#[serde(default)]` so partial rows still deserialize.
#[derive(Deserialize)]
struct Expense {
    #[serde(default)]
    name: String,
    category: String,
    #[serde(deserialize_with = "de_num")]
    amount: f64,
    date: String,
}

#[derive(Deserialize)]
struct Income {
    #[serde(default)]
    name: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(deserialize_with = "de_num")]
    amount: f64,
    date: String,
}

#[derive(Deserialize)]
struct Investment {
    #[serde(default)]
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    currency: String,
    #[serde(deserialize_with = "de_num")]
    total_cost: f64,
    #[serde(deserialize_with = "de_num")]
    total_market_value: f64,
    #[serde(deserialize_with = "de_num")]
    exchange_rate: f64,
    #[serde(default)]
    updated_at: String,
}

impl Investment {
    /// Market value converted to CAD, matching the web app's `cadValue`.
    fn cad_value(&self) -> f64 {
        self.total_market_value * self.exchange_rate
    }
}

#[derive(Deserialize)]
struct Snapshot {
    investment_id: String,
    recorded_at: String,
    #[serde(deserialize_with = "de_num")]
    total_market_value: f64,
    #[serde(deserialize_with = "de_num")]
    exchange_rate: f64,
}

impl Snapshot {
    fn cad_value(&self) -> f64 {
        self.total_market_value * self.exchange_rate
    }
}

pub fn run(sb: &Supabase, command: Option<BudgetCommand>) -> Result<()> {
    match command.unwrap_or(BudgetCommand::Status { months: 12 }) {
        BudgetCommand::Status { months } => status(sb, months),
        BudgetCommand::Summary { months } => summary(sb, months),
        BudgetCommand::Expenses { limit } => expenses(sb, limit),
        BudgetCommand::Income { limit } => income(sb, limit),
        BudgetCommand::Investments => investments(sb),
        BudgetCommand::Tui => super::budget_tui::run(sb),
    }
}

fn summary(sb: &Supabase, months: u32) -> Result<()> {
    // Inclusive lower bound: first day of the month `months - 1` months ago.
    let now = Utc::now().date_naive();
    let start = now
        .with_day(1)
        .unwrap_or(now)
        .checked_sub_months(Months::new(months.saturating_sub(1)))
        .unwrap_or(now);
    let since = start.format("%Y-%m-%d").to_string();

    let incomes: Vec<Income> = sb.select_all(
        "incomes",
        &format!(
            "select=amount,date&user_id=eq.{}&date=gte.{since}",
            sb.user_id()
        ),
    )?;
    let expenses: Vec<Expense> = sb.select_all(
        "expenses",
        &format!(
            "select=amount,date,name,category&user_id=eq.{}&date=gte.{since}",
            sb.user_id()
        ),
    )?;
    let investments: Vec<Investment> = sb.select_all(
        "investments",
        &format!("select=*&user_id=eq.{}", sb.user_id()),
    )?;

    let income_total: f64 = incomes.iter().map(|i| i.amount).sum();
    let expense_total: f64 = expenses.iter().map(|e| e.amount).sum();
    let invest_total: f64 = investments.iter().map(Investment::cad_value).sum();

    let net = income_total - expense_total;
    println!(
        "{}",
        bold(&format!(
            "Budget summary (last {months} month(s), since {since})"
        ))
    );
    println!();
    println!(
        "  Income       : {}",
        paint(&cad(income_total), Color::Green)
    );
    println!(
        "  Expenses     : {}",
        paint(&cad(expense_total), Color::Red)
    );
    println!("  Net          : {}", paint_signed(&cad(net), net));
    println!(
        "  Investments  : {}  (current market value)",
        paint(&cad(invest_total), Color::Cyan)
    );
    Ok(())
}

/// The dashboard view: stat totals plus terminal charts mirroring the web app.
fn status(sb: &Supabase, months: u32) -> Result<()> {
    let month_keys = trailing_months(months);
    let in_range = |date: &str| month_keys.iter().any(|m| date.starts_with(m.as_str()));
    let since = month_keys.first().cloned().unwrap_or_default();

    // Fetch everything once, then bucket/filter in memory, mirroring the web app.
    let incomes: Vec<Income> = sb.select_all(
        "incomes",
        &format!("select=amount,date,source&user_id=eq.{}", sb.user_id()),
    )?;
    let expenses: Vec<Expense> = sb.select_all(
        "expenses",
        &format!("select=amount,date,category&user_id=eq.{}", sb.user_id()),
    )?;
    let investments: Vec<Investment> = sb.select_all(
        "investments",
        &format!("select=*&user_id=eq.{}", sb.user_id()),
    )?;
    let snapshots: Vec<Snapshot> = sb.select_all(
        "investment_snapshots",
        &format!(
            "select=investment_id,recorded_at,total_market_value,exchange_rate&user_id=eq.{}",
            sb.user_id()
        ),
    )?;

    // --- Stat tiles --------------------------------------------------------
    let income_total: f64 = incomes
        .iter()
        .filter(|i| in_range(&i.date))
        .map(|i| i.amount)
        .sum();
    let expense_total: f64 = expenses
        .iter()
        .filter(|e| in_range(&e.date))
        .map(|e| e.amount)
        .sum();
    let invest_total: f64 = investments.iter().map(Investment::cad_value).sum();

    let net = income_total - expense_total;
    println!(
        "{}",
        bold(&paint(
            &format!("Budget — last {months} month(s)"),
            Color::Blue
        ))
    );
    println!();
    println!("  Income       {}", paint(&cad(income_total), Color::Green));
    println!("  Expenses     {}", paint(&cad(expense_total), Color::Red));
    println!("  Net          {}", paint_signed(&cad(net), net));
    println!(
        "  Investments  {}  (market value)",
        paint(&cad(invest_total), Color::Cyan)
    );

    // --- Income vs expenses, per month ------------------------------------
    let income_by = bucket_sum(&incomes, |i| (i.date.as_str(), i.amount));
    let expense_by = bucket_sum(&expenses, |e| (e.date.as_str(), e.amount));
    let monthly_max = month_keys
        .iter()
        .flat_map(|m| {
            [
                *income_by.get(m).unwrap_or(&0.0),
                *expense_by.get(m).unwrap_or(&0.0),
            ]
        })
        .fold(0.0_f64, f64::max);

    section("Income vs expenses (monthly)");
    // Layout per row: "  YYYY-MM  in  " prefix (15) + bar + " " + amount.
    let amount_w = month_keys
        .iter()
        .flat_map(|m| {
            [
                cad(*income_by.get(m).unwrap_or(&0.0)).len(),
                cad(*expense_by.get(m).unwrap_or(&0.0)).len(),
            ]
        })
        .max()
        .unwrap_or(12);
    let bar_w = term_width().saturating_sub(15 + 1 + amount_w).max(8);
    for m in &month_keys {
        let inc = *income_by.get(m).unwrap_or(&0.0);
        let exp = *expense_by.get(m).unwrap_or(&0.0);
        // Pad the bar to its column width *before* coloring so the ANSI escape
        // codes don't throw off the amount alignment in a real terminal.
        let in_bar = format!("{:<bar_w$}", hbar(inc, monthly_max, bar_w));
        let out_bar = format!("{:<bar_w$}", hbar(exp, monthly_max, bar_w));
        println!(
            "  {m}  in  {} {:>amount_w$}",
            paint(&in_bar, Color::Green),
            cad(inc)
        );
        println!(
            "           out {} {:>amount_w$}",
            paint(&out_bar, Color::Red),
            cad(exp)
        );
    }

    // Each chart spans the full terminal width and a fixed height; the figures
    // live in the heading so the plot itself can use the whole terminal.
    let chart_w = term_width().saturating_sub(2).max(8);
    let chart_h = 8;
    let plot = |series: &[f64], color: Color| {
        for row in chart(series, chart_w, chart_h) {
            println!("  {}", paint(&row, color));
        }
    };

    // --- Cumulative cash flow (running net over all time) ------------------
    let cash_flow = cumulative_cash_flow(&incomes, &expenses, &month_keys);
    if let (Some(first), Some(last)) = (cash_flow.first(), cash_flow.last()) {
        section(&format!(
            "Cumulative cash flow  ({} → {})",
            cad(*first),
            cad(*last)
        ));
        plot(&cash_flow, Color::Blue);
    }

    // --- Investment market value over time --------------------------------
    let value_series = investment_value_series(&investments, &snapshots, &month_keys);
    if value_series.iter().any(|v| *v > 0.0) {
        let last = value_series.last().copied().unwrap_or(0.0);
        section(&format!(
            "Investment market value over time  (→ {})",
            cad(last)
        ));
        plot(&value_series, Color::Cyan);
    }

    // --- Category breakdowns ----------------------------------------------
    let expense_cats = category_totals(expenses.iter().filter(|e| in_range(&e.date)), |e| {
        (&e.category, e.amount)
    });
    breakdown(
        &format!("Expenses by category (since {since})"),
        &expense_cats,
        Color::Red,
    );

    let income_cats = category_totals(incomes.iter().filter(|i| in_range(&i.date)), |i| {
        (i.source.as_deref().unwrap_or("(no source)"), i.amount)
    });
    breakdown(
        &format!("Income by source (since {since})"),
        &income_cats,
        Color::Green,
    );

    let alloc = category_totals(investments.iter(), |inv| (&inv.kind, inv.cad_value()));
    breakdown("Investment allocation by type", &alloc, Color::Cyan);

    Ok(())
}

fn section(title: &str) {
    println!();
    println!("{}", paint(title, Color::Dim));
}

/// Render a sorted, percentage-annotated horizontal-bar breakdown.
fn breakdown(title: &str, totals: &[(String, f64)], color: Color) {
    section(title);
    let grand: f64 = totals.iter().map(|(_, v)| v).sum();
    if grand <= 0.0 {
        println!("  (none)");
        return;
    }
    let label_w = totals
        .iter()
        .map(|(n, _)| n.len())
        .max()
        .unwrap_or(0)
        .min(20);
    let max = totals.iter().map(|(_, v)| *v).fold(0.0_f64, f64::max);
    let amount_w = totals
        .iter()
        .map(|(_, v)| cad(*v).len())
        .max()
        .unwrap_or(12);
    // Non-bar columns: "  " + label + "  " + " " + amount + "  " + "100%".
    let bar_w = term_width()
        .saturating_sub(2 + label_w + 2 + 1 + amount_w + 2 + 5)
        .max(8);
    for (name, value) in totals {
        let pct = value / grand * 100.0;
        // Pad the bar before coloring so ANSI codes don't skew column alignment.
        let bar = format!("{:<bar_w$}", hbar(*value, max, bar_w));
        println!(
            "  {:<label_w$}  {} {:>amount_w$}  {:>4.0}%",
            truncate(name, label_w),
            paint(&bar, color),
            cad(*value),
            pct,
        );
    }
}

/// Trailing `months` month keys ("YYYY-MM"), oldest first, ending this month.
fn trailing_months(months: u32) -> Vec<String> {
    let now = Utc::now().date_naive().with_day(1).unwrap();
    let mut keys: Vec<String> = (0..months.max(1))
        .rev()
        .filter_map(|back| now.checked_sub_months(Months::new(back)))
        .map(|d| d.format("%Y-%m").to_string())
        .collect();
    keys.dedup();
    keys
}

/// Sum amounts into "YYYY-MM" buckets (the web app's `monthBucket`).
fn bucket_sum<T>(rows: &[T], pick: impl Fn(&T) -> (&str, f64)) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    for row in rows {
        let (date, amount) = pick(row);
        if date.len() >= 7 {
            *out.entry(date[..7].to_string()).or_insert(0.0) += amount;
        }
    }
    out
}

/// Running (income − expense) total across all time, sampled at `month_keys`.
fn cumulative_cash_flow(
    incomes: &[Income],
    expenses: &[Expense],
    month_keys: &[String],
) -> Vec<f64> {
    let income_by = bucket_sum(incomes, |i| (i.date.as_str(), i.amount));
    let expense_by = bucket_sum(expenses, |e| (e.date.as_str(), e.amount));
    let mut all: BTreeMap<String, f64> = BTreeMap::new();
    for k in income_by.keys().chain(expense_by.keys()) {
        all.entry(k.clone()).or_insert(0.0);
    }
    let mut running = 0.0;
    let mut by_month = BTreeMap::new();
    for k in all.keys() {
        running += income_by.get(k).unwrap_or(&0.0) - expense_by.get(k).unwrap_or(&0.0);
        by_month.insert(k.clone(), running);
    }
    // Carry the last known running total forward through gaps in the range.
    let mut carry = 0.0;
    month_keys
        .iter()
        .map(|m| {
            if let Some(v) = by_month.get(m) {
                carry = *v;
            } else if let Some((_, v)) = by_month.range(..=m.clone()).next_back() {
                carry = *v;
            }
            carry
        })
        .collect()
}

/// Total CAD market value per month from the latest snapshot per investment
/// at-or-before each month end, matching the web app's `latestSnapshotByMonth`.
fn investment_value_series(
    investments: &[Investment],
    snapshots: &[Snapshot],
    month_keys: &[String],
) -> Vec<f64> {
    let mut by_investment: BTreeMap<&str, Vec<&Snapshot>> = BTreeMap::new();
    for s in snapshots {
        by_investment
            .entry(s.investment_id.as_str())
            .or_default()
            .push(s);
    }
    for list in by_investment.values_mut() {
        list.sort_by_key(|s| s.recorded_at.clone());
    }
    month_keys
        .iter()
        .map(|m| {
            let end = month_end(m);
            investments
                .iter()
                .filter_map(|inv| {
                    by_investment
                        .get(inv.id.as_str())?
                        .iter()
                        .rev()
                        .find(|s| recorded_before(&s.recorded_at, end))
                        .map(|s| s.cad_value())
                })
                .sum()
        })
        .collect()
}

fn month_end(month_key: &str) -> NaiveDate {
    let year: i32 = month_key[..4].parse().unwrap_or(2000);
    let month: u32 = month_key[5..7].parse().unwrap_or(1);
    let first_next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    first_next
        .and_then(|d| d.pred_opt())
        .unwrap_or_else(|| Utc::now().date_naive())
}

fn recorded_before(recorded_at: &str, month_end: NaiveDate) -> bool {
    match DateTime::parse_from_rfc3339(recorded_at) {
        Ok(dt) => dt.date_naive() <= month_end,
        // Fall back to a leading YYYY-MM-DD if it isn't full RFC3339.
        Err(_) => NaiveDate::parse_from_str(recorded_at.get(..10).unwrap_or(""), "%Y-%m-%d")
            .map(|d| d <= month_end)
            .unwrap_or(false),
    }
}

/// Group rows by key, returning `(name, total)` sorted by total descending.
fn category_totals<'a, T: 'a>(
    rows: impl Iterator<Item = &'a T>,
    pick: impl Fn(&'a T) -> (&'a str, f64),
) -> Vec<(String, f64)> {
    let mut map: BTreeMap<String, f64> = BTreeMap::new();
    for row in rows {
        let (name, amount) = pick(row);
        *map.entry(name.to_string()).or_insert(0.0) += amount;
    }
    let mut out: Vec<(String, f64)> = map.into_iter().filter(|(_, v)| *v > 0.0).collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{head}…")
    }
}

fn expenses(sb: &Supabase, limit: u32) -> Result<()> {
    let rows: Vec<Expense> = sb.select(
        "expenses",
        &format!(
            "select=date,name,category,amount&user_id=eq.{}&order=date.desc&limit={limit}",
            sb.user_id()
        ),
    )?;
    if rows.is_empty() {
        println!("No expenses found.");
        return Ok(());
    }
    let total: f64 = rows.iter().map(|e| e.amount).sum();
    let widths = (
        col_width("DATE", rows.iter().map(|r| r.date.as_str())),
        col_width("NAME", rows.iter().map(|r| r.name.as_str())),
        col_width("CATEGORY", rows.iter().map(|r| r.category.as_str())),
    );
    let header = format!(
        "{:<w0$}  {:<w1$}  {:<w2$}  {:>12}",
        "DATE",
        "NAME",
        "CATEGORY",
        "AMOUNT",
        w0 = widths.0,
        w1 = widths.1,
        w2 = widths.2
    );
    println!("{}", bold(&header));
    for r in &rows {
        // Pad each column to width *before* coloring so ANSI codes don't skew it.
        let date = paint(&format!("{:<w0$}", r.date, w0 = widths.0), Color::Dim);
        let amount = paint(&format!("{:>12}", cad(r.amount)), Color::Red);
        println!(
            "{date}  {:<w1$}  {:<w2$}  {amount}",
            r.name,
            r.category,
            w1 = widths.1,
            w2 = widths.2
        );
    }
    println!(
        "\n{} row(s), total {}",
        rows.len(),
        paint(&cad(total), Color::Red)
    );
    Ok(())
}

fn income(sb: &Supabase, limit: u32) -> Result<()> {
    let rows: Vec<Income> = sb.select(
        "incomes",
        &format!(
            "select=date,name,source,amount&user_id=eq.{}&order=date.desc&limit={limit}",
            sb.user_id()
        ),
    )?;
    if rows.is_empty() {
        println!("No income found.");
        return Ok(());
    }
    let total: f64 = rows.iter().map(|i| i.amount).sum();
    let source_of = |r: &Income| r.source.clone().unwrap_or_default();
    let widths = (
        col_width("DATE", rows.iter().map(|r| r.date.as_str())),
        col_width("NAME", rows.iter().map(|r| r.name.as_str())),
        rows.iter()
            .map(|r| source_of(r).len())
            .max()
            .unwrap_or(0)
            .max("SOURCE".len()),
    );
    let header = format!(
        "{:<w0$}  {:<w1$}  {:<w2$}  {:>12}",
        "DATE",
        "NAME",
        "SOURCE",
        "AMOUNT",
        w0 = widths.0,
        w1 = widths.1,
        w2 = widths.2
    );
    println!("{}", bold(&header));
    for r in &rows {
        let date = paint(&format!("{:<w0$}", r.date, w0 = widths.0), Color::Dim);
        let amount = paint(&format!("{:>12}", cad(r.amount)), Color::Green);
        println!(
            "{date}  {:<w1$}  {:<w2$}  {amount}",
            r.name,
            source_of(r),
            w1 = widths.1,
            w2 = widths.2
        );
    }
    println!(
        "\n{} row(s), total {}",
        rows.len(),
        paint(&cad(total), Color::Green)
    );
    Ok(())
}

fn investments(sb: &Supabase) -> Result<()> {
    let mut rows: Vec<Investment> = sb.select_all(
        "investments",
        &format!("select=*&user_id=eq.{}", sb.user_id()),
    )?;
    rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    if rows.is_empty() {
        println!("No investments found.");
        return Ok(());
    }
    let total: f64 = rows.iter().map(Investment::cad_value).sum();
    let cost_total: f64 = rows.iter().map(|i| i.total_cost * i.exchange_rate).sum();
    let widths = (
        col_width("NAME", rows.iter().map(|r| r.name.as_str())),
        col_width("TYPE", rows.iter().map(|r| r.kind.as_str())),
    );
    let header = format!(
        "{:<w0$}  {:<w1$}  {:<4}  {:>14}",
        "NAME",
        "TYPE",
        "CCY",
        "VALUE (CAD)",
        w0 = widths.0,
        w1 = widths.1
    );
    println!("{}", bold(&header));
    for r in &rows {
        let value = paint(&format!("{:>14}", cad(r.cad_value())), Color::Cyan);
        println!(
            "{:<w0$}  {:<w1$}  {:<4}  {value}",
            r.name,
            r.kind,
            r.currency,
            w0 = widths.0,
            w1 = widths.1
        );
    }
    let gain = total - cost_total;
    println!("\n{} holding(s)", rows.len());
    println!("Total cost   : {}", cad(cost_total));
    println!("Market value : {}", paint(&cad(total), Color::Cyan));
    println!("Gain/loss    : {}", paint_signed(&cad(gain), gain));
    Ok(())
}

/// Format a value as CAD currency with thousands separators.
pub(crate) fn cad(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_string();
    }
    let negative = value < 0.0;
    let cents = format!("{:.2}", value.abs());
    let (int_part, frac_part) = cents.split_once('.').unwrap_or((cents.as_str(), "00"));
    let mut grouped = String::new();
    for (i, ch) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!(
        "{}CA${grouped}.{frac_part}",
        if negative { "-" } else { "" }
    )
}

fn col_width<'a>(header: &str, values: impl Iterator<Item = &'a str>) -> usize {
    values.map(str::len).max().unwrap_or(0).max(header.len())
}

/// Deserialize an `f64` that PostgREST may encode as either a number or a string
/// (numeric columns are sometimes serialized as strings).
pub(crate) fn de_num<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    use serde::de::Error;
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Number(n) => n.as_f64().ok_or_else(|| Error::custom("invalid number")),
        serde_json::Value::String(s) => s.parse().map_err(Error::custom),
        serde_json::Value::Null => Ok(0.0),
        other => Err(Error::custom(format!("expected number, got {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_rows_deserialize() {
        // status/summary request only a subset of columns; numbers may be strings.
        let e: Expense =
            serde_json::from_str(r#"{"category":"Food","amount":"12.50","date":"2026-06-01"}"#)
                .unwrap();
        assert_eq!(e.amount, 12.5);
        assert_eq!(e.name, "");
        let i: Income = serde_json::from_str(r#"{"amount":100,"date":"2026-06-01"}"#).unwrap();
        assert_eq!(i.amount, 100.0);
        assert!(i.source.is_none());
    }

    #[test]
    fn bucket_sum_groups_by_month() {
        let rows = [
            ("2026-06-01", 10.0),
            ("2026-06-30", 5.0),
            ("2026-05-15", 2.0),
        ];
        let got = bucket_sum(&rows, |r| (r.0, r.1));
        assert_eq!(got.get("2026-06"), Some(&15.0));
        assert_eq!(got.get("2026-05"), Some(&2.0));
    }

    #[test]
    fn cash_flow_carries_forward_through_gaps() {
        let incomes = vec![Income {
            name: String::new(),
            source: None,
            amount: 100.0,
            date: "2026-04-10".to_string(),
        }];
        let expenses = vec![Expense {
            name: String::new(),
            category: String::new(),
            amount: 30.0,
            date: "2026-05-10".to_string(),
        }];
        let months = vec![
            "2026-04".to_string(),
            "2026-05".to_string(),
            "2026-06".to_string(),
        ];
        // 100 in April, -30 in May, nothing in June (carry the 70 forward).
        assert_eq!(
            cumulative_cash_flow(&incomes, &expenses, &months),
            vec![100.0, 70.0, 70.0]
        );
    }

    #[test]
    fn category_totals_sorted_desc() {
        let rows = [("a", 1.0), ("b", 5.0), ("a", 2.0)];
        let got = category_totals(rows.iter(), |r| (r.0, r.1));
        assert_eq!(got, vec![("b".to_string(), 5.0), ("a".to_string(), 3.0)]);
    }
}
