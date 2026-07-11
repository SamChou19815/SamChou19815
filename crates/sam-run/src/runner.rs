use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::manifest::{expand_placeholders, Manifest, PlatformEntry, DEFAULT_TTL_HOURS};
use crate::{cache, exec, extract, github, platform};

pub fn run(script: &Path, args: Vec<OsString>) -> Result<()> {
    let manifest = Manifest::load(script)?;
    let platform_key = platform::current();
    let entry = manifest.platforms.get(&platform_key).with_context(|| {
        format!(
            "{} does not support platform {platform_key} (supported keys: {})",
            script.display(),
            platform::SUPPORTED.join(", ")
        )
    })?;
    let dir = cache::entry_dir(&manifest.repo, manifest.tag.as_deref(), &platform_key)?;
    let ttl_secs = manifest.ttl_hours.unwrap_or(DEFAULT_TTL_HOURS) * 3600;

    let cached = cache::read_metadata(&dir);
    if let Some(metadata) = &cached {
        let bin = dir.join(&metadata.bin);
        if cache::now_unix().saturating_sub(metadata.checked_at) < ttl_secs && bin.is_file() {
            return exec::exec(&bin, &args);
        }
    }

    match refresh(&manifest, entry, &dir, cached.as_ref()) {
        Ok(bin) => exec::exec(&bin, &args),
        Err(err) => {
            // Never let an expired TTL break a working machine: on any resolution or download
            // failure (offline, rate limit, or the rolling release's delete/recreate gap),
            // fall back to the previously installed binary.
            if let Some(metadata) = &cached {
                let bin = dir.join(&metadata.bin);
                if bin.is_file() {
                    eprintln!(
                        "sam-run: warning: using stale cache for {}: {err:#}",
                        manifest.repo
                    );
                    return exec::exec(&bin, &args);
                }
            }
            Err(err)
        }
    }
}

fn refresh(
    manifest: &Manifest,
    entry: &PlatformEntry,
    dir: &Path,
    cached: Option<&cache::Metadata>,
) -> Result<PathBuf> {
    let client = github::Client::new()?;
    let release = client.resolve_release(&manifest.repo, manifest.tag.as_deref())?;
    let asset_name = expand_placeholders(&entry.asset, &release.tag_name);
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .with_context(|| {
            let available: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
            format!(
                "release {} of {} has no asset {asset_name:?} (available: {available:?})",
                release.tag_name, manifest.repo
            )
        })?;

    // The release still points at the asset we already installed: only refresh the timestamp.
    if let Some(metadata) = cached {
        if metadata.asset_id == asset.id && metadata.asset_updated_at == asset.updated_at {
            let bin = dir.join(&metadata.bin);
            if bin.is_file() {
                cache::write_metadata(
                    dir,
                    &cache::Metadata {
                        checked_at: cache::now_unix(),
                        ..metadata.clone()
                    },
                )?;
                return Ok(bin);
            }
        }
    }

    fs::create_dir_all(dir)?;
    let download = cache::temp_path(dir);
    let result = client.download(asset, &mut fs::File::create(&download)?);
    if result.is_err() {
        let _ = fs::remove_file(&download);
    }
    result?;

    let format = entry
        .format
        .unwrap_or_else(|| extract::infer_format(&asset.name));
    let inner = entry
        .path
        .as_deref()
        .map(|path| expand_placeholders(path, &release.tag_name));
    let bin_name = format!("bin-{}", asset.id);
    let bin = dir.join(&bin_name);
    let extracted = extract::extract(&download, format, inner.as_deref(), &bin);
    let _ = fs::remove_file(&download);
    extracted?;

    cache::write_metadata(
        dir,
        &cache::Metadata {
            checked_at: cache::now_unix(),
            tag: release.tag_name.clone(),
            asset_id: asset.id,
            asset_updated_at: asset.updated_at.clone(),
            bin: bin_name.clone(),
        },
    )?;
    cache::cleanup_old_binaries(dir, &bin_name);
    Ok(bin)
}
