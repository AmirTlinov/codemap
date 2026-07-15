// Responsibility: cache-identity
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::model::Project;

const CACHE_ARTIFACTS: &[&str] = &[
    "status.json",
    "inventory.json",
    "graph.json",
    "fingerprints.json",
    "runtime-root.json",
];

pub fn cache_base_dir() -> PathBuf {
    if let Ok(dir) = env::var("CODEMAP_CACHE_DIR") {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .unwrap_or_else(env::temp_dir)
        .join("codemap")
}

pub fn project_cache_dir(root: &Path, remote: Option<&str>, version: &str) -> PathBuf {
    cache_base_dir().join(repo_key(root, remote, version))
}

pub fn cache_enabled() -> bool {
    env::var_os("CODEMAP_NO_CACHE").is_none()
}

pub fn expected_artifacts() -> &'static [&'static str] {
    CACHE_ARTIFACTS
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
        if let Some(content_hash) = &file.content_hash {
            hasher.update(b"content");
            hasher.update(content_hash.as_bytes());
        } else if let Ok(meta) = std::fs::symlink_metadata(file.rel_path(project)) {
            if meta.file_type().is_symlink() {
                hasher.update(b"symlink");
                if let Ok(target) = std::fs::read_link(file.rel_path(project)) {
                    hasher.update(target.to_string_lossy().as_bytes());
                }
            } else {
                // Placeholders are deliberately not parsed, so timestamp-only
                // changes cannot alter any structural fact. Path and size are
                // already committed above; keep their snapshot independent of
                // incidental mtime churn.
                hasher.update(b"placeholder");
            }
        }
    }
    if let Some(path) = &project.config_path {
        hasher.update(path.as_bytes());
    }
    let hash = hasher.finalize();
    hex_prefix(&hash, 16)
}

pub fn inventory_fingerprint(root: &Path, files: &[String]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root.to_string_lossy().as_bytes());
    hasher.update([0]);
    let mut files = files.to_vec();
    files.sort();
    for rel in files {
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        if let Ok(meta) = fs::metadata(root.join(&rel)) {
            hasher.update(meta.len().to_string().as_bytes());
            hasher.update([0]);
            if let Ok(modified) = meta.modified()
                && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
            {
                hasher.update(duration.as_secs().to_string().as_bytes());
                hasher.update(duration.subsec_nanos().to_string().as_bytes());
            }
        }
        hasher.update([0]);
    }
    let hash = hasher.finalize();
    hex_prefix(&hash, 16)
}

pub(crate) fn hex_prefix(bytes: &[u8], chars: usize) -> String {
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
