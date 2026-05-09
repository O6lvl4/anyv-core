//! Standard XDG-style paths for an anyv-family tool. The app name (`"gv"`,
//! `"rv"`, …) parameterizes the directory layout; an optional env-var override
//! (`<APP>_HOME`) lets users redirect everything for tests or sandboxing.
//!
//! On macOS, we intentionally use XDG-style paths (`~/.local/share/`,
//! `~/.config/`, `~/.cache/`) instead of `~/Library/Application Support/`.
//! The literal space in `Application Support` breaks third-party build
//! tooling (ruby-build, groovy, clojure, lua, haskell, …) that expands
//! paths unquoted. Existing data under the legacy location is
//! auto-migrated on first access.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
#[cfg(not(target_os = "macos"))]
use directories::ProjectDirs;

#[derive(Debug, Clone)]
pub struct Paths {
    pub app: &'static str,
    pub data: PathBuf,
    pub config: PathBuf,
    pub cache: PathBuf,
}

impl Paths {
    /// Resolve paths for the given app. Honors `<APP_UPPERCASE>_HOME` first
    /// (with `data/`, `config/`, `cache/` subdirs), then falls back to
    /// XDG-style directories.
    ///
    /// On macOS, `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME` are
    /// respected; the defaults are `~/.local/share/<app>`,
    /// `~/.config/<app>`, `~/.cache/<app>`.
    pub fn discover(app: &'static str) -> Result<Self> {
        let env_var = format!("{}_HOME", app.to_ascii_uppercase());
        if let Ok(home) = std::env::var(&env_var) {
            let root = PathBuf::from(home);
            return Ok(Self {
                app,
                data: root.join("data"),
                config: root.join("config"),
                cache: root.join("cache"),
            });
        }

        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME").with_context(|| {
                format!("HOME not set; needed to resolve directories for {app}")
            })?;
            let home = PathBuf::from(home);

            let data = xdg_dir("XDG_DATA_HOME", &home, ".local/share", app);
            let config = xdg_dir("XDG_CONFIG_HOME", &home, ".config", app);
            let cache = xdg_dir("XDG_CACHE_HOME", &home, ".cache", app);

            migrate_from_application_support(&home, app, &data, &config);

            return Ok(Self {
                app,
                data,
                config,
                cache,
            });
        }

        #[cfg(not(target_os = "macos"))]
        {
            let pd = ProjectDirs::from("dev", "O6lvl4", app)
                .with_context(|| format!("could not resolve XDG directories for {app}"))?;
            Ok(Self {
                app,
                data: pd.data_dir().to_path_buf(),
                config: pd.config_dir().to_path_buf(),
                cache: pd.cache_dir().to_path_buf(),
            })
        }
    }

    pub fn store(&self) -> PathBuf {
        self.data.join("store")
    }
    pub fn versions(&self) -> PathBuf {
        self.data.join("versions")
    }
    pub fn version_dir(&self, version: &str) -> PathBuf {
        self.versions().join(version)
    }
    pub fn tools(&self) -> PathBuf {
        self.data.join("tools")
    }
    pub fn global_version_file(&self) -> PathBuf {
        self.config.join("global")
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for d in [
            &self.data,
            &self.config,
            &self.cache,
            &self.store(),
            &self.versions(),
            &self.tools(),
        ] {
            ensure_dir(d)?;
        }
        Ok(())
    }
}

pub fn ensure_dir(p: &Path) -> Result<()> {
    if !p.exists() {
        std::fs::create_dir_all(p).with_context(|| format!("create dir: {}", p.display()))?;
    }
    Ok(())
}

// ─── macOS: XDG helpers ─────────────────────────────────────────────

#[cfg(target_os = "macos")]
fn xdg_dir(env_var: &str, home: &Path, default_segment: &str, app: &str) -> PathBuf {
    if let Ok(v) = std::env::var(env_var) {
        PathBuf::from(v).join(app)
    } else {
        home.join(default_segment).join(app)
    }
}

/// One-shot migration from the legacy `~/Library/Application Support/`
/// layout to XDG-style `~/.local/share/`. On the old macOS layout, the
/// `directories` crate mapped both `data_dir` and `config_dir` to
/// `~/Library/Application Support/dev.O6lvl4.<app>/`, so versions,
/// store, tools, *and* the global pin file all lived in the same
/// directory. We move everything into the new data dir, then relocate
/// the global pin file (if any) into the proper config dir.
#[cfg(target_os = "macos")]
fn migrate_from_application_support(home: &Path, app: &str, new_data: &Path, new_config: &Path) {
    let old_data = home
        .join("Library/Application Support")
        .join(format!("dev.O6lvl4.{app}"));
    if !old_data.exists() || new_data.exists() {
        return;
    }
    if let Some(parent) = new_data.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::rename(&old_data, new_data).is_ok() {
        eprintln!(
            "{app}: migrated data from {} → {}",
            old_data.display(),
            new_data.display(),
        );
        // The global pin file was in data/ (since data == config on the
        // old macOS layout). Move it to the proper config location.
        let global_in_data = new_data.join("global");
        if global_in_data.exists() {
            let _ = std::fs::create_dir_all(new_config);
            let global_in_config = new_config.join("global");
            if std::fs::rename(&global_in_data, &global_in_config).is_ok() {
                eprintln!(
                    "{app}: migrated global pin → {}",
                    global_in_config.display(),
                );
            }
        }
    }
}
