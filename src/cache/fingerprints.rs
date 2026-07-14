// Responsibility: cache-fingerprints
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

pub use super::fingerprint_delta::CacheFileDelta;
use super::git_probe::current_git_head;
use crate::model::Project;

mod known_changes;
pub use known_changes::*;
mod store;
pub(crate) use store::*;

pub fn file_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    current_files: &[String],
    _config_path: Option<&str>,
) -> Option<CacheFileDelta> {
    let cached = read_valid_cached_fingerprints(root, cache_dir, version)?;
    Some(file_delta_from_cached(root, &cached, current_files))
}

fn file_delta_from_cached(
    root: &Path,
    cached: &CachedFingerprints,
    current_files: &[String],
) -> CacheFileDelta {
    let cached_by_path = cached
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let current_paths = current_files
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut unchanged = BTreeSet::new();
    let mut changed_or_added = BTreeSet::new();
    for rel in current_files {
        let Some(cached) = cached_by_path.get(rel.as_str()) else {
            changed_or_added.insert(rel.clone());
            continue;
        };
        if !cached_file_matches(root, rel, cached) {
            changed_or_added.insert(rel.clone());
            continue;
        }
        unchanged.insert(rel.clone());
    }
    let removed = cached_by_path
        .keys()
        .filter(|path| !current_paths.contains(**path))
        .map(|path| (*path).to_string())
        .collect();
    cache_file_delta(cached, (unchanged, changed_or_added, removed))
}

// Diff the current working tree against a saved agent snapshot (keyed by token),
// reusing the same per-file delta core. Returns None (fail-open) if the snapshot
// is missing, malformed, or from a different version/root (cleared cache or other
// machine).
pub fn snapshot_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    token: &str,
    current_files: &[String],
) -> Option<CacheFileDelta> {
    let text = fs::read_to_string(super::snapshots::snapshot_path(cache_dir, token)).ok()?;
    let cached: CachedFingerprints = serde_json::from_str(&text).ok()?;
    if cached.format_version != FINGERPRINT_CACHE_FORMAT {
        return None;
    }
    if cached.version != version || cached.root != root.to_string_lossy() {
        return None;
    }
    let delta = file_delta_from_cached(root, &cached, current_files);
    super::snapshots::touch(cache_dir, token);
    Some(delta)
}

pub fn cached_git_head(root: &Path, cache_dir: &Path, version: &str) -> Option<String> {
    read_valid_cached_fingerprints(root, cache_dir, version)?.git_head
}

pub fn cached_git_head_matches(root: &Path, cache_dir: &Path, version: &str) -> Option<bool> {
    let cached = read_valid_cached_fingerprints(root, cache_dir, version)?;
    Some(cached.git_head.is_some() && cached.git_head == current_git_head(root))
}

fn cache_file_delta(
    cached: &CachedFingerprints,
    sets: (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>),
) -> CacheFileDelta {
    let (unchanged, changed_or_added, removed) = sets;
    CacheFileDelta {
        cached_fingerprint: cached.fingerprint.clone(),
        cached_content_hashes: cached
            .files
            .iter()
            .map(|file| (file.path.clone(), file.content_hash.clone()))
            .collect(),
        unchanged,
        changed_or_added,
        removed,
    }
}

fn current_content_hash(path: impl AsRef<Path>) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let hash = Sha256::digest(&bytes);
    Some(super::hex_prefix(&hash, 16))
}

fn cached_file_matches(root: &Path, rel: &str, cached: &CachedFileFingerprint) -> bool {
    let Ok(meta) = fs::metadata(root.join(rel)) else {
        return false;
    };
    if meta.len() != cached.size {
        return false;
    }
    let (modified_secs, modified_nanos) = file_modified_parts_from_meta(&meta);
    if modified_secs == cached.modified_secs && modified_nanos == cached.modified_nanos {
        return true;
    }
    cached
        .content_hash
        .as_deref()
        .is_some_and(|hash| current_content_hash(root.join(rel)).as_deref() == Some(hash))
}

fn file_modified_parts(project: &Project, file: &crate::model::FileInfo) -> Option<(u64, u32)> {
    let meta = fs::metadata(file.rel_path(project)).ok()?;
    let (secs, nanos) = file_modified_parts_from_meta(&meta);
    Some((secs?, nanos?))
}

fn file_modified_parts_from_meta(meta: &fs::Metadata) -> (Option<u64>, Option<u32>) {
    let duration = meta
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok());
    (
        duration.map(|duration| duration.as_secs()),
        duration.map(|duration| duration.subsec_nanos()),
    )
}

trait RelPath {
    fn rel_path(&self, project: &Project) -> std::path::PathBuf;
}

impl RelPath for crate::model::FileInfo {
    fn rel_path(&self, project: &Project) -> std::path::PathBuf {
        project.root.join(&self.rel)
    }
}
