//! Configuration and on-disk session cache.
//!
//! The Supabase project URL and anon (publishable) key are not secrets — they
//! ship in the web app's client bundle — so we bake them in as defaults and let
//! the environment override them. The user's *session* (access / refresh
//! tokens), on the other hand, is sensitive and is cached in the user's config
//! directory with owner-only permissions.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Public Supabase values, mirrored from `packages/www/.env.local`. Overridable
/// with `SUPABASE_URL` / `SUPABASE_ANON_KEY` (the `NEXT_PUBLIC_`-prefixed names
/// are accepted too, so the same `.env.local` can be sourced).
const DEFAULT_SUPABASE_URL: &str = "https://yrklwvnpkqhmhsmmaele.supabase.co";
const DEFAULT_SUPABASE_ANON_KEY: &str = "sb_publishable_cWyX02iHRP9tHRaFwyf8Wg_EXlZKv3I";

/// Resolved Supabase connection settings.
pub struct Config {
    pub url: String,
    pub anon_key: String,
}

impl Config {
    /// Resolve settings from the environment, falling back to baked-in defaults.
    pub fn load() -> Self {
        Self {
            url: env_any(&["SUPABASE_URL", "NEXT_PUBLIC_SUPABASE_URL"])
                .unwrap_or_else(|| DEFAULT_SUPABASE_URL.to_string()),
            anon_key: env_any(&["SUPABASE_ANON_KEY", "NEXT_PUBLIC_SUPABASE_ANON_KEY"])
                .unwrap_or_else(|| DEFAULT_SUPABASE_ANON_KEY.to_string()),
        }
    }
}

fn env_any(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .filter(|v| !v.trim().is_empty())
}

/// A cached Supabase auth session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix timestamp (seconds) at which `access_token` expires.
    pub expires_at: i64,
    pub user_id: String,
    pub email: Option<String>,
}

impl Session {
    /// Whether the access token is expired (or within 60s of expiring).
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        self.expires_at - now <= 60
    }
}

fn session_path() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "developersam", "sam-cli")
        .context("could not determine a config directory for this platform")?;
    Ok(dirs.config_dir().join("session.json"))
}

/// Load the cached session, if any.
pub fn load_session() -> Result<Option<Session>> {
    let path = session_path()?;
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };
    let session = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing cached session at {}", path.display()))?;
    Ok(Some(session))
}

/// Persist the session to disk with owner-only permissions.
pub fn save_session(session: &Session) -> Result<()> {
    let path = session_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating config directory {}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(session)?;
    fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    restrict_permissions(&path)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &std::path::Path) -> Result<()> {
    Ok(())
}
