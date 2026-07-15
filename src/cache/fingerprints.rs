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
    let mut scan_stats = crate::repo::ScanStatsBuilder::default();
    let candidates = crate::repo::scan_candidate_inventory(root, &mut scan_stats);
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
        let boundary = candidates.boundary(root, rel);
        let needs_content_recheck = !cached.git_tracked
            || candidates
                .git_index
                .as_ref()
                .is_some_and(|index| index.needs_content_recheck(rel));
        if !cached_file_matches(root, rel, cached, boundary, needs_content_recheck) {
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
pub struct SnapshotDelta {
    pub files: CacheFileDelta,
    pub metadata: super::snapshots::SnapshotMetadata,
    pub base_texts: BTreeMap<String, String>,
    pub base_files: BTreeMap<String, CachedFileFingerprint>,
    pub content_complete: bool,
}

pub fn snapshot_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    token: &str,
    current_files: &[String],
) -> Option<SnapshotDelta> {
    let snapshot = super::snapshots::load(cache_dir, token)?;
    let cached = snapshot.fingerprints;
    if cached.version != version || cached.root != root.to_string_lossy() {
        return None;
    }
    let files = file_delta_from_cached(root, &cached, current_files);
    let changed_paths = files
        .changed_or_added
        .iter()
        .chain(files.removed.iter())
        .collect::<BTreeSet<_>>();
    let base_files = cached
        .files
        .iter()
        .filter(|file| changed_paths.contains(&file.path))
        .map(|file| (file.path.clone(), file.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut content_complete = true;
    let base_texts = base_files
        .values()
        .filter_map(|file| {
            let hash = file.content_hash.as_deref()?;
            let text = super::snapshots::content(cache_dir, hash);
            content_complete &= text.is_some();
            text.map(|text| (file.path.clone(), text))
        })
        .collect();
    super::snapshots::touch(cache_dir, token);
    Some(SnapshotDelta {
        files,
        metadata: snapshot.metadata,
        base_texts,
        base_files,
        content_complete,
    })
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

fn cached_file_matches(
    root: &Path,
    rel: &str,
    cached: &CachedFileFingerprint,
    current_boundary: Option<crate::model::IndexedBoundary>,
    needs_content_recheck: bool,
) -> bool {
    if cached.indexed_boundary != current_boundary {
        return false;
    }
    if current_boundary.is_some() {
        return true;
    }
    let Ok(meta) = fs::symlink_metadata(root.join(rel)) else {
        return false;
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return false;
    }
    let path = root.join(rel);
    let readable = fs::File::open(&path).is_ok();
    // A matching stat tuple cannot prove that a previously readable body is
    // still readable (permissions/ACLs may change without touching mtime).
    // Probe readability before taking the fast path so stale body facts never
    // survive an unreadable transition.
    if cached.content_hash.is_some() && !readable {
        return false;
    }
    // Conversely, a small supported file cached without body facts represents
    // a failed read, not a permanent parser placeholder. Once it becomes
    // readable it must be rescanned even when size/mtime are unchanged.
    let expects_body = !crate::repo::source_parser_requires_placeholder(&path)
        && crate::repo::scan_file_rejection(&path, rel, meta.len()).is_none();
    if cached.content_hash.is_none() && expects_body && readable {
        return false;
    }
    if meta.len() != cached.size {
        return false;
    }
    let (modified_secs, modified_nanos) = file_modified_parts_from_meta(&meta);
    if !needs_content_recheck
        && modified_secs == cached.modified_secs
        && modified_nanos == cached.modified_nanos
    {
        return true;
    }
    cached
        .content_hash
        .as_deref()
        .is_some_and(|hash| current_content_hash(root.join(rel)).as_deref() == Some(hash))
}

fn cached_file_content_matches(
    root: &Path,
    rel: &str,
    cached: &CachedFileFingerprint,
    current_boundary: Option<crate::model::IndexedBoundary>,
) -> bool {
    if cached.indexed_boundary != current_boundary || current_boundary.is_some() {
        return false;
    }
    let Ok(meta) = fs::symlink_metadata(root.join(rel)) else {
        return false;
    };
    meta.is_file()
        && !meta.file_type().is_symlink()
        && meta.len() == cached.size
        && cached
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn stat_fast_path_tracks_readability_transitions_in_both_directions() {
        let root = tempfile::TempDir::new().expect("fingerprint root");
        let rel = "src/app.ts";
        let path = root.path().join(rel);
        fs::create_dir_all(path.parent().expect("source parent")).expect("source parent");
        fs::write(&path, "app.get('/cached', handler);\n").expect("source body");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("deny body read");
        assert!(
            fs::File::open(&path).is_err(),
            "fixture must deny body reads"
        );
        let meta = fs::symlink_metadata(&path).expect("source metadata");
        let (modified_secs, modified_nanos) = file_modified_parts_from_meta(&meta);
        let readable_cache = CachedFileFingerprint {
            path: rel.to_string(),
            node_kind: "file".to_string(),
            git_tracked: true,
            size: meta.len(),
            indexed_boundary: None,
            content_hash: Some("previously-readable".to_string()),
            modified_secs,
            modified_nanos,
        };
        assert!(
            !cached_file_matches(root.path(), rel, &readable_cache, None, false),
            "matching stat metadata cannot retain facts after readability is lost"
        );

        let unreadable_cache = CachedFileFingerprint {
            content_hash: None,
            ..readable_cache
        };
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("restore body read");
        assert!(
            !cached_file_matches(root.path(), rel, &unreadable_cache, None, false),
            "restored readable bodies must replace an unreadable placeholder"
        );
    }
}
