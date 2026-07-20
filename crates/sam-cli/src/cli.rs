//! Command-line interface definition (clap).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Command-line companion for the in-canada and budget web apps.
///
/// Talks to the same Supabase backend the web apps use. On first run you'll be
/// prompted to sign in; the session is cached for subsequent invocations. Set
/// `SAM_CLI_EMAIL` / `SAM_CLI_PASSWORD` to sign in non-interactively.
#[derive(Debug, Parser)]
#[command(name = "sam", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Track days spent in Canada.
    InCanada(InCanadaArgs),
    /// Inspect expenses, income, and investments.
    Budget(BudgetArgs),
    /// Dashboard of git repos under ~/Desktop: unreleased work and language mix.
    Projects(ProjectsArgs),
    /// Interactive TUI for a local knowledge graph: undirected links between
    /// nodes, a markdown note per node, and a visual map of the whole graph.
    KnowledgeGraph,
}

#[derive(Debug, Args)]
pub struct InCanadaArgs {
    #[command(subcommand)]
    pub command: Option<InCanadaCommand>,
}

#[derive(Debug, Subcommand)]
pub enum InCanadaCommand {
    /// Show the day counter (default).
    Status,
    /// List the recorded days spent outside Canada.
    List,
    /// Record one or more days spent outside Canada (YYYY-MM-DD).
    Add {
        #[arg(required = true, value_name = "YYYY-MM-DD")]
        dates: Vec<String>,
    },
    /// Remove one or more recorded days outside Canada (YYYY-MM-DD).
    Remove {
        #[arg(required = true, value_name = "YYYY-MM-DD")]
        dates: Vec<String>,
    },
}

#[derive(Debug, Args)]
pub struct BudgetArgs {
    #[command(subcommand)]
    pub command: Option<BudgetCommand>,
}

#[derive(Debug, Subcommand)]
pub enum BudgetCommand {
    /// Dashboard with charts: totals, monthly flow, and category breakdowns (default).
    Status {
        /// Number of trailing months to chart.
        #[arg(long, default_value_t = 12)]
        months: u32,
    },
    /// Show a plain-text summary of income, expenses, and investments.
    Summary {
        /// Number of trailing months to include for income/expenses.
        #[arg(long, default_value_t = 12)]
        months: u32,
    },
    /// List recent expenses.
    Expenses {
        /// Maximum number of rows to show.
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List recent income.
    Income {
        /// Maximum number of rows to show.
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// List investments and their value in CAD.
    Investments,
    /// Interactive TUI to add, edit, and delete expenses, income, and investments.
    Tui,
}

#[derive(Debug, Args)]
pub struct TopArgs {
    /// Number of processes to show.
    #[arg(long, short = 'n', default_value_t = 5)]
    pub count: usize,
    /// Metric to rank processes by.
    #[arg(long, value_enum, default_value_t = TopSort::Cpu)]
    pub by: TopSort,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TopSort {
    /// Percentage of CPU time.
    Cpu,
    /// Percentage of physical memory.
    Mem,
}

#[derive(Debug, Args)]
pub struct TrafficArgs {
    /// Seconds between samples.
    #[arg(long, default_value_t = 1.0)]
    pub interval: f64,
    /// Number of samples to take (0 = run until interrupted).
    #[arg(long, short = 'n', default_value_t = 1)]
    pub count: u64,
}

#[derive(Debug, Args)]
pub struct ProjectsArgs {
    /// Directory to scan for git repositories (defaults to ~/Desktop).
    #[arg(value_name = "DIR")]
    pub path: Option<PathBuf>,
}
