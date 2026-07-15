// Responsibility: cache-identity
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::model::Project;

mod runtime_scope;
pub(crate) use runtime_scope::runtime_scope_fingerprint_from_project_snapshot;
pub use runtime_scope::{
    runtime_scope_fingerprint, runtime_scope_has_unindexed_entries,
    runtime_scope_is_logically_empty,
};

const CACHE_ARTIFACTS: &[&str] = &[
    "status.json",
    "inventory.json",
    "graph.json",
    "fingerprints.json",
    "reverse-imports.json",
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
        if let Some(boundary) = file.indexed_boundary {
            hasher.update(b"indexed_boundary");
            hasher.update(indexed_boundary_fingerprint_token(boundary));
        } else if let Some(content_hash) = &file.content_hash {
            hasher.update(b"content");
            hasher.update(content_hash.as_bytes());
        } else if let Ok(meta) = std::fs::symlink_metadata(file.rel_path(project)) {
            if meta.file_type().is_symlink() {
                hasher.update(b"symlink");
                if let Ok(target) = std::fs::read_link(file.rel_path(project)) {
                    hasher.update(target.to_string_lossy().as_bytes());
                }
            } else {
                // Unparsed placeholders depend on path/size, not incidental mtime churn.
                hasher.update(b"placeholder");
            }
        }
    }
    if let Some(path) = &project.config_path {
        hasher.update(path.as_bytes());
    }
    let mut inventory_boundaries = project.scan_stats.inventory_boundaries.clone();
    inventory_boundaries.sort();
    inventory_boundaries.dedup();
    for boundary in inventory_boundaries {
        hasher.update(b"scan_inventory_boundary");
        hasher.update(scan_inventory_boundary_fingerprint_token(boundary));
        hasher.update([0]);
    }
    let hash = hasher.finalize();
    hex_prefix(&hash, 16)
}

pub fn inventory_fingerprint(root: &Path, files: &[String]) -> String {
    let mut hasher = Sha256::new();
    let git_index = crate::repo::git_index_inventory(root);
    hasher.update(root.to_string_lossy().as_bytes());
    hasher.update([0]);
    hasher.update(b"git_index_probe");
    hasher.update(if git_index.is_some() {
        b"available".as_slice()
    } else if crate::repo::is_git_repo(root) {
        b"unavailable".as_slice()
    } else {
        b"not_repository".as_slice()
    });
    hasher.update([0]);
    let mut files = files.to_vec();
    files.sort();
    for rel in files {
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        let boundary = crate::repo::indexed_boundary_for_path(
            root,
            &rel,
            git_index.as_ref().and_then(|index| index.kind(&rel)),
        );
        if let Some(boundary) = boundary {
            hasher.update(indexed_boundary_fingerprint_token(boundary));
            hasher.update([0]);
            continue;
        }
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

fn indexed_boundary_fingerprint_token(boundary: crate::model::IndexedBoundary) -> &'static [u8] {
    match boundary {
        crate::model::IndexedBoundary::ExternalTree => b"external_tree",
        crate::model::IndexedBoundary::ExternalGitlink => b"external_gitlink",
        crate::model::IndexedBoundary::IgnoredTrackedFile => b"ignored_tracked_file",
        crate::model::IndexedBoundary::TraversalError => b"traversal_error",
        crate::model::IndexedBoundary::UnavailableTrackedFile => b"unavailable_tracked_file",
    }
}

fn scan_inventory_boundary_fingerprint_token(
    boundary: crate::model::ScanInventoryBoundary,
) -> &'static [u8] {
    match boundary {
        crate::model::ScanInventoryBoundary::FilesystemTraversalUnavailable => {
            b"filesystem_traversal_unavailable"
        }
        crate::model::ScanInventoryBoundary::GitIndexUnavailable => b"git_index_unavailable",
    }
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
