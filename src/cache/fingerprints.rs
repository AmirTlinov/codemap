use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use super::fingerprint_delta::CacheFileDelta;
use super::git_probe::{
    current_git_head, current_git_status_has_untracked, git_path_is_ignored, git_tracked_paths,
};
use crate::model::Project;

const FINGERPRINT_CACHE_FORMAT: u32 = 8;

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
pub fn file_delta_for_known_changes(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    changed_or_added_candidates: &BTreeSet<String>,
    removed_candidates: &BTreeSet<String>,
) -> Option<CacheFileDelta> {
    let cached = read_valid_cached_fingerprints(root, cache_dir, version)?;
    if cached.git_head != current_git_head(root) {
        return None;
    }
    if !cached.git_status_probe_valid {
        return None;
    }
    file_delta_from_known_changes(
        root,
        &cached,
        changed_or_added_candidates,
        removed_candidates,
        DeltaRecheck::GitStatus,
    )
}

pub fn file_delta_for_head_change(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    changed_or_added_candidates: &BTreeSet<String>,
    removed_candidates: &BTreeSet<String>,
) -> Option<CacheFileDelta> {
    let cached = read_valid_cached_fingerprints(root, cache_dir, version)?;
    if cached.git_head.is_none() || cached.git_head == current_git_head(root) {
        return None;
    }
    file_delta_from_known_changes(
        root,
        &cached,
        changed_or_added_candidates,
        removed_candidates,
        DeltaRecheck::HeadChange,
    )
}

pub fn file_delta_by_rechecking_cached_files(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    changed_or_added_candidates: &BTreeSet<String>,
    removed_candidates: &BTreeSet<String>,
) -> Option<CacheFileDelta> {
    let cached = read_valid_cached_fingerprints(root, cache_dir, version)?;
    if cached.git_head != current_git_head(root) {
        return None;
    }
    file_delta_from_known_changes(
        root,
        &cached,
        changed_or_added_candidates,
        removed_candidates,
        DeltaRecheck::CachedFiles,
    )
}

pub fn cached_git_head(root: &Path, cache_dir: &Path, version: &str) -> Option<String> {
    read_valid_cached_fingerprints(root, cache_dir, version)?.git_head
}

#[derive(Clone, Copy)]
enum DeltaRecheck {
    GitStatus,
    HeadChange,
    CachedFiles,
}

fn file_delta_from_known_changes(
    root: &Path,
    cached: &CachedFingerprints,
    changed_or_added_candidates: &BTreeSet<String>,
    removed_candidates: &BTreeSet<String>,
    recheck: DeltaRecheck,
) -> Option<CacheFileDelta> {
    let cached_by_path = cached
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let mut verify_changed_or_added = changed_or_added_candidates.clone();
    let mut verify_removed = removed_candidates.clone();
    let include_cached_status =
        matches!(recheck, DeltaRecheck::GitStatus | DeltaRecheck::HeadChange)
            && cached.git_status_probe_valid;
    if include_cached_status {
        let cached_status_removed = cached
            .git_status_removed
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        verify_changed_or_added.extend(cached.git_status_changed_or_added.iter().cloned());
        verify_changed_or_added.extend(
            cached_status_removed
                .iter()
                .filter(|path| root.join(path).exists())
                .cloned(),
        );
        verify_removed.extend(
            cached_status_removed
                .iter()
                .filter(|path| !root.join(path).exists())
                .cloned(),
        );
    }
    let mut unchanged = cached_by_path
        .keys()
        .filter(|path| !verify_removed.contains(**path))
        .map(|path| (*path).to_string())
        .collect::<BTreeSet<_>>();
    let mut changed_or_added = BTreeSet::new();
    let mut removed = verify_removed
        .iter()
        .filter(|path| cached_by_path.contains_key(path.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();
    for rel in &verify_changed_or_added {
        unchanged.remove(rel);
        let Some(cached) = cached_by_path.get(rel.as_str()) else {
            if root.join(rel).exists() {
                changed_or_added.insert(rel.clone());
            }
            continue;
        };
        if cached_file_matches(root, rel, cached) {
            unchanged.insert(rel.clone());
        } else {
            changed_or_added.insert(rel.clone());
        }
    }
    for rel in &verify_changed_or_added {
        if !root.join(rel).exists() {
            changed_or_added.remove(rel);
            unchanged.remove(rel);
            if cached_by_path.contains_key(rel.as_str()) {
                removed.insert(rel.clone());
            }
        }
    }
    if matches!(recheck, DeltaRecheck::CachedFiles) {
        for cached_file in &cached.files {
            if verify_changed_or_added.contains(&cached_file.path)
                || verify_removed.contains(&cached_file.path)
            {
                continue;
            }
            recheck_cached_file(
                root,
                cached_file,
                &mut unchanged,
                &mut changed_or_added,
                &mut removed,
            );
        }
    }
    for cached_file in cached.files.iter().filter(|file| !file.git_tracked) {
        if changed_or_added_candidates.contains(&cached_file.path)
            || removed_candidates.contains(&cached_file.path)
        {
            continue;
        }
        if !root.join(&cached_file.path).exists() || git_path_is_ignored(root, &cached_file.path) {
            unchanged.remove(&cached_file.path);
            removed.insert(cached_file.path.clone());
        } else if !cached_file_matches(root, &cached_file.path, cached_file) {
            unchanged.remove(&cached_file.path);
            changed_or_added.insert(cached_file.path.clone());
        }
    }
    Some(cache_file_delta(
        cached,
        (unchanged, changed_or_added, removed),
    ))
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

fn recheck_cached_file(
    root: &Path,
    cached_file: &CachedFileFingerprint,
    unchanged: &mut BTreeSet<String>,
    changed_or_added: &mut BTreeSet<String>,
    removed: &mut BTreeSet<String>,
) {
    unchanged.remove(&cached_file.path);
    if !root.join(&cached_file.path).exists() {
        removed.insert(cached_file.path.clone());
    } else if cached_file_matches(root, &cached_file.path, cached_file) {
        unchanged.insert(cached_file.path.clone());
    } else {
        changed_or_added.insert(cached_file.path.clone());
    }
}

pub fn cached_git_head_matches(root: &Path, cache_dir: &Path, version: &str) -> Option<bool> {
    let cached = read_valid_cached_fingerprints(root, cache_dir, version)?;
    Some(cached.git_head.is_some() && cached.git_head == current_git_head(root))
}

pub(super) fn write_fingerprints(
    project: &Project,
    version: &str,
    git_status_change_sets: Option<(&BTreeSet<String>, &BTreeSet<String>)>,
) -> Result<()> {
    let tracked_paths = git_tracked_paths(&project.root);
    let (git_status_changed_or_added, git_status_removed) = git_status_change_sets
        .map(|(changed_or_added, removed)| {
            (
                changed_or_added.iter().cloned().collect(),
                removed.iter().cloned().collect(),
            )
        })
        .unwrap_or_default();
    let fingerprints = CachedFingerprints {
        format_version: FINGERPRINT_CACHE_FORMAT,
        version: version.to_string(),
        root: project.root.to_string_lossy().to_string(),
        git_head: current_git_head(&project.root),
        has_untracked: current_git_status_has_untracked(&project.root),
        git_status_probe_valid: git_status_change_sets.is_some(),
        git_status_changed_or_added,
        git_status_removed,
        fingerprint: super::fingerprint(project, None),
        files: project
            .files
            .values()
            .map(|file| {
                let modified = file_modified_parts(project, file);
                CachedFileFingerprint {
                    path: file.rel.clone(),
                    git_tracked: tracked_paths
                        .as_ref()
                        .is_some_and(|paths| paths.contains(&file.rel)),
                    size: file.size,
                    content_hash: file.content_hash.clone(),
                    modified_secs: modified.map(|parts| parts.0),
                    modified_nanos: modified.map(|parts| parts.1),
                }
            })
            .collect(),
    };
    let body = serde_json::to_string_pretty(&fingerprints)?;
    let body = format!("{body}\n");
    fs::write(project.cache_dir.join("fingerprints.json"), &body)?;
    // Persist a token-keyed snapshot so `--since <token>` can diff against the exact
    // state the agent saw. The dirty edit loop always writes fingerprints, so every
    // emitted snapshot token is backed by a snapshot file.
    super::snapshots::save(&project.cache_dir, &fingerprints.fingerprint, &body);
    Ok(())
}

fn read_cached_fingerprints(cache_dir: &Path) -> Option<CachedFingerprints> {
    let text = fs::read_to_string(cache_dir.join("fingerprints.json")).ok()?;
    serde_json::from_str(&text).ok()
}

fn read_valid_cached_fingerprints(
    root: &Path,
    cache_dir: &Path,
    version: &str,
) -> Option<CachedFingerprints> {
    let cached = read_cached_fingerprints(cache_dir)?;
    if cached.format_version != FINGERPRINT_CACHE_FORMAT {
        return None;
    }
    if cached.version != version || cached.root != root.to_string_lossy() {
        return None;
    }
    Some(cached)
}

#[derive(Deserialize, Serialize)]
struct CachedFingerprints {
    #[serde(default)]
    format_version: u32,
    version: String,
    root: String,
    #[serde(default)]
    git_head: Option<String>,
    #[serde(default)]
    has_untracked: bool,
    #[serde(default)]
    git_status_probe_valid: bool,
    #[serde(default)]
    git_status_changed_or_added: Vec<String>,
    #[serde(default)]
    git_status_removed: Vec<String>,
    fingerprint: String,
    files: Vec<CachedFileFingerprint>,
}

#[derive(Deserialize, Serialize)]
struct CachedFileFingerprint {
    path: String,
    #[serde(default)]
    git_tracked: bool,
    size: u64,
    #[serde(default)]
    content_hash: Option<String>,
    modified_secs: Option<u64>,
    modified_nanos: Option<u32>,
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
