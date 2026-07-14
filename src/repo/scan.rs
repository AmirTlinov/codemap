// Responsibility: repo-scan
mod candidates;
mod file_filters;
mod stats;

pub(crate) use candidates::*;
pub(crate) use file_filters::*;
pub(crate) use stats::*;

use crate::model::{FileInfo, ScanStats};
use crate::repo::{
    classify_roles, extract_imports_exports, is_asset_ext, path_tokens, scan_content_hash,
};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(crate) fn scan_files(root: &Path) -> Result<(BTreeMap<String, FileInfo>, ScanStats)> {
    let mut stats = ScanStatsBuilder::default();
    let rels = list_candidate_files_with_stats(root, &mut stats);
    let (files, scan_stats) = scan_candidate_rels(root, rels);
    stats.merge(scan_stats);
    stats.files_scanned = files.len();
    Ok((files, stats.finish()))
}

fn scan_candidate_rels(
    root: &Path,
    rels: Vec<String>,
) -> (BTreeMap<String, FileInfo>, ScanStatsBuilder) {
    let worker_count = scan_worker_count(rels.len());
    if worker_count <= 1 {
        return scan_candidate_rels_sequential(root, rels);
    }

    let chunk_size = rels.len().div_ceil(worker_count).max(1);
    let mut results = Vec::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in rels.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            handles.push(scope.spawn(move || scan_candidate_rels_sequential(root, chunk)));
        }
        for handle in handles {
            results.push(handle.join().expect("scan worker should not panic"));
        }
    });

    let mut files = BTreeMap::new();
    let mut stats = ScanStatsBuilder::default();
    for (worker_files, worker_stats) in results {
        files.extend(worker_files);
        stats.merge(worker_stats);
    }
    (files, stats)
}

fn scan_candidate_rels_sequential(
    root: &Path,
    rels: Vec<String>,
) -> (BTreeMap<String, FileInfo>, ScanStatsBuilder) {
    let mut stats = ScanStatsBuilder::default();
    let mut files = BTreeMap::new();
    for rel in rels {
        if rel.is_empty() {
            continue;
        }
        if let Some(reason) = ignore_reason(&rel) {
            stats.record_ignored(&reason, &rel);
            continue;
        }
        if let Some(info) = scan_file(root, &rel, &mut stats) {
            files.insert(rel, info);
        }
    }
    (files, stats)
}

fn scan_worker_count(file_count: usize) -> usize {
    if file_count < 256 {
        return 1;
    }
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(8)
        .min(file_count)
}

pub(crate) fn scan_selected_files(
    root: &Path,
    rels: &BTreeSet<String>,
) -> (BTreeMap<String, FileInfo>, ScanStats) {
    let mut stats = ScanStatsBuilder::default();
    let mut files = BTreeMap::new();
    for rel in rels {
        if let Some(info) = scan_file(root, rel, &mut stats) {
            files.insert(rel.clone(), info);
        }
    }
    stats.files_scanned = files.len();
    (files, stats.finish())
}

fn scan_file(root: &Path, rel: &str, stats: &mut ScanStatsBuilder) -> Option<FileInfo> {
    stats.files_visited += 1;
    let path = root.join(rel);
    let Ok(meta) = fs::symlink_metadata(&path) else {
        return None;
    };
    if meta.file_type().is_symlink() || !meta.is_file() {
        stats.record_skipped("not_regular_file", rel);
        return None;
    }
    if let Some(reason) = scan_file_rejection(&path, meta.len()) {
        stats.record_skipped(reason, rel);
        return None;
    }
    stats.bytes_scanned += meta.len();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let language = language_for(&path);
    let mut info = FileInfo {
        rel: rel.to_string(),
        ext,
        size: meta.len(),
        content_hash: None,
        line_count: 0,
        language,
        roles: BTreeSet::new(),
        imports: BTreeSet::new(),
        import_bindings: BTreeMap::new(),
        resolved_imports: BTreeSet::new(),
        unresolved_imports: BTreeSet::new(),
        resolved_import_bindings: BTreeMap::new(),
        exports: BTreeSet::new(),
        symbols: Vec::new(),
        tokens: path_tokens(rel),
        references: BTreeSet::new(),
        jsx_tags: BTreeSet::new(),
        local_bindings: BTreeSet::new(),
        surface_tokens: BTreeSet::new(),
        surface_phrases: BTreeSet::new(),
        visited_route_paths: BTreeSet::new(),
    };
    classify_roles(root, &mut info);
    if is_asset_ext(&info.ext) && info.ext != "svg" {
        if let Ok(bytes) = fs::read(&path) {
            info.content_hash = Some(scan_content_hash(&bytes));
        }
    } else {
        extract_imports_exports(root, &mut info);
    }
    if info.has_role("generated") {
        stats.record_generated("generated_path_or_header", rel);
    }
    Some(info)
}
