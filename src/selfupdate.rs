//! Self-update: query GitHub for the latest release, sha256-verify the
//! archive for our target triple, atomic-replace the running binary.
//! Generic over the binary name so `gv` and `rv` (and any future tool) share
//! the same code path.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::extract::extract_archive;
use crate::target::target_triple;

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Outcome {
    AlreadyUpToDate,
    NewerAvailable, // when --check
    Updated,
}

pub struct SelfUpdate {
    pub repo: &'static str,    // "O6lvl4/gv"
    pub bin_name: &'static str, // "gv"
    pub current_version: &'static str, // env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub outcome: Outcome,
    pub binary_path: Option<PathBuf>,
}

impl SelfUpdate {
    /// Resolve the latest tag without doing anything else.
    pub async fn latest_tag(&self, client: &reqwest::Client) -> Result<String> {
        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        let release: GhRelease = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", format!("{}/{}", self.bin_name, self.current_version))
            .send().await
            .with_context(|| format!("GET {url}"))?
            .error_for_status()?
            .json().await
            .context("parse GitHub release JSON")?;
        Ok(release.tag_name)
    }

    /// Check vs install. When `check_only`, returns AlreadyUpToDate or
    /// NewerAvailable without touching disk. Otherwise downloads + replaces.
    pub async fn run(&self, client: &reqwest::Client, check_only: bool) -> Result<UpdateInfo> {
        let latest_tag = self.latest_tag(client).await?;
        let latest = latest_tag.strip_prefix('v').unwrap_or(&latest_tag).to_string();
        let current = self.current_version.to_string();

        if !is_semver_newer(&latest, &current) {
            return Ok(UpdateInfo {
                current, latest,
                outcome: Outcome::AlreadyUpToDate,
                binary_path: None,
            });
        }
        if check_only {
            return Ok(UpdateInfo {
                current, latest,
                outcome: Outcome::NewerAvailable,
                binary_path: None,
            });
        }

        let triple = target_triple()
            .ok_or_else(|| anyhow!("self-update is not supported on this platform"))?;
        let archive_name = if cfg!(target_os = "windows") {
            format!("{}-{latest_tag}-{triple}.zip", self.bin_name)
        } else {
            format!("{}-{latest_tag}-{triple}.tar.gz", self.bin_name)
        };
        let url = format!(
            "https://github.com/{}/releases/download/{}/{}",
            self.repo, latest_tag, archive_name
        );

        let bytes = client.get(&url).send().await?.error_for_status()?.bytes().await?;
        let sha_text = client
            .get(format!("{url}.sha256"))
            .send().await?
            .error_for_status()?
            .text().await?;
        let expected: String = sha_text.split_whitespace().next().unwrap_or("").to_string();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode(hasher.finalize());
        if !expected.is_empty() && expected != actual {
            bail!("sha256 mismatch for {archive_name}: expected {expected}, got {actual}");
        }

        let tmp = tempdir(&format!("{}-self-update-", self.bin_name))?;
        let archive_path = tmp.join(&archive_name);
        std::fs::write(&archive_path, &bytes)?;
        extract_archive(&archive_path, &tmp)?;

        let stage = tmp.join(format!("{}-{latest_tag}-{triple}", self.bin_name));
        let exe_name = if cfg!(windows) {
            format!("{}.exe", self.bin_name)
        } else {
            self.bin_name.to_string()
        };
        let new_exe = stage.join(&exe_name);
        if !new_exe.exists() {
            bail!("extracted archive missing expected binary at {}", new_exe.display());
        }

        let current_exe = std::env::current_exe()?;
        replace_binary(&new_exe, &current_exe)?;

        Ok(UpdateInfo {
            current, latest,
            outcome: Outcome::Updated,
            binary_path: Some(current_exe),
        })
    }
}

fn is_semver_newer(latest: &str, current: &str) -> bool {
    fn parse(s: &str) -> (u64, u64, u64) {
        let mut p = s.split('.').map(|x| x.split('-').next().unwrap_or(""));
        (
            p.next().and_then(|x| x.parse().ok()).unwrap_or(0),
            p.next().and_then(|x| x.parse().ok()).unwrap_or(0),
            p.next().and_then(|x| x.parse().ok()).unwrap_or(0),
        )
    }
    parse(latest) > parse(current)
}

fn tempdir(prefix: &str) -> Result<PathBuf> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let p = std::env::temp_dir().join(format!("{prefix}{nonce}"));
    std::fs::create_dir_all(&p)?;
    Ok(p)
}

fn replace_binary(src: &Path, dest: &Path) -> Result<()> {
    if cfg!(windows) && dest.exists() {
        let backup = dest.with_extension("old");
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(dest, &backup)
            .with_context(|| format!("rename {} → {}", dest.display(), backup.display()))?;
    }
    std::fs::rename(src, dest)
        .or_else(|_| {
            std::fs::copy(src, dest)
                .map(|_| ())
                .and_then(|_| std::fs::remove_file(src))
        })
        .with_context(|| format!("install binary at {}", dest.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(dest, perms)?;
    }
    Ok(())
}
