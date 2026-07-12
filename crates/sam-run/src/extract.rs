use std::fs::{self, File};
use std::io;
use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::cache;
use crate::manifest::Format;

pub fn infer_format(asset_name: &str) -> Format {
    if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        Format::TarGz
    } else if asset_name.ends_with(".zip") {
        Format::Zip
    } else if asset_name.ends_with(".gz") {
        Format::Gz
    } else {
        Format::Binary
    }
}

/// Turns the downloaded asset at `download` into the executable at `dest` (a sibling path),
/// extracting archives as needed. The download file is consumed or left for caller cleanup.
pub fn extract(download: &Path, format: Format, inner: Option<&str>, dest: &Path) -> Result<()> {
    match format {
        Format::Binary => {
            make_executable(download)?;
            fs::rename(download, dest)?;
            Ok(())
        }
        Format::Gz => {
            let temp = cache::temp_path(dest.parent().context("destination has no parent")?);
            let mut decoder = flate2::read::GzDecoder::new(File::open(download)?);
            let mut out = File::create(&temp)?;
            io::copy(&mut decoder, &mut out).context("failed to decompress gzip asset")?;
            drop(out);
            make_executable(&temp)?;
            fs::rename(&temp, dest)?;
            Ok(())
        }
        Format::TarGz => extract_tar_gz(download, inner, dest),
        Format::Zip => extract_zip(download, inner, dest),
    }
}

fn extract_tar_gz(download: &Path, inner: Option<&str>, dest: &Path) -> Result<()> {
    let open = || -> Result<tar::Archive<flate2::read::GzDecoder<File>>> {
        Ok(tar::Archive::new(flate2::read::GzDecoder::new(File::open(
            download,
        )?)))
    };
    let inner = match inner {
        Some(inner) => inner.to_string(),
        // Without an explicit path, the archive must contain exactly one regular file.
        None => {
            let mut files = Vec::new();
            for entry in open()?.entries()? {
                let entry = entry?;
                if entry.header().entry_type().is_file() {
                    files.push(entry.path()?.to_string_lossy().into_owned());
                }
            }
            sole_file(files)?
        }
    };
    let mut archive = open()?;
    for entry in archive.entries()? {
        let mut entry = entry?;
        if paths_match(&entry.path()?.to_string_lossy(), &inner) {
            let temp = cache::temp_path(dest.parent().context("destination has no parent")?);
            let mut out = File::create(&temp)?;
            io::copy(&mut entry, &mut out).context("failed to extract archive entry")?;
            drop(out);
            make_executable(&temp)?;
            fs::rename(&temp, dest)?;
            return Ok(());
        }
    }
    bail!("archive has no entry named {inner:?}");
}

fn extract_zip(download: &Path, inner: Option<&str>, dest: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(download)?)?;
    let inner = match inner {
        Some(inner) => inner.to_string(),
        None => {
            let files: Vec<String> = (0..archive.len())
                .filter_map(|index| {
                    let entry = archive.by_index(index).ok()?;
                    entry.is_file().then(|| entry.name().to_string())
                })
                .collect();
            sole_file(files)?
        }
    };
    let index = (0..archive.len())
        .find(|&index| {
            archive
                .by_index(index)
                .is_ok_and(|entry| paths_match(entry.name(), &inner))
        })
        .with_context(|| format!("archive has no entry named {inner:?}"))?;
    let mut entry = archive.by_index(index)?;
    let temp = cache::temp_path(dest.parent().context("destination has no parent")?);
    let mut out = File::create(&temp)?;
    io::copy(&mut entry, &mut out).context("failed to extract archive entry")?;
    drop(out);
    make_executable(&temp)?;
    fs::rename(&temp, dest)?;
    Ok(())
}

fn sole_file(files: Vec<String>) -> Result<String> {
    match files.as_slice() {
        [only] => Ok(only.clone()),
        [] => bail!("archive contains no regular files"),
        _ => bail!("archive contains multiple files; specify which one with \"path\": {files:?}"),
    }
}

fn paths_match(entry: &str, wanted: &str) -> bool {
    entry == wanted || entry.strip_prefix("./") == Some(wanted)
}

#[cfg(unix)]
pub fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("failed to mark {} executable", path.display()))
}

#[cfg(not(unix))]
pub fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}
