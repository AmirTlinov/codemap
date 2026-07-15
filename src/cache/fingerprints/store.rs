// Responsibility: cache-fingerprints-store
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::file_modified_parts;
use crate::cache::git_probe::{
    current_git_head, current_git_status_has_untracked, git_tracked_paths,
};
use crate::model::Project;

pub(crate) const FINGERPRINT_CACHE_FORMAT: u32 = 8;

pub(crate) fn format_version() -> u32 {
    FINGERPRINT_CACHE_FORMAT
}

pub(crate) fn write_fingerprints(
    project: &Project,
    version: &str,
    git_status_change_sets: Option<(
        &std::collections::BTreeSet<String>,
        &std::collections::BTreeSet<String>,
    )>,
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
        fingerprint: crate::cache::fingerprint(project, None),
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
    crate::cache::snapshots::save(&project.cache_dir, &fingerprints.fingerprint, &body);
    Ok(())
}

fn read_cached_fingerprints(cache_dir: &Path) -> Option<CachedFingerprints> {
    let text = fs::read_to_string(cache_dir.join("fingerprints.json")).ok()?;
    serde_json::from_str(&text).ok()
}

pub(crate) fn read_valid_cached_fingerprints(
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
pub(crate) struct CachedFingerprints {
    #[serde(default)]
    pub(crate) format_version: u32,
    pub(crate) version: String,
    pub(crate) root: String,
    #[serde(default)]
    pub(crate) git_head: Option<String>,
    #[serde(default)]
    pub(crate) has_untracked: bool,
    #[serde(default)]
    pub(crate) git_status_probe_valid: bool,
    #[serde(default)]
    pub(crate) git_status_changed_or_added: Vec<String>,
    #[serde(default)]
    pub(crate) git_status_removed: Vec<String>,
    pub(crate) fingerprint: String,
    pub(crate) files: Vec<CachedFileFingerprint>,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct CachedFileFingerprint {
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) git_tracked: bool,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) content_hash: Option<String>,
    pub(crate) modified_secs: Option<u64>,
    pub(crate) modified_nanos: Option<u32>,
}
