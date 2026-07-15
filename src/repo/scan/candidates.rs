// Responsibility: repo-scan-candidates-and-index-boundaries
use super::file_filters::{scan_file_rejection, scan_rejection_keeps_placeholder};
use crate::model::{FileInfo, IndexedBoundary, ScanInventoryBoundary};
use crate::repo::{COMMON_IGNORE_DIRS, ScanStatsBuilder, normalize_rel_path};
use ignore::WalkBuilder;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::git_index::{GitIndexInventory, GitIndexKind, GitIndexProbe, git_index_probe};

pub(crate) struct ScanCandidateInventory {
    pub(crate) rels: Vec<String>,
    pub(crate) git_index: Option<GitIndexInventory>,
    boundary_overrides: BTreeMap<String, IndexedBoundary>,
}

impl ScanCandidateInventory {
    pub(crate) fn boundary(&self, root: &Path, rel: &str) -> Option<IndexedBoundary> {
        self.boundary_overrides.get(rel).copied().or_else(|| {
            indexed_boundary_for_path(
                root,
                rel,
                self.git_index.as_ref().and_then(|index| index.kind(rel)),
            )
        })
    }

    pub(crate) fn path_boundaries(&self) -> impl Iterator<Item = &String> {
        self.boundary_overrides.keys()
    }
}

pub(crate) fn indexed_boundary_for_path(
    root: &Path,
    rel: &str,
    kind: Option<GitIndexKind>,
) -> Option<IndexedBoundary> {
    match kind {
        Some(GitIndexKind::Gitlink) => Some(IndexedBoundary::ExternalGitlink),
        Some(GitIndexKind::Symlink) => Some(IndexedBoundary::ExternalTree),
        Some(GitIndexKind::Regular) if should_ignore_rel(rel) => {
            Some(IndexedBoundary::IgnoredTrackedFile)
        }
        Some(GitIndexKind::Regular) => match fs::symlink_metadata(root.join(rel)) {
            Ok(metadata) if metadata.is_file() => None,
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Some(IndexedBoundary::ExternalTree)
            }
            Ok(_) | Err(_) => Some(IndexedBoundary::UnavailableTrackedFile),
        },
        None => match fs::symlink_metadata(root.join(rel)) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Some(IndexedBoundary::ExternalTree)
            }
            Ok(metadata) if metadata.is_dir() && fs::read_dir(root.join(rel)).is_err() => {
                Some(IndexedBoundary::TraversalError)
            }
            _ => None,
        },
    }
}

pub(crate) fn indexed_boundary(root: &Path, file: &FileInfo) -> Option<IndexedBoundary> {
    file.indexed_boundary.or_else(|| {
        fs::symlink_metadata(root.join(&file.rel))
            .ok()
            .filter(|metadata| metadata.file_type().is_symlink())
            .map(|_| IndexedBoundary::ExternalTree)
    })
}

pub(crate) fn is_external_tree_boundary(root: &Path, file: &FileInfo) -> bool {
    matches!(
        indexed_boundary(root, file),
        Some(IndexedBoundary::ExternalTree | IndexedBoundary::ExternalGitlink)
    )
}

pub(crate) fn is_incomplete_indexed_boundary(root: &Path, file: &FileInfo) -> bool {
    indexed_boundary(root, file).is_some()
}

pub(crate) fn cache_candidate_files(root: &Path) -> Vec<String> {
    let mut stats = ScanStatsBuilder::default();
    let inventory = scan_candidate_inventory(root, &mut stats);
    let ScanCandidateInventory {
        rels,
        git_index,
        boundary_overrides,
    } = inventory;
    let mut files = rels
        .into_iter()
        .filter(|rel| {
            boundary_overrides.contains_key(rel)
                || git_index
                    .as_ref()
                    .is_some_and(|index| index.kind(rel).is_some())
                || is_cache_candidate_file(root, rel)
                || git_index.as_ref().is_some_and(|index| {
                    index.kind(rel).is_some()
                        && indexed_boundary_for_path(root, rel, index.kind(rel)).is_some()
                })
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

pub(crate) fn structural_inventory_candidate_files(root: &Path) -> Vec<String> {
    cache_candidate_files(root)
}

pub(crate) fn list_visible_candidate_files(root: &Path) -> Vec<String> {
    let mut stats = ScanStatsBuilder::default();
    scan_candidate_inventory(root, &mut stats)
        .rels
        .into_iter()
        .filter(|rel| !should_ignore_rel(rel))
        .collect()
}

pub(crate) fn is_cache_candidate_file(root: &Path, rel: &str) -> bool {
    if should_ignore_rel(rel) {
        return false;
    }
    let path = root.join(rel);
    fs::symlink_metadata(&path).ok().is_some_and(|meta| {
        if meta.file_type().is_symlink() {
            return true;
        }
        if !meta.is_file() {
            return false;
        }
        match scan_file_rejection(&path, rel, meta.len()) {
            None => true,
            Some(reason) => scan_rejection_keeps_placeholder(&path, rel, reason),
        }
    })
}

pub(crate) fn is_cache_candidate_with_index(
    root: &Path,
    rel: &str,
    git_index: &GitIndexInventory,
) -> bool {
    if git_index.kind(rel).is_some() {
        return true;
    }
    is_cache_candidate_file(root, rel) || indexed_boundary_for_path(root, rel, None).is_some()
}

pub(crate) fn scan_candidate_inventory(
    root: &Path,
    stats: &mut ScanStatsBuilder,
) -> ScanCandidateInventory {
    let (git_index, index_unavailable) = match git_index_probe(root) {
        GitIndexProbe::Available(index) => (Some(index), false),
        GitIndexProbe::Unavailable => (None, true),
        GitIndexProbe::NotRepository => (None, false),
    };
    let gitlinks = git_index
        .as_ref()
        .map(GitIndexInventory::gitlinks)
        .unwrap_or_default();
    let mut rels = git_index
        .as_ref()
        .map(|index| index.paths().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let (walked, traversal_errors) = walk_files(root, stats, &gitlinks);
    rels.extend(walked);
    let boundary_overrides = traversal_errors
        .into_iter()
        .map(|path| (path, IndexedBoundary::TraversalError))
        .collect::<BTreeMap<_, _>>();
    if index_unavailable {
        stats.record_inventory_boundary(ScanInventoryBoundary::GitIndexUnavailable);
    }
    rels.extend(boundary_overrides.keys().cloned());
    ScanCandidateInventory {
        rels: rels.into_iter().collect(),
        git_index,
        boundary_overrides,
    }
}

fn walk_files(
    root: &Path,
    stats: &mut ScanStatsBuilder,
    gitlinks: &BTreeSet<String>,
) -> (Vec<String>, BTreeSet<String>) {
    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(true)
        .hidden(false)
        .follow_links(false);
    let root_for_filter = root.to_path_buf();
    let gitlinks = gitlinks.clone();
    let ignored_entries = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
    let ignored_for_filter = Arc::clone(&ignored_entries);
    builder.filter_entry(move |entry| {
        entry
            .path()
            .strip_prefix(&root_for_filter)
            .ok()
            .map(|path| {
                let rel = normalize_rel_path(&path.to_string_lossy());
                if rel.is_empty() {
                    return true;
                }
                if gitlinks.contains(&rel) {
                    return false;
                }
                if let Some(reason) = ignore_reason(&rel) {
                    if let Ok(mut entries) = ignored_for_filter.lock() {
                        entries.push((reason, rel));
                    }
                    return false;
                }
                true
            })
            .unwrap_or(true)
    });
    let mut out = Vec::new();
    let mut traversal_errors = BTreeSet::new();
    for result in builder.build() {
        match result {
            Ok(entry)
                if entry
                    .file_type()
                    .is_some_and(|kind| kind.is_file() || kind.is_symlink()) =>
            {
                if let Ok(path) = entry.path().strip_prefix(root)
                    && !path.as_os_str().is_empty()
                {
                    out.push(normalize_rel_path(&path.to_string_lossy()));
                }
            }
            Ok(_) => {}
            Err(error) => {
                if !record_walk_error_paths(root, &error, &mut traversal_errors) {
                    stats.record_inventory_boundary(
                        ScanInventoryBoundary::FilesystemTraversalUnavailable,
                    );
                }
            }
        }
    }
    if let Ok(entries) = ignored_entries.lock() {
        for (reason, rel) in entries.iter() {
            stats.record_ignored(reason, rel);
        }
    }
    (out, traversal_errors)
}

/// Returns true only when every underlying walker error can be bound to a
/// concrete repository-relative path. A false result must become a scan-wide
/// inventory boundary: silently dropping a root/pathless error would certify a
/// partial traversal as complete.
fn record_walk_error_paths(root: &Path, error: &ignore::Error, out: &mut BTreeSet<String>) -> bool {
    match error {
        ignore::Error::Partial(errors) => errors
            .iter()
            .all(|error| record_walk_error_paths(root, error, out)),
        ignore::Error::WithLineNumber { err, .. } | ignore::Error::WithDepth { err, .. } => {
            record_walk_error_paths(root, err, out)
        }
        ignore::Error::WithPath { path, .. } => {
            if let Ok(path) = path.strip_prefix(root) {
                if path.as_os_str().is_empty() {
                    return false;
                }
                let rel = normalize_rel_path(&path.to_string_lossy());
                out.insert(rel);
                return true;
            }
            false
        }
        ignore::Error::Loop { child, .. } => {
            if let Ok(path) = child.strip_prefix(root) {
                if path.as_os_str().is_empty() {
                    return false;
                }
                let rel = normalize_rel_path(&path.to_string_lossy());
                out.insert(rel);
                return true;
            }
            false
        }
        _ => false,
    }
}

pub(crate) fn should_ignore_rel(rel: &str) -> bool {
    ignore_reason(rel).is_some()
}

pub(crate) fn ignore_reason(rel: &str) -> Option<String> {
    rel.split('/').find_map(|part| {
        COMMON_IGNORE_DIRS
            .iter()
            .any(|ignored| ignored == &part)
            .then(|| format!("common_ignore_dir:{part}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn pathless_walker_errors_require_a_global_inventory_boundary() {
        let root = Path::new("/repo");
        let mut paths = BTreeSet::new();
        let error = ignore::Error::Partial(vec![
            ignore::Error::WithPath {
                path: root.join("blocked"),
                err: Box::new(ignore::Error::Io(io::Error::other("denied"))),
            },
            ignore::Error::Io(io::Error::other("root unavailable")),
        ]);

        assert!(!record_walk_error_paths(root, &error, &mut paths));
        assert_eq!(paths, BTreeSet::from(["blocked".to_string()]));
    }
}
