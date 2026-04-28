//! Archive extraction. Detects `.tar.gz` vs `.zip` from the filename and
//! dispatches to the right backend so callers never need to care.

use std::path::Path;

use anyhow::{Context, Result};

/// Extract `archive` into `dest`. The format is inferred from the filename's
/// extension (`.zip` → zip; anything else → tar+gzip, the unix default).
pub fn extract_archive(archive: &Path, dest: &Path) -> Result<()> {
    let name = archive
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.ends_with(".zip") {
        extract_zip(archive, dest)
    } else {
        extract_tar_gz(archive, dest)
    }
}

pub fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    let f = std::fs::File::open(archive)
        .with_context(|| format!("open {}", archive.display()))?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut tar = tar::Archive::new(gz);
    tar.set_preserve_permissions(true);
    tar.set_overwrite(true);
    tar.unpack(dest)
        .with_context(|| format!("unpack tar.gz to {}", dest.display()))?;
    Ok(())
}

pub fn extract_zip(archive: &Path, dest: &Path) -> Result<()> {
    let f = std::fs::File::open(archive)
        .with_context(|| format!("open {}", archive.display()))?;
    let mut zip = zip::ZipArchive::new(f).context("open zip")?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let outpath = match entry.enclosed_name() {
            Some(p) => dest.join(p),
            None => continue,
        };
        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)?;
            continue;
        }
        if let Some(parent) = outpath.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&outpath)
            .with_context(|| format!("create {}", outpath.display()))?;
        std::io::copy(&mut entry, &mut out)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}
