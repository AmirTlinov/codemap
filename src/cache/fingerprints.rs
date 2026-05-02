use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::Project;

const FINGERPRINT_CACHE_FORMAT: u32 = 2;

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
}

pub fn file_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    current_files: &[String],
    _config_path: Option<&str>,
) -> Option<CacheFileDelta> {
    let cached = read_cached_fingerprints(cache_dir)?;
    if cached.format_version != FINGERPRINT_CACHE_FORMAT {
        return None;
    }
    if cached.version != version || cached.root != root.to_string_lossy() {
        return None;
    }
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
        let meta = fs::metadata(root.join(rel)).ok()?;
        if meta.len() != cached.size {
            changed_or_added.insert(rel.clone());
            continue;
        }
        let (modified_secs, modified_nanos) = file_modified_parts_from_meta(&meta);
        if modified_secs != cached.modified_secs || modified_nanos != cached.modified_nanos {
            if cached
                .content_hash
                .as_deref()
                .is_some_and(|hash| current_content_hash(root.join(rel)).as_deref() == Some(hash))
            {
                unchanged.insert(rel.clone());
            } else {
                changed_or_added.insert(rel.clone());
            }
        } else {
            unchanged.insert(rel.clone());
        }
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

pub(super) fn write_fingerprints(project: &Project, version: &str) -> Result<()> {
    let fingerprints = CachedFingerprints {
        format_version: FINGERPRINT_CACHE_FORMAT,
        version: version.to_string(),
        root: project.root.to_string_lossy().to_string(),
        fingerprint: super::fingerprint(project, None),
        files: project
            .files
            .values()
            .map(|file| {
                let modified = file_modified_parts(project, file);
                CachedFileFingerprint {
                    path: file.rel.clone(),
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

#[derive(Deserialize, Serialize)]
struct CachedFingerprints {
    #[serde(default)]
    format_version: u32,
    version: String,
    root: String,
    fingerprint: String,
    files: Vec<CachedFileFingerprint>,
}

#[derive(Deserialize, Serialize)]
struct CachedFileFingerprint {
    path: String,
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
