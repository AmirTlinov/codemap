use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RepoStatus {
    pub cwd: PathBuf,
    pub repo_root: Option<PathBuf>,
    pub vcs: Option<String>,
    pub cache_dir: PathBuf,
    pub zero_footprint_default: bool,
}

pub fn detect_status() -> Result<RepoStatus> {
    let cwd = env::current_dir().context("failed to read current directory")?;
    let repo_root = find_git_root(&cwd);
    let vcs = repo_root.as_ref().map(|_| "git".to_string());
    let cache_dir = cache_root();

    Ok(RepoStatus {
        cwd,
        repo_root,
        vcs,
        cache_dir,
        zero_footprint_default: true,
    })
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        if path.join(".git").exists() {
            return Some(path.to_path_buf());
        }
        current = path.parent();
    }
    None
}

fn cache_root() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(env::temp_dir)
        .join("agent-context")
}
