use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sam-run",
    version = crate::update::version_string(),
    about = "Run shebang scripts that download and execute binaries from GitHub releases"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a sam-run script, forwarding remaining arguments to the downloaded binary
    Run {
        /// Path to the script file
        file: PathBuf,
        /// Arguments forwarded to the downloaded binary
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    /// Invalidate the download cache so the next run re-resolves the release
    Invalidate {
        /// Limit invalidation to one repository (owner/name); omit to clear the entire cache
        repo: Option<String>,
    },
    /// Download the latest released sam-run binary and replace this executable
    SelfUpdate {
        /// Only report whether an update is available; do not install it
        #[arg(long)]
        check: bool,
    },
}
