use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub const DEFAULT_TTL_HOURS: u64 = 24;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Manifest {
    /// GitHub repository in `owner/name` form.
    pub repo: String,
    /// Release tag to use; defaults to the repository's latest release.
    #[serde(default)]
    pub tag: Option<String>,
    /// How long a resolved release stays fresh before re-checking; 0 checks every run.
    #[serde(default)]
    pub ttl_hours: Option<u64>,
    /// Platform key (e.g. `macos-aarch64`) to release asset.
    pub platforms: BTreeMap<String, PlatformEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PlatformEntry {
    /// Asset name within the release; `{tag}` and `{version}` placeholders are expanded.
    pub asset: String,
    /// Path of the executable inside an archive asset; placeholders are expanded.
    #[serde(default)]
    pub path: Option<String>,
    /// Asset format; inferred from the asset extension when omitted.
    #[serde(default)]
    pub format: Option<Format>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    #[serde(rename = "binary")]
    Binary,
    #[serde(rename = "tar.gz")]
    TarGz,
    #[serde(rename = "zip")]
    Zip,
    #[serde(rename = "gz")]
    Gz,
}

impl Manifest {
    pub fn load(script: &Path) -> Result<Manifest> {
        let text = std::fs::read_to_string(script)
            .with_context(|| format!("failed to read script {}", script.display()))?;
        Manifest::parse(&text)
            .with_context(|| format!("failed to parse manifest in {}", script.display()))
    }

    fn parse(text: &str) -> Result<Manifest> {
        let body = match text.strip_prefix("#!") {
            Some(rest) => rest.split_once('\n').map_or("", |(_, body)| body),
            None => text,
        };
        let manifest: Manifest = serde_json::from_str(body)?;
        let (owner, name) = manifest
            .repo
            .split_once('/')
            .with_context(|| format!("repo {:?} is not in owner/name form", manifest.repo))?;
        for part in [owner, name] {
            if part.is_empty() || part == "." || part == ".." || part.contains(['/', '\\']) {
                bail!("repo {:?} is not a valid owner/name", manifest.repo);
            }
        }
        Ok(manifest)
    }
}

/// Expands `{tag}` and `{version}` (the tag with one leading `v` stripped) in asset names and
/// archive paths, since most projects embed the release version in their asset names.
pub fn expand_placeholders(template: &str, tag: &str) -> String {
    let version = tag.strip_prefix('v').unwrap_or(tag);
    template.replace("{tag}", tag).replace("{version}", version)
}
