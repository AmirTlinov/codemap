use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::model::Project;

pub fn cache_base_dir() -> PathBuf {
    if let Ok(dir) = env::var("CTX_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .unwrap_or_else(env::temp_dir)
        .join("agent-context")
}

pub fn project_cache_dir(root: &Path, remote: Option<&str>, version: &str) -> PathBuf {
    cache_base_dir().join(repo_key(root, remote, version))
}

pub fn repo_key(root: &Path, remote: Option<&str>, version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root.to_string_lossy().as_bytes());
    hasher.update([0]);
    if let Some(remote) = remote {
        hasher.update(remote.as_bytes());
    }
    hasher.update([0]);
    hasher.update(version.as_bytes());
    let hash = hasher.finalize();
    hex_prefix(&hash, 20)
}

pub fn fingerprint(project: &Project, domain_path: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    let domain_prefix = domain_path
        .filter(|p| *p != ".")
        .map(|p| format!("{}/", p.trim_end_matches('/')));
    for file in project.files.values() {
        if let Some(prefix) = &domain_prefix
            && !file.rel.starts_with(prefix)
        {
            continue;
        }
        hasher.update(file.rel.as_bytes());
        hasher.update([0]);
        hasher.update(file.size.to_string().as_bytes());
        if let Ok(meta) = std::fs::metadata(file.rel_path(project))
            && let Ok(modified) = meta.modified()
            && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            hasher.update(duration.as_secs().to_string().as_bytes());
        }
    }
    if let Some(path) = &project.config_path {
        hasher.update(path.as_bytes());
    }
    let hash = hasher.finalize();
    hex_prefix(&hash, 16)
}

pub fn write_status(project: &Project, version: &str) -> Result<()> {
    if env::var_os("CTX_NO_CACHE").is_some() {
        return Ok(());
    }
    fs::create_dir_all(&project.cache_dir)?;
    let status = CacheStatus {
        version,
        root: project.root.to_string_lossy().to_string(),
        fingerprint: fingerprint(project, None),
        files: project.files.len(),
        domains: project
            .domains
            .iter()
            .map(|d| CachedDomain {
                id: d.id.clone(),
                path: d.path.clone(),
                config: d.config_path.clone(),
            })
            .collect(),
    };
    let body = serde_json::to_string_pretty(&status)?;
    fs::write(project.cache_dir.join("status.json"), format!("{body}\n"))?;
    Ok(())
}

#[derive(Serialize)]
struct CacheStatus<'a> {
    version: &'a str,
    root: String,
    fingerprint: String,
    files: usize,
    domains: Vec<CachedDomain>,
}

#[derive(Serialize)]
struct CachedDomain {
    id: String,
    path: String,
    config: Option<String>,
}

fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    bytes
        .iter()
        .flat_map(|b| [b >> 4, b & 0x0f])
        .take(chars)
        .map(|n| char::from_digit(n as u32, 16).expect("hex digit"))
        .collect()
}

trait RelPath {
    fn rel_path(&self, project: &Project) -> PathBuf;
}

impl RelPath for crate::model::FileInfo {
    fn rel_path(&self, project: &Project) -> PathBuf {
        project.root.join(&self.rel)
    }
}
