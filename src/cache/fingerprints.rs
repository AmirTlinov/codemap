use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::Project;

const FINGERPRINT_CACHE_FORMAT: u32 = 4;

pub struct CacheFileDelta {
    pub cached_fingerprint: String,
    pub unchanged: BTreeSet<String>,
    pub changed_or_added: BTreeSet<String>,
    pub removed: BTreeSet<String>,
}

impl CacheFileDelta {
    pub fn is_exact_hit(&self) -> bool {
        self.changed_or_added.is_empty() && self.removed.is_empty()
    }

    pub fn current_file_count(&self) -> usize {
        self.unchanged.len() + self.changed_or_added.len()
    }
}

pub fn file_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    current_files: &[String],
    _config_path: Option<&str>,
) -> Option<CacheFileDelta> {
    let cached = read_valid_cached_fingerprints(root, cache_dir, version)?;
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
    Some(CacheFileDelta {
        cached_fingerprint: cached.fingerprint,
        unchanged,
        changed_or_added,
        removed,
    })
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
    let cached_by_path = cached
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let mut unchanged = cached_by_path
        .keys()
        .filter(|path| !removed_candidates.contains(**path))
        .map(|path| (*path).to_string())
        .collect::<BTreeSet<_>>();
    let mut changed_or_added = BTreeSet::new();
    let mut removed = removed_candidates
        .iter()
        .filter(|path| cached_by_path.contains_key(path.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();
    for rel in changed_or_added_candidates {
        unchanged.remove(rel);
        let Some(cached) = cached_by_path.get(rel.as_str()) else {
            changed_or_added.insert(rel.clone());
            continue;
        };
        if cached_file_matches(root, rel, cached) {
            unchanged.insert(rel.clone());
        } else {
            changed_or_added.insert(rel.clone());
        }
    }
    for rel in changed_or_added_candidates {
        if !root.join(rel).exists() {
            changed_or_added.remove(rel);
            unchanged.remove(rel);
            if cached_by_path.contains_key(rel.as_str()) {
                removed.insert(rel.clone());
            }
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
    Some(CacheFileDelta {
        cached_fingerprint: cached.fingerprint,
        unchanged,
        changed_or_added,
        removed,
    })
}

pub fn cached_git_head_matches(root: &Path, cache_dir: &Path, version: &str) -> Option<bool> {
    let cached = read_valid_cached_fingerprints(root, cache_dir, version)?;
    Some(cached.git_head.is_some() && cached.git_head == current_git_head(root))
}

pub(super) fn write_fingerprints(project: &Project, version: &str) -> Result<()> {
    let tracked_paths = git_tracked_paths(&project.root);
    let fingerprints = CachedFingerprints {
        format_version: FINGERPRINT_CACHE_FORMAT,
        version: version.to_string(),
        root: project.root.to_string_lossy().to_string(),
        git_head: current_git_head(&project.root),
        has_untracked: current_git_status_has_untracked(&project.root),
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
    fs::write(
        project.cache_dir.join("fingerprints.json"),
        format!("{body}\n"),
    )?;
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

fn current_git_head(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let head = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!head.is_empty()).then_some(head)
}

fn current_git_status_has_untracked(root: &Path) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "-uall", "--", "."])
        .output();
    let Ok(output) = output else {
        return true;
    };
    if !output.status.success() {
        return true;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.starts_with("?? "))
}

fn git_tracked_paths(root: &Path) -> Option<BTreeSet<String>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-c"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let rel = line.trim();
                (!rel.is_empty()).then(|| rel.replace('\\', "/"))
            })
            .collect(),
    )
}

fn git_path_is_ignored(root: &Path, rel: &str) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "-q", "--", rel])
        .output();
    output.is_ok_and(|output| output.status.success())
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
