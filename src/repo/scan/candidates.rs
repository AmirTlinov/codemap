// Responsibility: repo-scan-candidates
use super::file_filters::{
    scan_file_rejection, scan_rejection_keeps_placeholder, source_symlink_keeps_placeholder,
};
use crate::repo::{COMMON_IGNORE_DIRS, ScanStatsBuilder, normalize_rel_path};
use ignore::WalkBuilder;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub(crate) fn cache_candidate_files(root: &Path) -> Vec<String> {
    let candidates =
        git_cache_candidate_files(root).unwrap_or_else(|| list_visible_candidate_files(root));
    let mut files = candidates
        .into_iter()
        .filter(|rel| is_cache_candidate_file(root, rel))
        .collect::<Vec<_>>();
    files.sort();
    files
}

pub(crate) fn structural_inventory_candidate_files(root: &Path) -> Vec<String> {
    cache_candidate_files(root)
}

pub(crate) fn is_cache_candidate_file(root: &Path, rel: &str) -> bool {
    if should_ignore_rel(rel) {
        return false;
    }
    let path = root.join(rel);
    fs::symlink_metadata(&path).ok().is_some_and(|meta| {
        if meta.file_type().is_symlink() {
            return source_symlink_keeps_placeholder(&path);
        }
        if !meta.is_file() {
            return false;
        }
        match scan_file_rejection(&path, meta.len()) {
            None => true,
            Some(reason) => scan_rejection_keeps_placeholder(&path, reason),
        }
    })
}

fn git_cache_candidate_files(root: &Path) -> Option<Vec<String>> {
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-c", "-o", "--exclude-standard"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(normalize_rel_path)
            .filter(|rel| !rel.is_empty() && !should_ignore_rel(rel))
            .collect(),
    )
}

fn list_candidate_files(root: &Path) -> Vec<String> {
    let mut stats = ScanStatsBuilder::default();
    list_candidate_files_with_stats(root, &mut stats)
}

pub(crate) fn list_visible_candidate_files(root: &Path) -> Vec<String> {
    list_candidate_files(root)
        .into_iter()
        .filter(|rel| !should_ignore_rel(rel))
        .collect()
}

pub(crate) fn list_candidate_files_with_stats(
    root: &Path,
    stats: &mut ScanStatsBuilder,
) -> Vec<String> {
    git_list_files(root, stats).unwrap_or_else(|| walk_files(root, stats))
}

fn git_list_files(root: &Path, stats: &mut ScanStatsBuilder) -> Option<Vec<String>> {
    let output = crate::repo::read_only_git_command()
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-c", "--exclude-standard"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut rels = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(normalize_rel_path)
        .filter(|rel| !rel.is_empty())
        .collect::<BTreeSet<_>>();
    rels.extend(walk_files(root, stats));
    Some(rels.into_iter().collect())
}

fn walk_files(root: &Path, stats: &mut ScanStatsBuilder) -> Vec<String> {
    let mut builder = WalkBuilder::new(root);
    builder
        .standard_filters(true)
        .hidden(false)
        .follow_links(false);
    let root_for_filter = root.to_path_buf();
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
    let out = builder
        .build()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .map(|kind| kind.is_file() || kind.is_symlink())
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            entry
                .path()
                .strip_prefix(root)
                .ok()
                .map(|p| normalize_rel_path(&p.to_string_lossy()))
        })
        .collect();
    if let Ok(entries) = ignored_entries.lock() {
        for (reason, rel) in entries.iter() {
            stats.record_ignored(reason, rel);
        }
    }
    out
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
