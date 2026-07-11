//! `sam budget tui` — interactive editor for expenses, income, and
//! investments (including per-investment snapshot history), mirroring the
//! Budget web app's add/edit/delete features and validation rules.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Cell, Clear, Paragraph, Row, Table, TableState, Tabs};
use ratatui::{DefaultTerminal, Frame};
use serde::{Deserialize, Serialize};

use super::budget::{cad, de_num};
use crate::supabase::Supabase;

const TAB_TITLES: [&str; 3] = ["Expenses", "Income", "Investments"];
const INVESTMENTS_TAB: usize = 2;

/// Distinguishes the two entry tables wherever their editing model differs
/// (payload variant, required fields, form labels).
#[derive(Clone, Copy)]
enum EntryKind {
    Expense,
    Income,
}

/// Per-tab configuration for the two row-entry tabs. Expenses and income share
/// the same editing model except for the grouping column: a required
/// `category` vs an optional `source`. Both are fetched under the `group_name`
/// alias so one row type serves both tables. The investments tab has its own
/// row shape and is handled separately.
struct Spec {
    table: &'static str,
    noun: &'static str,
    kind: EntryKind,
    group_col: &'static str,
    group_label: &'static str,
    amount_color: Color,
}

// A `static` (not `const`) so `&SPECS[i]` is a true `&'static Spec` that a
// form can carry around.
static SPECS: [Spec; 2] = [
    Spec {
        table: "expenses",
        noun: "expense",
        kind: EntryKind::Expense,
        group_col: "category",
        group_label: "Category",
        amount_color: Color::Red,
    },
    Spec {
        table: "incomes",
        noun: "income",
        kind: EntryKind::Income,
        group_col: "source",
        group_label: "Source",
        amount_color: Color::Green,
    },
];

fn noun(tab: usize) -> &'static str {
    match tab {
        INVESTMENTS_TAB => "investment",
        t => SPECS[t].noun,
    }
}

#[derive(Deserialize)]
struct Entry {
    id: String,
    name: String,
    #[serde(default)]
    group_name: Option<String>,
    #[serde(deserialize_with = "de_num")]
    amount: f64,
    date: String,
    #[serde(default)]
    notes: Option<String>,
    /// Only used as the newest-first sort tiebreaker.
    #[serde(default)]
    created_at: String,
}

#[derive(Deserialize)]
struct InvestmentRow {
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
    updated_at: String,
}

impl InvestmentRow {
    /// Market value converted to CAD, matching the web app's `cadValue`.
    fn cad_value(&self) -> f64 {
        self.total_market_value * self.exchange_rate
    }
}

#[derive(Deserialize)]
struct SnapshotRow {
    id: String,
    #[serde(deserialize_with = "de_num")]
    total_cost: f64,
    #[serde(deserialize_with = "de_num")]
    total_market_value: f64,
    #[serde(deserialize_with = "de_num")]
    exchange_rate: f64,
    recorded_at: String,
}

impl SnapshotRow {
    /// Market value converted to CAD, matching the web app's `snapshotCadValue`.
    fn cad_value(&self) -> f64 {
        self.total_market_value * self.exchange_rate
    }
}

/// Typed row payload for the two entry tables; the variant fixes which
/// grouping column is serialized, so an expense payload can't silently grow a
/// `source` column (or an income a `category`).
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum EntryPayload {
    Expense {
        name: String,
        category: String,
        amount: f64,
        date: String,
        notes: Option<String>,
    },
    Income {
        name: String,
        source: Option<String>,
        amount: f64,
        date: String,
        notes: Option<String>,
    },
}

#[derive(Debug, Serialize)]
struct InvestmentPayload {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    currency: String,
    total_cost: f64,
    total_market_value: f64,
    exchange_rate: f64,
    updated_at: String,
}

#[derive(Debug, Serialize)]
struct SnapshotPayload {
    total_cost: f64,
    total_market_value: f64,
    exchange_rate: f64,
    recorded_at: String,
}

/// Attaches the owning `user_id` column to a payload for inserts (updates
/// filter on it instead and never write it).
#[derive(Serialize)]
struct ForUser<'a, T: Serialize> {
    user_id: &'a str,
    #[serde(flatten)]
    payload: &'a T,
}

/// A snapshot insert additionally links its parent investment.
#[derive(Serialize)]
struct NewSnapshot<'a> {
    user_id: &'a str,
    investment_id: &'a str,
    #[serde(flatten)]
    payload: &'a SnapshotPayload,
}

/// The only column read back from `insert_returning`.
#[derive(Deserialize)]
struct InsertedRow {
    id: String,
}

/// The drill-in snapshot-history screen for one investment — the TUI
/// equivalent of expanding an investment row in the web app.
struct SnapshotsView {
    investment_id: String,
    investment_name: String,
    rows: Vec<SnapshotRow>,
    table_state: TableState,
}

impl SnapshotsView {
    fn selected(&self) -> Option<&SnapshotRow> {
        self.table_state
            .selected()
            .filter(|index| *index < self.rows.len())
            .map(|index| &self.rows[index])
    }
}

// Entry form field order: matches the web app's add form.
const NAME: usize = 0;
const GROUP: usize = 1;
const AMOUNT: usize = 2;
const DATE: usize = 3;
const NOTES: usize = 4;

// Investment form field order: matches the web app's add form.
const INV_NAME: usize = 0;
const INV_TYPE: usize = 1;
const INV_CURRENCY: usize = 2;
const INV_COST: usize = 3;
const INV_MARKET: usize = 4;
const INV_FX: usize = 5;
const INV_UPDATED: usize = 6;

// Snapshot form field order: matches the web app's backfill-snapshot form.
const SNAP_COST: usize = 0;
const SNAP_MARKET: usize = 1;
const SNAP_FX: usize = 2;
const SNAP_RECORDED: usize = 3;

const EXPENSE_LABELS: [&str; 5] = ["Name", "Category", "Amount (CAD)", "Date", "Notes"];
const INCOME_LABELS: [&str; 5] = ["Name", "Source", "Amount (CAD)", "Date", "Notes"];
const INVESTMENT_LABELS: [&str; 7] = [
    "Name",
    "Type",
    "Currency",
    "Total cost",
    "Market value",
    "FX → CAD",
    "Updated at",
];
const SNAPSHOT_LABELS: [&str; 4] = ["Total cost", "Market value", "FX → CAD", "Recorded at"];

/// What a form edits, with its field values. Each variant carries a
/// fixed-size array (so the payload builders get compile-time length checks)
/// plus whatever context submitting needs, making a form self-contained
/// instead of dependent on the tab/view it was opened from.
enum FormTarget {
    Entry(&'static Spec, [String; 5]),
    Investment([String; 7]),
    Snapshot {
        investment_id: String,
        fields: [String; 4],
    },
}

impl FormTarget {
    fn fields(&self) -> &[String] {
        match self {
            FormTarget::Entry(_, fields) => fields,
            FormTarget::Investment(fields) => fields,
            FormTarget::Snapshot { fields, .. } => fields,
        }
    }

    fn fields_mut(&mut self) -> &mut [String] {
        match self {
            FormTarget::Entry(_, fields) => fields,
            FormTarget::Investment(fields) => fields,
            FormTarget::Snapshot { fields, .. } => fields,
        }
    }

    fn labels(&self) -> &'static [&'static str] {
        match self {
            FormTarget::Entry(spec, _) => match spec.kind {
                EntryKind::Expense => &EXPENSE_LABELS,
                EntryKind::Income => &INCOME_LABELS,
            },
            FormTarget::Investment(_) => &INVESTMENT_LABELS,
            FormTarget::Snapshot { .. } => &SNAPSHOT_LABELS,
        }
    }

    fn noun(&self) -> &'static str {
        match self {
            FormTarget::Entry(spec, _) => spec.noun,
            FormTarget::Investment(_) => "investment",
            FormTarget::Snapshot { .. } => "snapshot",
        }
    }
}

struct Form {
    /// `None` when adding a new row, `Some(id)` when editing an existing one.
    editing_id: Option<String>,
    target: FormTarget,
    focus: usize,
    /// Cursor position (in chars) within the focused field.
    cursor: usize,
}

impl Form {
    fn add_entry(spec: &'static Spec) -> Self {
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        Self::new(
            None,
            FormTarget::Entry(
                spec,
                [
                    String::new(),
                    String::new(),
                    String::new(),
                    today,
                    String::new(),
                ],
            ),
        )
    }

    fn edit_entry(spec: &'static Spec, entry: &Entry) -> Self {
        Self::new(
            Some(entry.id.clone()),
            FormTarget::Entry(
                spec,
                [
                    entry.name.clone(),
                    entry.group_name.clone().unwrap_or_default(),
                    format!("{}", entry.amount),
                    entry.date.clone(),
                    entry.notes.clone().unwrap_or_default(),
                ],
            ),
        )
    }

    fn add_investment() -> Self {
        // Mirror the web app's empty draft: CAD with FX pinned to 1,
        // updated-at prefilled with the current local time.
        Self::new(
            None,
            FormTarget::Investment([
                String::new(),
                String::new(),
                "CAD".to_string(),
                String::new(),
                String::new(),
                "1".to_string(),
                Local::now().format("%Y-%m-%d %H:%M").to_string(),
            ]),
        )
    }

    fn edit_investment(inv: &InvestmentRow) -> Self {
        Self::new(
            Some(inv.id.clone()),
            FormTarget::Investment([
                inv.name.clone(),
                inv.kind.clone(),
                inv.currency.clone(),
                format!("{}", inv.total_cost),
                format!("{}", inv.total_market_value),
                format!("{}", inv.exchange_rate),
                iso_to_local_input(&inv.updated_at),
            ]),
        )
    }

    fn add_snapshot(investment_id: String) -> Self {
        // Mirror the web app's backfill-snapshot draft: FX 1, recorded-at now.
        Self::new(
            None,
            FormTarget::Snapshot {
                investment_id,
                fields: [
                    String::new(),
                    String::new(),
                    "1".to_string(),
                    Local::now().format("%Y-%m-%d %H:%M").to_string(),
                ],
            },
        )
    }

    fn edit_snapshot(investment_id: String, snapshot: &SnapshotRow) -> Self {
        Self::new(
            Some(snapshot.id.clone()),
            FormTarget::Snapshot {
                investment_id,
                fields: [
                    format!("{}", snapshot.total_cost),
                    format!("{}", snapshot.total_market_value),
                    format!("{}", snapshot.exchange_rate),
                    iso_to_local_input(&snapshot.recorded_at),
                ],
            },
        )
    }

    fn new(editing_id: Option<String>, target: FormTarget) -> Self {
        // Editing starts with the cursor at the end of the prefilled first
        // field; adding starts at the beginning of the empty one.
        let cursor = if editing_id.is_some() {
            target.fields()[0].chars().count()
        } else {
            0
        };
        Self {
            editing_id,
            target,
            focus: 0,
            cursor,
        }
    }

    fn focused_len(&self) -> usize {
        self.target.fields()[self.focus].chars().count()
    }

    fn set_focus(&mut self, focus: usize) {
        self.focus = focus;
        self.cursor = self.focused_len();
    }
}

enum Mode {
    List,
    Form(Form),
    ConfirmDelete { id: String },
}

struct App<'a> {
    sb: &'a Supabase,
    tab: usize,
    entries: [Vec<Entry>; 2],
    investments: Vec<InvestmentRow>,
    /// When set, the investments tab is drilled into one investment's
    /// snapshot history and list keys operate on snapshots instead.
    snapshots: Option<SnapshotsView>,
    table_state: [TableState; 3],
    mode: Mode,
    /// One-shot message shown in the status line: `(text, is_error)`.
    status: Option<(String, bool)>,
    quit: bool,
}

pub fn run(sb: &Supabase) -> Result<()> {
    // Fetch before entering the alternate screen so a network/auth failure
    // prints as a normal error instead of flashing an empty TUI.
    let mut app = App::new(sb)?;
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}

// The fetches use the paginating `select_all` (PostgREST silently caps plain
// selects at 1,000 rows) and sort in memory, since `select_all` owns the
// server-side ordering.

fn fetch_entries(sb: &Supabase, spec: &Spec) -> Result<Vec<Entry>> {
    let mut rows: Vec<Entry> = sb.select_all(
        spec.table,
        &format!(
            "select=id,name,group_name:{},amount,date,notes,created_at&user_id=eq.{}",
            spec.group_col,
            sb.user_id()
        ),
    )?;
    rows.sort_by(|a, b| {
        (b.date.as_str(), b.created_at.as_str()).cmp(&(a.date.as_str(), a.created_at.as_str()))
    });
    Ok(rows)
}

fn fetch_investments(sb: &Supabase) -> Result<Vec<InvestmentRow>> {
    let mut rows: Vec<InvestmentRow> = sb.select_all(
        "investments",
        &format!(
            "select=id,name,type,currency,total_cost,total_market_value,exchange_rate,updated_at&user_id=eq.{}",
            sb.user_id()
        ),
    )?;
    rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(rows)
}

fn fetch_snapshots(sb: &Supabase, investment_id: &str) -> Result<Vec<SnapshotRow>> {
    let mut rows: Vec<SnapshotRow> = sb.select_all(
        "investment_snapshots",
        &format!(
            "select=id,total_cost,total_market_value,exchange_rate,recorded_at&user_id=eq.{}&investment_id=eq.{investment_id}",
            sb.user_id()
        ),
    )?;
    rows.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
    Ok(rows)
}

impl<'a> App<'a> {
    fn new(sb: &'a Supabase) -> Result<Self> {
        let entries = [fetch_entries(sb, &SPECS[0])?, fetch_entries(sb, &SPECS[1])?];
        let investments = fetch_investments(sb)?;
        let mut app = Self {
            sb,
            tab: 0,
            entries,
            investments,
            snapshots: None,
            table_state: [
                TableState::default(),
                TableState::default(),
                TableState::default(),
            ],
            mode: Mode::List,
            status: None,
            quit: false,
        };
        for tab in 0..TAB_TITLES.len() {
            app.clamp_selection(tab);
        }
        Ok(app)
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.on_key(key);
                }
            }
        }
        Ok(())
    }

    fn tab_len(&self, tab: usize) -> usize {
        match tab {
            INVESTMENTS_TAB => self.investments.len(),
            t => self.entries[t].len(),
        }
    }

    // --- Event handling -----------------------------------------------------

    fn on_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        // Status messages are one-shot: any keypress dismisses them.
        self.status = None;
        match self.mode {
            Mode::List if self.snapshots.is_some() => self.on_snapshots_key(key),
            Mode::List => self.on_list_key(key),
            Mode::Form(_) => self.on_form_key(key),
            Mode::ConfirmDelete { .. } => self.on_confirm_key(key),
        }
    }

    fn on_list_key(&mut self, key: KeyEvent) {
        let tab_count = TAB_TITLES.len();
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Tab | KeyCode::Right => self.tab = (self.tab + 1) % tab_count,
            KeyCode::BackTab | KeyCode::Left => {
                self.tab = (self.tab + tab_count - 1) % tab_count;
            }
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('3') => self.tab = INVESTMENTS_TAB,
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Char('r') => {
                for tab in 0..tab_count {
                    self.refresh(tab);
                }
                if self.status.is_none() {
                    self.status = Some(("Refreshed.".to_string(), false));
                }
            }
            KeyCode::Char('a') => {
                let form = if self.tab == INVESTMENTS_TAB {
                    Form::add_investment()
                } else {
                    Form::add_entry(&SPECS[self.tab])
                };
                self.mode = Mode::Form(form);
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(index) = self.selected_index() {
                    let form = if self.tab == INVESTMENTS_TAB {
                        Form::edit_investment(&self.investments[index])
                    } else {
                        Form::edit_entry(&SPECS[self.tab], &self.entries[self.tab][index])
                    };
                    self.mode = Mode::Form(form);
                }
            }
            KeyCode::Char('d') => {
                if let Some(index) = self.selected_index() {
                    let id = if self.tab == INVESTMENTS_TAB {
                        self.investments[index].id.clone()
                    } else {
                        self.entries[self.tab][index].id.clone()
                    };
                    self.mode = Mode::ConfirmDelete { id };
                }
            }
            KeyCode::Char('s') if self.tab == INVESTMENTS_TAB => {
                if let Some(index) = self.selected_index() {
                    let inv = &self.investments[index];
                    match fetch_snapshots(self.sb, &inv.id) {
                        Ok(rows) => {
                            let mut table_state = TableState::default();
                            if !rows.is_empty() {
                                table_state.select(Some(0));
                            }
                            self.snapshots = Some(SnapshotsView {
                                investment_id: inv.id.clone(),
                                investment_name: inv.name.clone(),
                                rows,
                                table_state,
                            });
                        }
                        Err(err) => self.status = Some((err.to_string(), true)),
                    }
                }
            }
            _ => {}
        }
    }

    /// List keys while drilled into an investment's snapshot history.
    fn on_snapshots_key(&mut self, key: KeyEvent) {
        let Some(view) = &mut self.snapshots else {
            return;
        };
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.snapshots = None,
            KeyCode::Up | KeyCode::Char('k') => step(&mut view.table_state, view.rows.len(), -1),
            KeyCode::Down | KeyCode::Char('j') => step(&mut view.table_state, view.rows.len(), 1),
            KeyCode::Char('r') => self.refresh_snapshots(),
            KeyCode::Char('a') => {
                self.mode = Mode::Form(Form::add_snapshot(view.investment_id.clone()));
            }
            KeyCode::Char('e') | KeyCode::Enter => {
                if let Some(snapshot) = view.selected() {
                    self.mode =
                        Mode::Form(Form::edit_snapshot(view.investment_id.clone(), snapshot));
                }
            }
            KeyCode::Char('d') => {
                if let Some(snapshot) = view.selected() {
                    self.mode = Mode::ConfirmDelete {
                        id: snapshot.id.clone(),
                    };
                }
            }
            _ => {}
        }
    }

    fn on_form_key(&mut self, key: KeyEvent) {
        let Mode::Form(form) = &mut self.mode else {
            return;
        };
        let field_count = form.target.fields().len();
        match key.code {
            KeyCode::Esc => self.mode = Mode::List,
            KeyCode::Enter => self.submit_form(),
            KeyCode::Tab | KeyCode::Down => form.set_focus((form.focus + 1) % field_count),
            KeyCode::BackTab | KeyCode::Up => {
                form.set_focus((form.focus + field_count - 1) % field_count);
            }
            KeyCode::Left => form.cursor = form.cursor.saturating_sub(1),
            KeyCode::Right => form.cursor = (form.cursor + 1).min(form.focused_len()),
            KeyCode::Home => form.cursor = 0,
            KeyCode::End => form.cursor = form.focused_len(),
            KeyCode::Backspace if form.cursor > 0 => {
                form.cursor -= 1;
                let value = &mut form.target.fields_mut()[form.focus];
                let at = byte_index(value, form.cursor);
                value.remove(at);
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let value = &mut form.target.fields_mut()[form.focus];
                let at = byte_index(value, form.cursor);
                value.insert(at, c);
                form.cursor += 1;
            }
            _ => {}
        }
    }

    fn on_confirm_key(&mut self, key: KeyEvent) {
        let Mode::ConfirmDelete { id } = &self.mode else {
            return;
        };
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let id = id.clone();
                let (table, deleted_noun) = if self.snapshots.is_some() {
                    ("investment_snapshots", "snapshot")
                } else {
                    match self.tab {
                        INVESTMENTS_TAB => ("investments", "investment"),
                        t => (SPECS[t].table, SPECS[t].noun),
                    }
                };
                let query = format!("id=eq.{id}&user_id=eq.{}", self.sb.user_id());
                match self.sb.delete(table, &query) {
                    Ok(()) => {
                        self.status = Some((format!("Deleted {deleted_noun}."), false));
                        if self.snapshots.is_some() {
                            self.refresh_snapshots();
                        } else {
                            self.refresh(self.tab);
                        }
                    }
                    Err(err) => self.status = Some((err.to_string(), true)),
                }
                self.mode = Mode::List;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => self.mode = Mode::List,
            _ => {}
        }
    }

    // --- Mutations ------------------------------------------------------------

    fn submit_form(&mut self) {
        let Mode::Form(form) = std::mem::replace(&mut self.mode, Mode::List) else {
            return;
        };
        // The form knows what it edits; no ambient tab/view state involved.
        match form.target {
            FormTarget::Entry(..) => self.submit_entry(form),
            FormTarget::Investment(_) => self.submit_investment(form),
            FormTarget::Snapshot { .. } => self.submit_snapshot(form),
        }
    }

    fn submit_entry(&mut self, form: Form) {
        let (spec, payload) = {
            let FormTarget::Entry(spec, fields) = &form.target else {
                return;
            };
            (*spec, build_entry_payload(spec, fields))
        };
        let payload = match payload {
            Ok(payload) => payload,
            Err(message) => {
                self.status = Some((message, true));
                self.mode = Mode::Form(form);
                return;
            }
        };
        let result = match &form.editing_id {
            Some(id) => {
                let query = format!("id=eq.{id}&user_id=eq.{}", self.sb.user_id());
                self.sb.update(spec.table, &query, &payload)
            }
            None => self.sb.insert(
                spec.table,
                &ForUser {
                    user_id: self.sb.user_id(),
                    payload: &payload,
                },
                None,
            ),
        };
        match result {
            Ok(()) => {
                let verb = if form.editing_id.is_some() {
                    "Updated"
                } else {
                    "Added"
                };
                self.status = Some((format!("{verb} {}.", spec.noun), false));
                self.refresh(self.tab);
            }
            Err(err) => {
                // Keep the form (with the user's input) open on API failure.
                self.status = Some((err.to_string(), true));
                self.mode = Mode::Form(form);
            }
        }
    }

    fn submit_investment(&mut self, form: Form) {
        let payload = {
            let FormTarget::Investment(fields) = &form.target else {
                return;
            };
            build_investment_payload(fields)
        };
        let payload = match payload {
            Ok(payload) => payload,
            Err(message) => {
                self.status = Some((message, true));
                self.mode = Mode::Form(form);
                return;
            }
        };
        let result: Result<String> = match &form.editing_id {
            Some(id) => {
                let query = format!("id=eq.{id}&user_id=eq.{}", self.sb.user_id());
                self.sb
                    .update("investments", &query, &payload)
                    .map(|()| id.clone())
            }
            None => self
                .sb
                .insert_returning::<_, Vec<InsertedRow>>(
                    "investments",
                    &ForUser {
                        user_id: self.sb.user_id(),
                        payload: &payload,
                    },
                )
                .and_then(|rows| {
                    rows.into_iter()
                        .next()
                        .map(|row| row.id)
                        .ok_or_else(|| anyhow!("insert returned no id"))
                }),
        };
        match result {
            Ok(id) => {
                // Mirror the web app: every add/edit also records a snapshot,
                // which feeds the value-over-time chart.
                let snapshot = SnapshotPayload {
                    total_cost: payload.total_cost,
                    total_market_value: payload.total_market_value,
                    exchange_rate: payload.exchange_rate,
                    recorded_at: payload.updated_at.clone(),
                };
                let insert = NewSnapshot {
                    user_id: self.sb.user_id(),
                    investment_id: &id,
                    payload: &snapshot,
                };
                let verb = if form.editing_id.is_some() {
                    "Updated"
                } else {
                    "Added"
                };
                self.status = match self.sb.insert("investment_snapshots", &insert, None) {
                    Ok(()) => Some((format!("{verb} investment."), false)),
                    Err(err) => Some((
                        format!("{verb} investment, but snapshot failed: {err}"),
                        true,
                    )),
                };
                self.refresh(INVESTMENTS_TAB);
            }
            Err(err) => {
                // Keep the form (with the user's input) open on API failure.
                self.status = Some((err.to_string(), true));
                self.mode = Mode::Form(form);
            }
        }
    }

    fn submit_snapshot(&mut self, form: Form) {
        let (investment_id, payload) = {
            let FormTarget::Snapshot {
                investment_id,
                fields,
            } = &form.target
            else {
                return;
            };
            (investment_id.clone(), build_snapshot_payload(fields))
        };
        let payload = match payload {
            Ok(payload) => payload,
            Err(message) => {
                self.status = Some((message, true));
                self.mode = Mode::Form(form);
                return;
            }
        };
        let result = match &form.editing_id {
            Some(id) => {
                let query = format!("id=eq.{id}&user_id=eq.{}", self.sb.user_id());
                self.sb.update("investment_snapshots", &query, &payload)
            }
            None => self.sb.insert(
                "investment_snapshots",
                &NewSnapshot {
                    user_id: self.sb.user_id(),
                    investment_id: &investment_id,
                    payload: &payload,
                },
                None,
            ),
        };
        match result {
            Ok(()) => {
                let verb = if form.editing_id.is_some() {
                    "Updated"
                } else {
                    "Added"
                };
                self.status = Some((format!("{verb} snapshot."), false));
                self.refresh_snapshots();
            }
            Err(err) => {
                // Keep the form (with the user's input) open on API failure.
                self.status = Some((err.to_string(), true));
                self.mode = Mode::Form(form);
            }
        }
    }

    fn refresh(&mut self, tab: usize) {
        let result = match tab {
            INVESTMENTS_TAB => fetch_investments(self.sb).map(|rows| self.investments = rows),
            t => fetch_entries(self.sb, &SPECS[t]).map(|rows| self.entries[t] = rows),
        };
        if let Err(err) = result {
            self.status = Some((err.to_string(), true));
        }
        self.clamp_selection(tab);
    }

    fn refresh_snapshots(&mut self) {
        let Some(view) = &mut self.snapshots else {
            return;
        };
        match fetch_snapshots(self.sb, &view.investment_id) {
            Ok(rows) => view.rows = rows,
            Err(err) => self.status = Some((err.to_string(), true)),
        }
        let len = view.rows.len();
        let selected = (len > 0).then(|| view.table_state.selected().unwrap_or(0).min(len - 1));
        view.table_state.select(selected);
    }

    // --- Selection --------------------------------------------------------------

    fn clamp_selection(&mut self, tab: usize) {
        let len = self.tab_len(tab);
        let selected =
            (len > 0).then(|| self.table_state[tab].selected().unwrap_or(0).min(len - 1));
        self.table_state[tab].select(selected);
    }

    fn selected_index(&self) -> Option<usize> {
        self.table_state[self.tab]
            .selected()
            .filter(|index| *index < self.tab_len(self.tab))
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.tab_len(self.tab);
        step(&mut self.table_state[self.tab], len, delta);
    }

    // --- Rendering ----------------------------------------------------------------

    fn draw(&mut self, frame: &mut Frame) {
        let [tabs_area, table_area, status_area, help_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        if let Some(view) = &self.snapshots {
            let breadcrumb = Line::from(vec![
                Span::styled("Investments", Style::new().fg(Color::DarkGray)),
                Span::raw(" ▸ "),
                Span::styled(
                    view.investment_name.clone(),
                    Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" — snapshot history", Style::new().fg(Color::DarkGray)),
            ]);
            frame.render_widget(Paragraph::new(breadcrumb), tabs_area);
        } else {
            let tabs = Tabs::new(TAB_TITLES)
                .select(self.tab)
                .highlight_style(Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD));
            frame.render_widget(tabs, tabs_area);
        }

        if let Some(view) = &self.snapshots {
            if view.rows.is_empty() {
                frame.render_widget(
                    Paragraph::new("No snapshots yet — press a to add one.")
                        .style(Style::new().fg(Color::DarkGray)),
                    table_area,
                );
            } else {
                self.draw_snapshots_table(frame, table_area);
            }
        } else if self.tab_len(self.tab) == 0 {
            frame.render_widget(
                Paragraph::new("No entries yet — press a to add one.")
                    .style(Style::new().fg(Color::DarkGray)),
                table_area,
            );
        } else if self.tab == INVESTMENTS_TAB {
            self.draw_investments_table(frame, table_area);
        } else {
            self.draw_entries_table(frame, table_area);
        }
        self.draw_status(frame, status_area);
        frame.render_widget(
            Paragraph::new(self.help_text()).style(Style::new().fg(Color::DarkGray)),
            help_area,
        );

        match &self.mode {
            Mode::List => {}
            Mode::Form(form) => draw_form(frame, form),
            Mode::ConfirmDelete { .. } => {
                draw_confirm(frame, self.tab, self.snapshots.is_some());
            }
        }
    }

    fn draw_entries_table(&mut self, frame: &mut Frame, area: Rect) {
        let spec = &SPECS[self.tab];
        let dim = Style::new().fg(Color::DarkGray);
        let rows = self.entries[self.tab].iter().map(|entry| {
            Row::new(vec![
                Cell::from(entry.date.clone()).style(dim),
                Cell::from(entry.name.clone()),
                Cell::from(entry.group_name.clone().unwrap_or_default()),
                Cell::from(Line::from(cad(entry.amount)).right_aligned())
                    .style(Style::new().fg(spec.amount_color)),
                Cell::from(entry.notes.clone().unwrap_or_default()).style(dim),
            ])
        });
        let header = Row::new(vec![
            Cell::from("DATE"),
            Cell::from("NAME"),
            Cell::from(spec.group_label.to_uppercase()),
            Cell::from(Line::from("AMOUNT").right_aligned()),
            Cell::from("NOTES"),
        ])
        .style(Style::new().add_modifier(Modifier::BOLD));
        let table = Table::new(
            rows,
            [
                Constraint::Length(10),
                Constraint::Fill(3),
                Constraint::Fill(2),
                Constraint::Length(13),
                Constraint::Fill(3),
            ],
        )
        .header(header)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(table, area, &mut self.table_state[self.tab]);
    }

    fn draw_investments_table(&mut self, frame: &mut Frame, area: Rect) {
        let dim = Style::new().fg(Color::DarkGray);
        let right = |text: String| Cell::from(Line::from(text).right_aligned());
        let rows = self.investments.iter().map(|inv| {
            Row::new(vec![
                Cell::from(inv.name.clone()),
                Cell::from(inv.kind.clone()),
                Cell::from(inv.currency.clone()),
                right(thousands(inv.total_cost)),
                right(thousands(inv.total_market_value)),
                right(format!("{}", inv.exchange_rate)),
                right(cad(inv.cad_value())).style(Style::new().fg(Color::Cyan)),
                Cell::from(iso_to_local_input(&inv.updated_at)).style(dim),
            ])
        });
        let header = Row::new(vec![
            Cell::from("NAME"),
            Cell::from("TYPE"),
            Cell::from("CCY"),
            right("COST".to_string()),
            right("MARKET".to_string()),
            right("FX".to_string()),
            right("VALUE (CAD)".to_string()),
            Cell::from("UPDATED"),
        ])
        .style(Style::new().add_modifier(Modifier::BOLD));
        let table = Table::new(
            rows,
            [
                Constraint::Fill(3),
                Constraint::Fill(2),
                Constraint::Length(3),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(7),
                Constraint::Length(13),
                Constraint::Length(16),
            ],
        )
        .header(header)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(table, area, &mut self.table_state[INVESTMENTS_TAB]);
    }

    fn draw_snapshots_table(&mut self, frame: &mut Frame, area: Rect) {
        let Some(view) = &mut self.snapshots else {
            return;
        };
        let dim = Style::new().fg(Color::DarkGray);
        let right = |text: String| Cell::from(Line::from(text).right_aligned());
        let rows = view.rows.iter().map(|snapshot| {
            Row::new(vec![
                Cell::from(iso_to_local_input(&snapshot.recorded_at)).style(dim),
                right(thousands(snapshot.total_cost)),
                right(thousands(snapshot.total_market_value)),
                right(format!("{}", snapshot.exchange_rate)),
                right(cad(snapshot.cad_value())).style(Style::new().fg(Color::Cyan)),
            ])
        });
        let header = Row::new(vec![
            Cell::from("RECORDED AT"),
            right("COST".to_string()),
            right("MARKET".to_string()),
            right("FX".to_string()),
            right("VALUE (CAD)".to_string()),
        ])
        .style(Style::new().add_modifier(Modifier::BOLD));
        let table = Table::new(
            rows,
            [
                Constraint::Length(16),
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Length(9),
                Constraint::Fill(1),
            ],
        )
        .header(header)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        frame.render_stateful_widget(table, area, &mut view.table_state);
    }

    fn draw_status(&self, frame: &mut Frame, area: Rect) {
        let (text, style) = match &self.status {
            Some((message, true)) => (message.clone(), Style::new().fg(Color::Red)),
            Some((message, false)) => (message.clone(), Style::new().fg(Color::Green)),
            None if self.snapshots.is_some() => {
                let view = self.snapshots.as_ref().expect("checked above");
                (
                    format!("{} snapshot(s)", view.rows.len()),
                    Style::new().fg(Color::DarkGray),
                )
            }
            None if self.tab == INVESTMENTS_TAB => {
                let total: f64 = self.investments.iter().map(InvestmentRow::cad_value).sum();
                (
                    format!(
                        "{} holding(s) · market value {}",
                        self.investments.len(),
                        cad(total)
                    ),
                    Style::new().fg(Color::DarkGray),
                )
            }
            None => {
                let rows = &self.entries[self.tab];
                let total: f64 = rows.iter().map(|e| e.amount).sum();
                (
                    format!("{} row(s) · total {}", rows.len(), cad(total)),
                    Style::new().fg(Color::DarkGray),
                )
            }
        };
        frame.render_widget(Paragraph::new(text).style(style), area);
    }

    fn help_text(&self) -> &'static str {
        match self.mode {
            Mode::List if self.snapshots.is_some() => {
                "↑/↓ select · a add · e/enter edit · d delete · r refresh · esc back"
            }
            Mode::List if self.tab == INVESTMENTS_TAB => {
                "↑/↓ select · a add · e/enter edit · d delete · s snapshots · tab switch · r refresh · q quit"
            }
            Mode::List => {
                "↑/↓ select · a add · e/enter edit · d delete · tab switch · r refresh · q quit"
            }
            Mode::Form(_) => "enter save · esc cancel · tab/↑/↓ field · ←/→ move cursor",
            Mode::ConfirmDelete { .. } => "y delete · n/esc cancel",
        }
    }
}

const FORM_LABEL_WIDTH: u16 = 14;

fn draw_form(frame: &mut Frame, form: &Form) {
    let labels = form.target.labels();
    let area = centered(frame.area(), 64, labels.len() as u16 + 2);
    frame.render_widget(Clear, area);
    let action = if form.editing_id.is_some() {
        "Edit"
    } else {
        "Add"
    };
    let block = Block::bordered().title(format!(" {action} {} ", form.target.noun()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let field_areas =
        Layout::vertical(std::iter::repeat_n(Constraint::Length(1), labels.len())).split(inner);
    let available = inner.width.saturating_sub(FORM_LABEL_WIDTH) as usize;
    for (i, label) in labels.iter().enumerate() {
        let focused = i == form.focus;
        // Keep the cursor visible when a value is wider than the field.
        let skip = if focused {
            form.cursor.saturating_sub(available.saturating_sub(1))
        } else {
            0
        };
        let value: String = form.target.fields()[i]
            .chars()
            .skip(skip)
            .take(available)
            .collect();
        let label_style = if focused {
            Style::new().fg(Color::Blue).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(Color::DarkGray)
        };
        let line = Line::from(vec![
            Span::styled(
                format!("{label:<width$}", width = FORM_LABEL_WIDTH as usize),
                label_style,
            ),
            Span::raw(value),
        ]);
        frame.render_widget(Paragraph::new(line), field_areas[i]);
        if focused {
            frame.set_cursor_position(Position::new(
                field_areas[i].x + FORM_LABEL_WIDTH + (form.cursor - skip) as u16,
                field_areas[i].y,
            ));
        }
    }
}

fn draw_confirm(frame: &mut Frame, tab: usize, snapshots_view: bool) {
    // Investments cascade-delete their snapshots; say so, like the web app.
    let message = if snapshots_view {
        "Delete this snapshot? (y/n)".to_string()
    } else if tab == INVESTMENTS_TAB {
        "Delete this investment and all its snapshots? (y/n)".to_string()
    } else {
        format!("Delete this {}? (y/n)", noun(tab))
    };
    let area = centered(frame.area(), message.chars().count() as u16 + 4, 3);
    frame.render_widget(Clear, area);
    let block = Block::bordered().title(" Confirm ");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(message).centered(), inner);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// Move a table selection by `delta`, clamped to the row range.
fn step(state: &mut TableState, len: usize, delta: isize) {
    if len == 0 {
        return;
    }
    let current = state.selected().unwrap_or(0) as isize;
    state.select(Some((current + delta).clamp(0, len as isize - 1) as usize));
}

fn byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Validate the entry form and build its typed row payload, mirroring the web
/// app's rules: name and date required, amount ≥ 0, category required for
/// expenses, source optional for income, blank notes stored as null.
fn build_entry_payload(spec: &Spec, values: &[String; 5]) -> Result<EntryPayload, String> {
    let name = values[NAME].trim();
    let group = values[GROUP].trim();
    let date = values[DATE].trim();
    let group_required = matches!(spec.kind, EntryKind::Expense);
    let required = || {
        if group_required {
            "Name, category, amount (≥ 0), and date are required.".to_string()
        } else {
            "Name, amount (≥ 0), and date are required.".to_string()
        }
    };
    if name.is_empty() || (group_required && group.is_empty()) || date.is_empty() {
        return Err(required());
    }
    let Some(amount) = parse_amount(&values[AMOUNT]) else {
        return Err(required());
    };
    if NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
        return Err("Date must be a valid YYYY-MM-DD.".to_string());
    }
    let notes = values[NOTES].trim();
    let notes = (!notes.is_empty()).then(|| notes.to_string());
    Ok(match spec.kind {
        EntryKind::Expense => EntryPayload::Expense {
            name: name.to_string(),
            category: group.to_string(),
            amount,
            date: date.to_string(),
            notes,
        },
        EntryKind::Income => EntryPayload::Income {
            name: name.to_string(),
            source: (!group.is_empty()).then(|| group.to_string()),
            amount,
            date: date.to_string(),
            notes,
        },
    })
}

/// Validate the investment form and build its typed row payload, mirroring
/// the web app's `validateDraft`: name and type required, cost and market
/// value ≥ 0, currency CAD or USD, FX pinned to 1 for CAD and > 0 for USD,
/// and a valid local updated-at timestamp stored as UTC.
fn build_investment_payload(values: &[String; 7]) -> Result<InvestmentPayload, String> {
    let name = values[INV_NAME].trim();
    if name.is_empty() {
        return Err("Name is required.".to_string());
    }
    let kind = values[INV_TYPE].trim();
    if kind.is_empty() {
        return Err("Type is required.".to_string());
    }
    let currency = values[INV_CURRENCY].trim().to_uppercase();
    if currency != "CAD" && currency != "USD" {
        return Err("Currency must be CAD or USD.".to_string());
    }
    let Some(cost) = parse_amount(&values[INV_COST]) else {
        return Err("Total cost must be ≥ 0.".to_string());
    };
    let Some(market_value) = parse_amount(&values[INV_MARKET]) else {
        return Err("Total market value must be ≥ 0.".to_string());
    };
    let exchange_rate = if currency == "CAD" {
        1.0
    } else {
        match parse_amount(&values[INV_FX]).filter(|rate| *rate > 0.0) {
            Some(rate) => rate,
            None => return Err("Exchange rate must be > 0.".to_string()),
        }
    };
    let Some(updated_at) = parse_local_datetime(&values[INV_UPDATED]) else {
        return Err("Updated at must be a valid YYYY-MM-DD HH:MM.".to_string());
    };
    Ok(InvestmentPayload {
        name: name.to_string(),
        kind: kind.to_string(),
        currency,
        total_cost: cost,
        total_market_value: market_value,
        exchange_rate,
        updated_at,
    })
}

/// Validate the snapshot form and build its typed row payload, mirroring the
/// web app's snapshot `validate`: cost and market value ≥ 0, FX > 0, and a
/// valid local recorded-at timestamp stored as UTC.
fn build_snapshot_payload(values: &[String; 4]) -> Result<SnapshotPayload, String> {
    let Some(cost) = parse_amount(&values[SNAP_COST]) else {
        return Err("Total cost must be ≥ 0.".to_string());
    };
    let Some(market_value) = parse_amount(&values[SNAP_MARKET]) else {
        return Err("Total market value must be ≥ 0.".to_string());
    };
    let Some(exchange_rate) = parse_amount(&values[SNAP_FX]).filter(|rate| *rate > 0.0) else {
        return Err("Exchange rate must be > 0.".to_string());
    };
    let Some(recorded_at) = parse_local_datetime(&values[SNAP_RECORDED]) else {
        return Err("Recorded at must be a valid YYYY-MM-DD HH:MM.".to_string());
    };
    Ok(SnapshotPayload {
        total_cost: cost,
        total_market_value: market_value,
        exchange_rate,
        recorded_at,
    })
}

/// The web app's `parseNonNegFloat`: strip commas, require a finite value ≥ 0.
fn parse_amount(raw: &str) -> Option<f64> {
    let cleaned = raw.trim().replace(',', "");
    if cleaned.is_empty() {
        return None;
    }
    let n: f64 = cleaned.parse().ok()?;
    (n.is_finite() && n >= 0.0).then_some(n)
}

/// Parse a local timestamp ("YYYY-MM-DD HH:MM", with a "T" separator or
/// seconds also accepted, or a bare date meaning midnight) into a UTC RFC 3339
/// string — the equivalent of the web app's `localInputValueToISO`.
fn parse_local_datetime(raw: &str) -> Option<String> {
    let raw = raw.trim().replace('T', " ");
    let naive = NaiveDateTime::parse_from_str(&raw, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(&raw, "%Y-%m-%d %H:%M"))
        .ok()
        .or_else(|| {
            NaiveDate::parse_from_str(&raw, "%Y-%m-%d")
                .ok()
                .and_then(|date| date.and_hms_opt(0, 0, 0))
        })?;
    let local = Local.from_local_datetime(&naive).earliest()?;
    Some(local.with_timezone(&Utc).to_rfc3339())
}

/// Render a stored UTC timestamp in local time as "YYYY-MM-DD HH:MM", the
/// equivalent of the web app's `isoToLocalInputValue`.
fn iso_to_local_input(iso: &str) -> String {
    DateTime::parse_from_rfc3339(iso)
        .map(|dt| {
            dt.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| iso.to_string())
}

/// Format a plain number with thousands separators (the web app's
/// `toLocaleString` for cost/market-value cells).
fn thousands(value: f64) -> String {
    cad(value).replacen("CA$", "", 1)
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    fn values(name: &str, group: &str, amount: &str, date: &str, notes: &str) -> [String; 5] {
        [name, group, amount, date, notes].map(str::to_string)
    }

    /// Assertions run on the serialized form so the serde attributes
    /// (`untagged`, `rename`, `flatten`) are covered, not just the fields.
    fn to_json(payload: impl Serialize) -> Value {
        serde_json::to_value(payload).unwrap()
    }

    #[test]
    fn parse_amount_mirrors_web_rules() {
        assert_eq!(parse_amount("1,234.50"), Some(1234.5));
        assert_eq!(parse_amount(" 0 "), Some(0.0));
        assert_eq!(parse_amount("-1"), None);
        assert_eq!(parse_amount("abc"), None);
        assert_eq!(parse_amount(""), None);
        assert_eq!(parse_amount("inf"), None);
    }

    #[test]
    fn expense_payload_requires_category() {
        let spec = &SPECS[0];
        let err =
            build_entry_payload(spec, &values("Lunch", "", "12", "2026-07-01", "")).unwrap_err();
        assert!(err.contains("category"));
        let body = to_json(
            build_entry_payload(spec, &values("Lunch", "Food", "12", "2026-07-01", "")).unwrap(),
        );
        assert_eq!(body["category"], "Food");
        assert_eq!(body["amount"], 12.0);
        assert_eq!(body["notes"], Value::Null);
        // An expense payload can never carry a `source` column.
        assert!(body.get("source").is_none());
    }

    #[test]
    fn income_source_is_optional_and_blank_becomes_null() {
        let spec = &SPECS[1];
        let body = to_json(
            build_entry_payload(spec, &values("Salary", "", "5000", "2026-07-01", " hi ")).unwrap(),
        );
        assert_eq!(body["source"], Value::Null);
        assert_eq!(body["notes"], "hi");
        assert!(body.get("category").is_none());
    }

    #[test]
    fn invalid_date_is_rejected() {
        let spec = &SPECS[1];
        let err =
            build_entry_payload(spec, &values("Salary", "", "1", "2026-13-40", "")).unwrap_err();
        assert!(err.contains("YYYY-MM-DD"));
        assert!(build_entry_payload(spec, &values("Salary", "", "1", "07/01/2026", "")).is_err());
    }

    #[test]
    fn investment_cad_pins_exchange_rate_to_one() {
        let body = to_json(
            build_investment_payload(
                &[
                    "TFSA",
                    "Stocks",
                    "cad",
                    "10,000",
                    "11250",
                    "1.36",
                    "2026-07-01 09:00",
                ]
                .map(str::to_string),
            )
            .unwrap(),
        );
        assert_eq!(body["currency"], "CAD");
        assert_eq!(body["exchange_rate"], 1.0);
        assert_eq!(body["total_cost"], 10000.0);
        // The Rust field is `kind`; the column must serialize as `type`.
        assert_eq!(body["type"], "Stocks");
        assert!(body.get("kind").is_none());
        // The stored timestamp is UTC RFC 3339.
        assert!(DateTime::parse_from_rfc3339(body["updated_at"].as_str().unwrap()).is_ok());
    }

    #[test]
    fn investment_usd_requires_positive_exchange_rate() {
        let base = |fx: &str| {
            build_investment_payload(
                &[
                    "US Index",
                    "ETF",
                    "USD",
                    "5000",
                    "5400",
                    fx,
                    "2026-07-01T09:00",
                ]
                .map(str::to_string),
            )
        };
        assert_eq!(base("0").unwrap_err(), "Exchange rate must be > 0.");
        assert_eq!(base("").unwrap_err(), "Exchange rate must be > 0.");
        assert_eq!(base("1.36").unwrap().exchange_rate, 1.36);
    }

    #[test]
    fn investment_validation_errors() {
        let ok = ["A", "ETF", "CAD", "1", "1", "1", "2026-07-01 09:00"];
        let with = |i: usize, v: &str| {
            let mut fields = ok;
            fields[i] = v;
            build_investment_payload(&fields.map(str::to_string))
        };
        assert!(with(INV_NAME, " ").is_err());
        assert!(with(INV_TYPE, "").is_err());
        assert_eq!(
            with(INV_CURRENCY, "EUR").unwrap_err(),
            "Currency must be CAD or USD."
        );
        assert!(with(INV_COST, "-5").is_err());
        assert!(with(INV_MARKET, "x").is_err());
        assert!(with(INV_UPDATED, "yesterday").is_err());
        // A bare date is accepted and means local midnight.
        assert!(with(INV_UPDATED, "2026-07-01").is_ok());
    }

    #[test]
    fn local_datetime_roundtrips_through_display_format() {
        let iso = parse_local_datetime("2026-07-01 09:30").unwrap();
        assert_eq!(iso_to_local_input(&iso), "2026-07-01 09:30");
    }

    #[test]
    fn snapshot_payload_mirrors_web_validation() {
        let snap = |fields: [&str; 4]| build_snapshot_payload(&fields.map(str::to_string));
        let body = snap(["10,000", "10500", "1", "2026-03-01 16:00"]).unwrap();
        assert_eq!(body.total_cost, 10000.0);
        assert_eq!(body.exchange_rate, 1.0);
        assert!(DateTime::parse_from_rfc3339(&body.recorded_at).is_ok());
        assert_eq!(
            snap(["", "1", "1", "2026-03-01 16:00"]).unwrap_err(),
            "Total cost must be ≥ 0."
        );
        assert_eq!(
            snap(["1", "-2", "1", "2026-03-01 16:00"]).unwrap_err(),
            "Total market value must be ≥ 0."
        );
        assert_eq!(
            snap(["1", "1", "0", "2026-03-01 16:00"]).unwrap_err(),
            "Exchange rate must be > 0."
        );
        assert_eq!(
            snap(["1", "1", "1", "not a time"]).unwrap_err(),
            "Recorded at must be a valid YYYY-MM-DD HH:MM."
        );
    }

    #[test]
    fn insert_wrappers_flatten_owner_columns() {
        let payload = SnapshotPayload {
            total_cost: 1.0,
            total_market_value: 2.0,
            exchange_rate: 1.5,
            recorded_at: "2026-07-01T00:00:00+00:00".to_string(),
        };
        let body = to_json(NewSnapshot {
            user_id: "u1",
            investment_id: "i1",
            payload: &payload,
        });
        assert_eq!(
            body,
            json!({
                "user_id": "u1",
                "investment_id": "i1",
                "total_cost": 1.0,
                "total_market_value": 2.0,
                "exchange_rate": 1.5,
                "recorded_at": "2026-07-01T00:00:00+00:00",
            })
        );

        let spec = &SPECS[1];
        let income =
            build_entry_payload(spec, &values("Salary", "Work", "5000", "2026-07-01", "")).unwrap();
        let body = to_json(ForUser {
            user_id: "u1",
            payload: &income,
        });
        assert_eq!(
            body,
            json!({
                "user_id": "u1",
                "name": "Salary",
                "source": "Work",
                "amount": 5000.0,
                "date": "2026-07-01",
                "notes": null,
            })
        );
    }
}
