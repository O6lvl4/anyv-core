//! Standard XDG-style paths for an anyv-family tool. The app name (`"gv"`,
//! `"rv"`, …) parameterizes the directory layout; an optional env-var override
//! (`<APP>_HOME`) lets users redirect everything for tests or sandboxing.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
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
    /// (with `data/`, `config/`, `cache/` subdirs), then falls back to the
    /// platform's XDG conventions.
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
        let pd = ProjectDirs::from("dev", "O6lvl4", app)
            .with_context(|| format!("could not resolve XDG directories for {app}"))?;
        Ok(Self {
            app,
            data: pd.data_dir().to_path_buf(),
            config: pd.config_dir().to_path_buf(),
            cache: pd.cache_dir().to_path_buf(),
        })
    }

    pub fn store(&self) -> PathBuf { self.data.join("store") }
    pub fn versions(&self) -> PathBuf { self.data.join("versions") }
    pub fn version_dir(&self, version: &str) -> PathBuf { self.versions().join(version) }
    pub fn tools(&self) -> PathBuf { self.data.join("tools") }
    pub fn global_version_file(&self) -> PathBuf { self.config.join("global") }

    pub fn ensure_dirs(&self) -> Result<()> {
        for d in [
            &self.data, &self.config, &self.cache,
            &self.store(), &self.versions(), &self.tools(),
        ] {
            ensure_dir(d)?;
        }
        Ok(())
    }
}

pub fn ensure_dir(p: &Path) -> Result<()> {
    if !p.exists() {
        std::fs::create_dir_all(p)
            .with_context(|| format!("create dir: {}", p.display()))?;
    }
    Ok(())
}
