use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const METADATA_FILE: &str = "metadata.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct Metadata {
    /// Unix timestamp of the last successful release resolution.
    pub checked_at: u64,
    pub tag: String,
    pub asset_id: u64,
    pub asset_updated_at: String,
    /// Filename of the installed binary within the cache entry directory.
    pub bin: String,
}

pub fn root() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "sam-run")
        .context("cannot determine the cache directory for this user")?;
    Ok(dirs.cache_dir().to_path_buf())
}

pub fn entry_dir(repo: &str, tag: Option<&str>, platform: &str) -> Result<PathBuf> {
    Ok(root()?
        .join(repo)
        .join(tag.unwrap_or("latest"))
        .join(platform))
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

pub fn read_metadata(dir: &Path) -> Option<Metadata> {
    let text = fs::read_to_string(dir.join(METADATA_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_metadata(dir: &Path, metadata: &Metadata) -> Result<()> {
    fs::create_dir_all(dir)?;
    let json = serde_json::to_string_pretty(metadata)?;
    let temp = temp_path(dir);
    fs::write(&temp, json)?;
    fs::rename(&temp, dir.join(METADATA_FILE))?;
    Ok(())
}

/// A unique sibling path so writes can be atomically renamed into place; concurrent invocations
/// may duplicate a download but never observe partially written files.
pub fn temp_path(dir: &Path) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.subsec_nanos());
    dir.join(format!(".tmp-{}-{nanos}", std::process::id()))
}

/// Removes previously installed binaries other than `keep`. Best-effort: an old binary that is
/// still mid-exec elsewhere keeps its inode alive on unix, so deletion is always safe.
pub fn cleanup_old_binaries(dir: &Path, keep: &str) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if (name.starts_with("bin-") || name.starts_with(".tmp-")) && name != keep {
            let _ = fs::remove_file(entry.path());
        }
    }
}

pub fn invalidate(repo: Option<&str>) -> Result<()> {
    let root = root()?;
    let target = match repo {
        Some(repo) => root.join(repo),
        None => root,
    };
    match fs::remove_dir_all(&target) {
        Ok(()) => {
            println!("Invalidated cache at {}", target.display());
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            println!("Cache at {} is already empty", target.display());
            Ok(())
        }
        Err(err) => {
            Err(err).with_context(|| format!("failed to remove cache at {}", target.display()))
        }
    }
}
