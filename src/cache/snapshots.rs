// Agent snapshots: a per-call copy of the fingerprint set, keyed by the snapshot
// token (the full 16-hex fingerprint shown in the Map Snapshot line). They let
// `changed --since <token>` / `proof changed --since <token>` diff against the
// exact state the agent saw earlier, independent of git commits. Stored under the
// external cache only — zero target-repo footprint.
use std::fs;
use std::path::{Path, PathBuf};

pub const MAX_SNAPSHOTS: usize = 32;
const SNAPSHOT_TOKEN_LEN: usize = 16;

fn snapshots_dir(cache_dir: &Path) -> PathBuf {
    cache_dir.join("snapshots")
}

pub fn snapshot_path(cache_dir: &Path, token: &str) -> PathBuf {
    snapshots_dir(cache_dir).join(format!("{token}.json"))
}

pub fn looks_like_snapshot_token(value: &str) -> bool {
    value.len() == SNAPSHOT_TOKEN_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn save(cache_dir: &Path, token: &str, body: &str) {
    if !looks_like_snapshot_token(token) {
        return;
    }
    let dir = snapshots_dir(cache_dir);
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    if fs::write(snapshot_path(cache_dir, token), body).is_ok() {
        prune_to(&dir, MAX_SNAPSHOTS);
    }
}

// Best-effort LRU refresh: bump mtime by rewriting the (small) snapshot so a
// frequently used baseline is not pruned away.
pub fn touch(cache_dir: &Path, token: &str) {
    let path = snapshot_path(cache_dir, token);
    if let Ok(body) = fs::read(&path) {
        let _ = fs::write(&path, body);
    }
}

fn prune_to(dir: &Path, max: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(PathBuf, std::time::SystemTime)> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                return None;
            }
            let mtime = entry.metadata().ok()?.modified().ok()?;
            Some((path, mtime))
        })
        .collect();
    if files.len() <= max {
        return;
    }
    files.sort_by_key(|(_, mtime)| *mtime);
    let remove_count = files.len() - max;
    for (path, _) in files.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
}
