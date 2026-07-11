mod cache;
mod cli;
mod exec;
mod extract;
mod github;
mod manifest;
mod platform;
mod runner;

use std::ffi::OsString;
use std::path::Path;

use clap::Parser;

fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    if let Err(err) = dispatch(args) {
        eprintln!("sam-run: {err:#}");
        std::process::exit(1);
    }
}

fn dispatch(args: Vec<OsString>) -> anyhow::Result<()> {
    // Shebang invocation arrives as `sam-run /path/to/script [args...]`: the kernel passes the
    // script path as the first argument. Anything else falls through to normal CLI parsing.
    if let Some(first) = args.get(1) {
        let path = Path::new(first);
        if path.is_file() {
            return runner::run(path, args[2..].to_vec());
        }
    }
    match cli::Cli::parse_from(args).command {
        cli::Command::Run { file, args } => runner::run(&file, args),
        cli::Command::Invalidate { repo } => cache::invalidate(repo.as_deref()),
    }
}
