// Responsibility: cache-fingerprints-known-changes
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{
    CacheFileDelta, CachedFileFingerprint, CachedFingerprints, cache_file_delta,
    cached_file_content_matches, cached_file_matches, read_valid_cached_fingerprints,
};
use crate::cache::git_probe::{current_git_head, git_path_is_ignored};
use crate::repo::{
    GitIndexInventory, git_index_inventory, indexed_boundary_for_path, is_cache_candidate_file,
};

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
    let git_index = git_index_inventory(root)?;
    let cached_by_path = cached
        .files
        .iter()
        .map(|file| (file.path.as_str(), file))
        .collect::<BTreeMap<_, _>>();
    let mut verify_changed_or_added = changed_or_added_candidates.clone();
    // Status is a working-tree delta, not an inventory-set delta. After an
    // unavailable index has produced a deliberately partial cache, restoring
    // the same index/HEAD is status-clean; compare the current index owner to
    // the cached set so its previously hidden tracked paths re-enter scanning.
    verify_changed_or_added.extend(
        git_index
            .paths()
            .filter(|path| !cached_by_path.contains_key(path.as_str()))
            .filter(|path| crate::repo::is_cache_candidate_with_index(root, path, &git_index)),
    );
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
                .filter(|path| candidate_exists(root, path, &git_index))
                .cloned(),
        );
        verify_removed.extend(
            cached_status_removed
                .iter()
                .filter(|path| !candidate_exists(root, path, &git_index))
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
            if candidate_exists(root, rel, &git_index) {
                changed_or_added.insert(rel.clone());
            }
            continue;
        };
        // Git already identified this path as changed. Its old size and mtime
        // can be restored deliberately (or collide on a coarse filesystem), so
        // only the cached content hash may turn it back into an unchanged file.
        if cached_file_content_matches(root, rel, cached, current_boundary(root, rel, &git_index)) {
            unchanged.insert(rel.clone());
        } else {
            changed_or_added.insert(rel.clone());
        }
    }
    for rel in &verify_changed_or_added {
        if !candidate_exists(root, rel, &git_index) {
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
                &git_index,
                &mut unchanged,
                &mut changed_or_added,
                &mut removed,
            );
        }
    }
    if !matches!(recheck, DeltaRecheck::CachedFiles) {
        for cached_file in cached.files.iter().filter(|file| file.git_tracked) {
            if verify_changed_or_added.contains(&cached_file.path)
                || verify_removed.contains(&cached_file.path)
            {
                continue;
            }
            let boundary = current_boundary(root, &cached_file.path, &git_index);
            let needs_content_recheck = git_index.needs_content_recheck(&cached_file.path);
            if boundary != cached_file.indexed_boundary
                || (needs_content_recheck
                    && !cached_file_matches(root, &cached_file.path, cached_file, boundary, true))
            {
                unchanged.remove(&cached_file.path);
                changed_or_added.insert(cached_file.path.clone());
            }
        }
    }
    for cached_file in cached.files.iter().filter(|file| !file.git_tracked) {
        if changed_or_added_candidates.contains(&cached_file.path)
            || removed_candidates.contains(&cached_file.path)
        {
            continue;
        }
        let boundary = indexed_boundary_for_path(root, &cached_file.path, None);
        if git_path_is_ignored(root, &cached_file.path)
            || (!is_cache_candidate_file(root, &cached_file.path) && boundary.is_none())
        {
            unchanged.remove(&cached_file.path);
            removed.insert(cached_file.path.clone());
        } else if !cached_file_matches(root, &cached_file.path, cached_file, boundary, true) {
            unchanged.remove(&cached_file.path);
            changed_or_added.insert(cached_file.path.clone());
        }
    }
    Some(cache_file_delta(
        cached,
        (unchanged, changed_or_added, removed),
    ))
}

fn recheck_cached_file(
    root: &Path,
    cached_file: &CachedFileFingerprint,
    git_index: &GitIndexInventory,
    unchanged: &mut BTreeSet<String>,
    changed_or_added: &mut BTreeSet<String>,
    removed: &mut BTreeSet<String>,
) {
    unchanged.remove(&cached_file.path);
    if !candidate_exists(root, &cached_file.path, git_index) {
        removed.insert(cached_file.path.clone());
    } else if cached_file_matches(
        root,
        &cached_file.path,
        cached_file,
        current_boundary(root, &cached_file.path, git_index),
        !cached_file.git_tracked || git_index.needs_content_recheck(&cached_file.path),
    ) {
        unchanged.insert(cached_file.path.clone());
    } else {
        changed_or_added.insert(cached_file.path.clone());
    }
}

fn current_boundary(
    root: &Path,
    rel: &str,
    git_index: &GitIndexInventory,
) -> Option<crate::model::IndexedBoundary> {
    indexed_boundary_for_path(root, rel, git_index.kind(rel))
}

fn candidate_exists(root: &Path, rel: &str, git_index: &GitIndexInventory) -> bool {
    git_index.kind(rel).is_some()
        || is_cache_candidate_file(root, rel)
        || indexed_boundary_for_path(root, rel, None).is_some()
}
