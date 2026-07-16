// Responsibility: external-session-snapshot-envelope-and-content-store
use super::fingerprints::{CachedFingerprints, FINGERPRINT_CACHE_FORMAT};
use crate::model::Project;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MAX_SNAPSHOTS: usize = 32;
const SNAPSHOT_TOKEN_LEN: usize = 16;
const SNAPSHOT_FORMAT: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SnapshotMetadata {
    pub format_version: u32,
    pub token: String,
    pub created_unix_seconds: u64,
    pub root: String,
    pub git_head: Option<String>,
    pub file_count: usize,
    pub content_files: usize,
    pub storage: String,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct SnapshotEnvelope {
    pub metadata: SnapshotMetadata,
    pub fingerprints: CachedFingerprints,
}

fn snapshots_dir(cache_dir: &Path) -> PathBuf {
    cache_dir.join("snapshots")
}

fn blobs_dir(cache_dir: &Path) -> PathBuf {
    snapshots_dir(cache_dir).join("blobs")
}

pub fn snapshot_path(cache_dir: &Path, token: &str) -> PathBuf {
    snapshots_dir(cache_dir).join(format!("{token}.json"))
}

fn blob_path(cache_dir: &Path, hash: &str) -> PathBuf {
    blobs_dir(cache_dir).join(format!("{hash}.txt"))
}

pub fn looks_like_snapshot_token(value: &str) -> bool {
    value.len() == SNAPSHOT_TOKEN_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn save(project: &Project, fingerprints: &CachedFingerprints, legacy_body: &str) {
    let token = &fingerprints.fingerprint;
    if !looks_like_snapshot_token(token) {
        return;
    }
    let dir = snapshots_dir(&project.cache_dir);
    if fs::create_dir_all(&dir).is_err()
        || fs::create_dir_all(blobs_dir(&project.cache_dir)).is_err()
    {
        return;
    }
    let content_files = persist_content_blobs(project);
    let created_unix_seconds = load(&project.cache_dir, token)
        .map(|snapshot| snapshot.metadata.created_unix_seconds)
        .unwrap_or_else(now_unix_seconds);
    let envelope = SnapshotEnvelope {
        metadata: SnapshotMetadata {
            format_version: SNAPSHOT_FORMAT,
            token: token.clone(),
            created_unix_seconds,
            root: fingerprints.root.clone(),
            git_head: fingerprints.git_head.clone(),
            file_count: fingerprints.files.len(),
            content_files,
            storage: "external_cache".to_string(),
        },
        fingerprints: fingerprints.clone(),
    };
    let body = serde_json::to_string_pretty(&envelope)
        .map(|body| format!("{body}\n"))
        .unwrap_or_else(|_| legacy_body.to_string());
    if super::io::write_cache_path(
        &project.cache_dir,
        &snapshot_path(&project.cache_dir, token),
        body,
    )
    .is_ok()
        && prune_to(&dir, MAX_SNAPSHOTS)
    {
        prune_unreferenced_blobs(&project.cache_dir);
    }
}

pub(crate) fn load(cache_dir: &Path, token: &str) -> Option<SnapshotEnvelope> {
    let text = fs::read_to_string(snapshot_path(cache_dir, token)).ok()?;
    let envelope = serde_json::from_str::<SnapshotEnvelope>(&text).ok()?;
    (envelope.metadata.format_version == SNAPSHOT_FORMAT
        && envelope.metadata.token == token
        && envelope.fingerprints.format_version == FINGERPRINT_CACHE_FORMAT)
        .then_some(envelope)
}

pub fn metadata(cache_dir: &Path, token: &str) -> Option<SnapshotMetadata> {
    load(cache_dir, token).map(|snapshot| snapshot.metadata)
}

pub(crate) fn content(cache_dir: &Path, hash: &str) -> Option<String> {
    if !valid_hash(hash) {
        return None;
    }
    let path = blob_path(cache_dir, hash);
    let text = fs::read_to_string(&path).ok()?;
    if crate::repo::scan_content_hash(text.as_bytes()) != hash {
        let _ = fs::remove_file(path);
        return None;
    }
    Some(text)
}

// Best-effort LRU refresh keeps a frequently used session baseline alive.
pub fn touch(cache_dir: &Path, token: &str) {
    let path = snapshot_path(cache_dir, token);
    if let Ok(body) = fs::read(&path) {
        let _ = super::io::write_cache_path(cache_dir, &path, body);
    }
}

fn persist_content_blobs(project: &Project) -> usize {
    let mut stored = 0usize;
    for file in project.files.values() {
        let Some(hash) = file.content_hash.as_deref().filter(|hash| valid_hash(hash)) else {
            continue;
        };
        let path = blob_path(&project.cache_dir, hash);
        if path.exists() {
            stored += 1;
            continue;
        }
        let Some(text) = project.read_indexed_text(&file.rel) else {
            continue;
        };
        // Blobs are immutable, content-addressed and verified on read. Writing
        // them without a per-file fsync keeps one snapshot from issuing
        // thousands of durability barriers; a torn blob simply fails open.
        if write_content_blob(&path, &text).is_ok() {
            stored += 1;
        }
    }
    stored
}

fn write_content_blob(path: &Path, text: &str) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(text.as_bytes())
}

fn prune_to(dir: &Path, max: usize) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                return None;
            }
            Some((path, entry.metadata().ok()?.modified().ok()?))
        })
        .collect();
    files.sort_by_key(|(_, mtime)| *mtime);
    let remove_count = files.len().saturating_sub(max);
    let removed = remove_count > 0;
    for (path, _) in files.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
    removed
}

fn prune_unreferenced_blobs(cache_dir: &Path) {
    let Ok(entries) = fs::read_dir(snapshots_dir(cache_dir)) else {
        return;
    };
    let mut retained = BTreeSet::new();
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(token) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if let Some(snapshot) = load(cache_dir, token) {
            retained.extend(
                snapshot
                    .fingerprints
                    .files
                    .into_iter()
                    .filter_map(|file| file.content_hash),
            );
        }
    }
    let Ok(blobs) = fs::read_dir(blobs_dir(cache_dir)) else {
        return;
    };
    for blob in blobs.filter_map(|entry| entry.ok()) {
        let retained_blob = blob
            .path()
            .file_stem()
            .and_then(|value| value.to_str())
            .is_some_and(|hash| retained.contains(hash));
        if !retained_blob {
            let _ = fs::remove_file(blob.path());
        }
    }
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 16 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_rejects_a_torn_blob() {
        let cache = tempfile::TempDir::new().expect("snapshot cache");
        fs::create_dir_all(blobs_dir(cache.path())).expect("blob directory");
        let expected = "complete snapshot body\n";
        let hash = crate::repo::scan_content_hash(expected.as_bytes());
        fs::write(blob_path(cache.path(), &hash), "partial").expect("torn blob");
        assert_eq!(content(cache.path(), &hash), None);
        assert!(!blob_path(cache.path(), &hash).exists());
        fs::write(blob_path(cache.path(), &hash), expected).expect("complete blob");
        assert_eq!(content(cache.path(), &hash).as_deref(), Some(expected));
    }
}
