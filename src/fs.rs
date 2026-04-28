//! Light filesystem helpers used by `cache info` / `cache prune` style
//! commands. Symlinks are not followed (we account for the link itself, not
//! the dereferenced target — otherwise prune would double-count).

use std::path::{Path, PathBuf};

use anyhow::Result;

/// Total size in bytes plus shallow entry count (top-level only) of a tree.
pub fn dir_size(path: &Path) -> Result<(u64, usize)> {
    if path.is_file() {
        return Ok((std::fs::metadata(path)?.len(), 1));
    }
    if !path.is_dir() {
        return Ok((0, 0));
    }
    let mut total: u64 = 0;
    let mut count: usize = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)? {
            let entry = entry?;
            let p = entry.path();
            let meta = entry.metadata()?;
            if meta.is_symlink() {
                continue;
            }
            if meta.is_dir() {
                if d == path { count += 1; }
                stack.push(p);
            } else {
                if d == path { count += 1; }
                total += meta.len();
            }
        }
    }
    Ok((total, count))
}

/// Walk files under `root` up to `max_depth`, ignoring common rubbish
/// directories (`.git`, `node_modules`, `vendor`, dotfiles).
pub fn walk_files(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue; };
        for entry in rd.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "vendor" || name == "node_modules" {
                continue;
            }
            if p.is_dir() {
                if depth < max_depth {
                    stack.push((p, depth + 1));
                }
            } else {
                out.push(p);
            }
        }
    }
    out
}
