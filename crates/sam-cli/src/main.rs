mod charts;
mod cli;
mod commands;
mod config;
mod supabase;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};
use supabase::Supabase;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // Commands backed by Supabase sign in (prompting on first run) lazily,
        // so the purely local commands below never trigger an auth prompt.
        Command::InCanada(args) => commands::in_canada::run(&Supabase::connect()?, args.command),
        Command::Budget(args) => commands::budget::run(&Supabase::connect()?, args.command),
        Command::Projects(args) => commands::projects::run(args),
        Command::KnowledgeGraph => commands::knowledge_graph::run(),
    }
}
