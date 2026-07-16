// Responsibility: repo-scan
mod candidates;
mod file_filters;
mod git_index;
mod stats;

pub(crate) use candidates::*;
pub(crate) use file_filters::*;
pub(crate) use git_index::*;
pub(crate) use stats::*;

use crate::model::{FileInfo, IndexedBoundary, ScanStats};
use crate::repo::{
    classify_build_ci_role, classify_roles, extract_imports_exports, is_asset_ext, path_tokens,
    scan_content_hash,
};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(crate) fn scan_files(root: &Path) -> Result<(BTreeMap<String, FileInfo>, ScanStats)> {
    let mut stats = ScanStatsBuilder::default();
    let candidates = scan_candidate_inventory(root, &mut stats);
    let (files, scan_stats) = scan_candidate_rels(root, &candidates);
    stats.merge(scan_stats);
    stats.files_scanned = files.len();
    Ok((files, stats.finish()))
}

fn scan_candidate_rels(
    root: &Path,
    candidates: &ScanCandidateInventory,
) -> (BTreeMap<String, FileInfo>, ScanStatsBuilder) {
    let rels = &candidates.rels;
    let worker_count = scan_worker_count(rels.len());
    if worker_count <= 1 {
        return scan_candidate_rels_sequential(root, rels, candidates);
    }

    let chunk_size = rels.len().div_ceil(worker_count).max(1);
    let mut results = Vec::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in rels.chunks(chunk_size) {
            handles
                .push(scope.spawn(move || scan_candidate_rels_sequential(root, chunk, candidates)));
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
    rels: &[String],
    candidates: &ScanCandidateInventory,
) -> (BTreeMap<String, FileInfo>, ScanStatsBuilder) {
    let mut stats = ScanStatsBuilder::default();
    let mut files = BTreeMap::new();
    for rel in rels {
        if rel.is_empty() || rel == "." {
            continue;
        }
        let index_kind = candidates
            .git_index
            .as_ref()
            .and_then(|index| index.kind(rel));
        if let Some(reason) = ignore_reason(rel)
            && index_kind.is_none()
            && candidates.boundary(root, rel).is_none()
        {
            stats.record_ignored(&reason, rel);
            continue;
        }
        if let Some(info) = scan_file(
            root,
            rel,
            index_kind,
            candidates.boundary(root, rel),
            &mut stats,
        ) {
            files.insert(rel.clone(), info);
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
    let candidates = scan_candidate_inventory(root, &mut stats);
    let mut selected = rels.clone();
    selected.extend(candidates.path_boundaries().cloned());
    for rel in &selected {
        let index_kind = candidates
            .git_index
            .as_ref()
            .and_then(|index| index.kind(rel));
        if let Some(info) = scan_file(
            root,
            rel,
            index_kind,
            candidates.boundary(root, rel),
            &mut stats,
        ) {
            files.insert(rel.clone(), info);
        }
    }
    stats.files_scanned = files
        .values()
        .filter(|file| file.indexed_boundary.is_none())
        .count();
    (files, stats.finish())
}

fn scan_file(
    root: &Path,
    rel: &str,
    index_kind: Option<GitIndexKind>,
    boundary_override: Option<IndexedBoundary>,
    stats: &mut ScanStatsBuilder,
) -> Option<FileInfo> {
    let indexed_boundary =
        boundary_override.or_else(|| indexed_boundary_for_path(root, rel, index_kind));
    scan_file_with_boundary(root, rel, indexed_boundary, stats)
}

pub(crate) fn scan_regular_file(
    root: &Path,
    rel: &str,
    stats: &mut ScanStatsBuilder,
) -> Option<FileInfo> {
    scan_file_with_boundary(root, rel, None, stats)
}

fn scan_file_with_boundary(
    root: &Path,
    rel: &str,
    indexed_boundary: Option<IndexedBoundary>,
    stats: &mut ScanStatsBuilder,
) -> Option<FileInfo> {
    stats.files_visited += 1;
    let path = root.join(rel);
    let meta = fs::symlink_metadata(&path).ok();
    if meta.is_none() && indexed_boundary.is_none() {
        return None;
    }
    let source_symlink = meta
        .as_ref()
        .is_some_and(|meta| meta.file_type().is_symlink());
    let parser_placeholder = meta.as_ref().is_some_and(|meta| {
        !meta.file_type().is_symlink()
            && meta.is_file()
            && source_parser_requires_placeholder(&path)
    });
    if matches!(
        indexed_boundary,
        Some(IndexedBoundary::ExternalTree | IndexedBoundary::ExternalGitlink)
    ) {
        stats.record_skipped("external_tree_boundary", rel);
    } else if indexed_boundary == Some(IndexedBoundary::IgnoredTrackedFile) {
        stats.record_skipped("tracked_file_ignored_by_default", rel);
    } else if indexed_boundary == Some(IndexedBoundary::TraversalError) {
        stats.record_skipped("traversal_error", rel);
    } else if indexed_boundary == Some(IndexedBoundary::UnavailableTrackedFile) {
        stats.record_skipped("tracked_file_unavailable", rel);
    } else if source_symlink {
        stats.record_skipped("symlink_not_followed", rel);
    } else if meta.as_ref().is_some_and(|meta| !meta.is_file()) {
        stats.record_skipped("not_regular_file", rel);
        return None;
    }
    if parser_placeholder {
        stats.record_skipped("unsupported_source_parser", rel);
    }
    let size = if indexed_boundary.is_some() {
        0
    } else {
        meta.as_ref().map_or(0, fs::Metadata::len)
    };
    let rejection = (indexed_boundary.is_none() && !source_symlink && !parser_placeholder)
        .then(|| scan_file_rejection(&path, rel, size))
        .flatten();
    if let Some(reason) = rejection {
        stats.record_skipped(reason, rel);
        if !scan_rejection_keeps_placeholder(&path, rel, reason) {
            return None;
        }
    }
    if indexed_boundary.is_none() && !source_symlink && !parser_placeholder {
        stats.bytes_scanned += size;
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let language = language_for(&path);
    let mut info = FileInfo {
        rel: rel.to_string(),
        ext,
        size,
        indexed_boundary,
        content_hash: None,
        scanned_source_text: None,
        line_count: 0,
        has_dynamic_import: false,
        has_dynamic_require: false,
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
    classify_build_ci_role(&mut info);
    if indexed_boundary.is_some() || source_symlink || parser_placeholder || rejection.is_some() {
        // Placeholder files still carry path-derived roles, but body probes in
        // the role owners are gated by `content_hash` and therefore cannot
        // follow a symlink or re-read an oversized/unparsed body.
        classify_roles(root, &mut info);
        return Some(info);
    }
    if is_asset_ext(&info.ext) && info.ext != "svg" {
        if let Ok(bytes) = fs::read(&path) {
            info.content_hash = Some(scan_content_hash(&bytes));
        }
    } else {
        extract_imports_exports(root, &mut info);
    }
    // Body-derived roles are classified only after the scanner has established
    // a readable, indexed body and recorded its content hash. If reading raced
    // or failed, the role owners fall back to path-only evidence.
    classify_roles(root, &mut info);
    if info.has_role("generated") {
        stats.record_generated("generated_path_or_header", rel);
    }
    Some(info)
}
