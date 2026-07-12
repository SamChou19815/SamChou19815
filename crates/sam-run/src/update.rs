use std::fs;
use std::io;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::{cache, extract, github, platform};

const REPO: &str = "SamChou19815/SamChou19815";
const TAG: &str = "sam-run-latest";
/// Commit SHA embedded at build time by GitHub Actions; None for local dev builds.
pub const BUILD_SHA: Option<&str> = option_env!("GITHUB_SHA");

/// Version string for --version: "0.1.0 (<commit sha>)" or "0.1.0 (dev)".
pub fn version_string() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            BUILD_SHA.unwrap_or("dev")
        )
    })
}

fn asset_name(platform: &str) -> Result<String> {
    if !platform::SUPPORTED.contains(&platform) {
        bail!(
            "sam-run has no released binary for platform {platform} (supported: {})",
            platform::SUPPORTED.join(", ")
        );
    }
    Ok(format!("sam-run-{platform}"))
}

fn is_up_to_date(local: Option<&str>, remote: &str) -> bool {
    !remote.is_empty() && local == Some(remote)
}

pub fn self_update(check: bool) -> Result<()> {
    let exe = std::env::current_exe().context("cannot determine the running executable")?;
    // Canonicalize so a symlinked install replaces the real file and keeps the link intact.
    let exe = fs::canonicalize(&exe)
        .with_context(|| format!("cannot resolve executable path {}", exe.display()))?;
    let dir = exe
        .parent()
        .context("executable path has no parent directory")?
        .to_path_buf();

    let client = github::Client::new()?;
    let release = client.resolve_release(REPO, Some(TAG))?;
    let remote = release.target_commitish.clone();
    if is_up_to_date(BUILD_SHA, &remote) {
        println!("sam-run is already up to date (commit {remote})");
        return Ok(());
    }
    if BUILD_SHA.is_none() {
        println!("this build has no embedded commit (dev build); the release is commit {remote}");
    }
    if check {
        println!(
            "update available: {} -> {remote}",
            BUILD_SHA.unwrap_or("dev")
        );
        return Ok(());
    }

    let asset_name = asset_name(&platform::current())?;
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .with_context(|| {
            let available: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
            format!(
                "release {} has no asset {asset_name:?} (available: {available:?})",
                release.tag_name
            )
        })?;

    // Download to a sibling of the executable so the final rename stays on one filesystem and
    // atomically swaps the binary; the running process keeps its old inode.
    let temp = cache::temp_path(&dir);
    let result = fs::File::create(&temp)
        .map_err(|err| permission_hint(err, &dir))
        .and_then(|mut file| client.download(asset, &mut file))
        .and_then(|()| extract::make_executable(&temp))
        .and_then(|()| fs::rename(&temp, &exe).map_err(|err| permission_hint(err, &dir)));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result?;
    println!("updated sam-run to commit {remote} at {}", exe.display());
    Ok(())
}

fn permission_hint(err: io::Error, dir: &Path) -> anyhow::Error {
    if err.kind() == io::ErrorKind::PermissionDenied {
        anyhow::anyhow!(
            "cannot write to {}: permission denied; re-run with `sudo -E sam-run self-update` \
             or reinstall sam-run to a directory you own",
            dir.display()
        )
    } else {
        anyhow::Error::new(err).context(format!("failed to write update into {}", dir.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_names_cover_supported_platforms() {
        for platform in platform::SUPPORTED {
            assert_eq!(asset_name(platform).unwrap(), format!("sam-run-{platform}"));
        }
    }

    #[test]
    fn asset_name_rejects_unsupported_platform() {
        assert!(asset_name("windows-x86_64").is_err());
    }

    #[test]
    fn up_to_date_only_on_exact_match() {
        assert!(is_up_to_date(Some("abc123"), "abc123"));
        assert!(!is_up_to_date(Some("abc123"), "def456"));
        assert!(!is_up_to_date(None, "abc123"));
        assert!(!is_up_to_date(Some(""), ""));
        assert!(!is_up_to_date(None, ""));
    }
}
