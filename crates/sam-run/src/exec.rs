use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

#[cfg(unix)]
pub fn exec(binary: &Path, args: &[OsString]) -> Result<()> {
    use std::os::unix::process::CommandExt;
    // exec only returns on failure; on success the binary replaces this process and owns the
    // tty, signals, and exit code.
    let err = Command::new(binary).args(args).exec();
    Err(err).with_context(|| format!("failed to execute {}", binary.display()))
}

#[cfg(not(unix))]
pub fn exec(binary: &Path, args: &[OsString]) -> Result<()> {
    let status = Command::new(binary)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute {}", binary.display()))?;
    std::process::exit(status.code().unwrap_or(1));
}
