// Responsibility: repo-project
use crate::cache;
use crate::model::{FileInfo, Project, ProjectTimings, ScanStats};
use crate::repo::{
    ProjectBuildInput, VERSION, apply_codemap_config_roles, build_project_from_files,
    cache_candidate_files, cached_index_cache_delta, cached_scan_stats, detect_languages,
    detect_package_edges, detect_package_manager, detect_packages, detect_scripts,
    detect_ts_path_aliases, discover_domains, enrich_accessible_surfaces_from_component_contracts,
    git_head_cache_delta, git_remote, git_status_cache_change_sets, git_status_cache_delta,
    is_git_repo, load_codemap_configs, nearest_agents, resolve_imports, resolve_root, scan_files,
    scan_selected_files,
};
use anyhow::Context;
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheWriteMode {
    Enabled,
    ReadOnly,
}

#[derive(Debug, Clone)]
pub enum RootSelection {
    Auto,
    Exact(PathBuf),
    Discover(PathBuf),
}

pub fn load_project_with_cache(
    root_selection: RootSelection,
    cache_write: CacheWriteMode,
) -> Result<Project> {
    let total_started = Instant::now();
    let root_started = Instant::now();
    let root_hint = match &root_selection {
        RootSelection::Auto => None,
        RootSelection::Exact(path) | RootSelection::Discover(path) => Some(path),
    };
    let cwd = if let Some(path) = root_hint {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        env::current_dir().context("failed to read current directory")?
    };
    let root = resolve_root(&root_selection, &cwd)?;
    let remote = git_remote(&root);
    let (anchors, config_path, config_errors) = load_codemap_configs(&root);
    let nearest_agents = nearest_agents(&cwd, &root);
    let root_ms = root_started.elapsed().as_millis();
    let vcs = if is_git_repo(&root) {
        Some("git".to_string())
    } else {
        None
    };
    let cache_dir = cache::project_cache_dir(&root, remote.as_deref(), VERSION);

    'cached_load: {
        if cache::cache_enabled() {
            let cache_probe_started = Instant::now();
            if let Some(delta) =
                incremental_file_delta(&root, &cache_dir, VERSION, config_path.as_deref())
                && let Some(mut cached) = cache::read_cached_project(
                    &cache_dir,
                    VERSION,
                    &root,
                    &delta.cached_fingerprint,
                )
            {
                let cache_probe_ms = cache_probe_started.elapsed().as_millis();
                let old_cached_files = cached.files.clone();
                let mut files = BTreeMap::new();
                let mut scan_candidates = delta.changed_or_added.clone();
                for rel in &delta.unchanged {
                    if let Some(file) = cached.files.remove(rel) {
                        if cached_file_facts_match_delta(&file, &delta, rel) {
                            files.insert(rel.clone(), file);
                        } else {
                            scan_candidates.insert(rel.clone());
                        }
                    } else {
                        scan_candidates.insert(rel.clone());
                    }
                }
                let file_reuse_count = files.len();
                let scan_started = Instant::now();
                let (rescanned, rescanned_stats) = scan_selected_files(&root, &scan_candidates);
                let scan_ms = scan_started.elapsed().as_millis();
                if scan_inventory_boundaries(&cached.scan_stats)
                    != scan_inventory_boundaries(&rescanned_stats)
                {
                    // A global completeness transition can add or remove paths
                    // which never had a per-file fingerprint. Rebuild from the
                    // actual scanner owner instead of composing unlike worlds.
                    break 'cached_load;
                }
                let discovered_boundary_count = rescanned
                    .keys()
                    .filter(|rel| {
                        !delta.unchanged.contains(*rel) && !delta.changed_or_added.contains(*rel)
                    })
                    .count();
                let expected_file_count = file_reuse_count + rescanned.len();
                files.extend(rescanned);
                // The git index also contains tracked files deliberately rejected by
                // the scanner (for example LICENSE or fixture manifests). They are
                // valid delta candidates but not indexed facts, so the reconstructed
                // inventory is complete when every reused or rescanned fact survived,
                // not when it equals the broader git candidate count.
                if files.len() == expected_file_count {
                    let files_rebuilt = rescanned_stats.files_scanned;
                    let scan_stats = cached_scan_stats(&cached.scan_stats, rescanned_stats, &files);
                    let mut project = build_project_from_files(ProjectBuildInput {
                        root,
                        cwd,
                        vcs,
                        cache_dir,
                        config_path,
                        config_errors,
                        nearest_agents,
                        anchors,
                        files,
                        scan_stats,
                        cache_strategy: if scan_candidates.is_empty()
                            && delta.is_exact_hit()
                            && discovered_boundary_count == 0
                        {
                            "warm_load".to_string()
                        } else {
                            "partial_rescan".to_string()
                        },
                        files_reused: file_reuse_count,
                        files_rebuilt,
                        old_cached_files: Some((
                            delta.cached_fingerprint.clone(),
                            old_cached_files,
                        )),
                    });
                    let cache_artifact_started = Instant::now();
                    let fingerprint = cache::fingerprint(&project, None);
                    let cache_artifacts = cache::artifact_statuses(&project, &fingerprint);
                    project.cache_state = cache::cache_state(&cache_artifacts);
                    project.cache_artifacts = cache_artifacts;
                    let cache_artifact_ms = cache_artifact_started.elapsed().as_millis();
                    let cache_write_started = Instant::now();
                    let cache_needs_refresh = project
                        .cache_artifacts
                        .iter()
                        .any(|artifact| artifact.fingerprint_match != Some(true));
                    let cached_head_mismatch =
                        cache::cached_git_head_matches(&project.root, &project.cache_dir, VERSION)
                            == Some(false);
                    if cache_write == CacheWriteMode::Enabled
                        && (!scan_candidates.is_empty()
                            || !delta.is_exact_hit()
                            || discovered_boundary_count > 0
                            || cache_needs_refresh
                            || cached_head_mismatch)
                    {
                        let git_status_change_sets = git_status_cache_change_sets(&project.root);
                        cache::write_status_with_change_sets(
                            &project,
                            VERSION,
                            git_status_change_sets
                                .as_ref()
                                .map(|(changed_or_added, removed)| (changed_or_added, removed)),
                        )?;
                        let fingerprint = cache::fingerprint(&project, None);
                        let cache_artifacts = cache::artifact_statuses(&project, &fingerprint);
                        project.cache_state = cache::cache_state(&cache_artifacts);
                        project.cache_artifacts = cache_artifacts;
                    }
                    let cache_write_ms = cache_write_started.elapsed().as_millis();
                    project.timings.root_ms = root_ms;
                    project.timings.cache_probe_ms = cache_probe_ms;
                    project.timings.scan_ms = scan_ms;
                    project.timings.cache_artifact_ms = cache_artifact_ms;
                    project.timings.cache_write_ms = cache_write_ms;
                    project.timings.total_ms = total_started.elapsed().as_millis();
                    return Ok(project);
                }
            }
        }
    }

    let scan_started = Instant::now();
    let (mut files, scan_stats) = scan_files(&root)?;
    let scan_ms = scan_started.elapsed().as_millis();

    let facts_started = Instant::now();
    apply_codemap_config_roles(&mut files, &anchors);
    let packages = detect_packages(&root, &files);
    let ts_path_aliases = detect_ts_path_aliases(&root, &files);
    resolve_imports(&root, &mut files, &packages, &ts_path_aliases);
    enrich_accessible_surfaces_from_component_contracts(&root, &mut files);
    let package_edges = detect_package_edges(&root, &files, &packages);
    let scripts = detect_scripts(&root, &files);
    let package_manager = detect_package_manager(&files);
    let languages = detect_languages(&files);
    let domains = discover_domains(&root, &files, &anchors, config_path.as_deref());
    let facts_ms = facts_started.elapsed().as_millis();
    let reverse_started = Instant::now();
    let reverse_update = cache::full_reverse_imports(&files);
    let reverse_index_ms = reverse_started.elapsed().as_millis();
    let rebuilt_file_facts = files.len();
    let mut project = Project {
        root,
        cwd,
        vcs,
        cache_dir,
        config_path,
        config_errors,
        nearest_agents,
        files,
        reverse_imports: reverse_update.index,
        packages,
        package_edges,
        domains,
        package_manager,
        scripts,
        languages,
        anchors,
        cache_state: String::new(),
        cache_artifacts: Vec::new(),
        cache_strategy: if cache::cache_enabled() {
            "full_scan".to_string()
        } else {
            "disabled".to_string()
        },
        cache_work: crate::model::CacheWork {
            per_file_facts_reused: 0,
            per_file_facts_rebuilt: rebuilt_file_facts,
            reverse_import_strategy: reverse_update.strategy.to_string(),
            reverse_import_targets_rebuilt: reverse_update.affected_targets,
        },
        files_reused: 0,
        scan_stats,
        timings: ProjectTimings::default(),
    };
    let cache_artifact_started = Instant::now();
    let fingerprint = cache::fingerprint(&project, None);
    let cache_artifacts = cache::artifact_statuses(&project, &fingerprint);
    project.cache_state = cache::cache_state(&cache_artifacts);
    project.cache_artifacts = cache_artifacts;
    let cache_artifact_ms = cache_artifact_started.elapsed().as_millis();

    let cache_write_started = Instant::now();
    if cache_write == CacheWriteMode::Enabled {
        let git_status_change_sets = git_status_cache_change_sets(&project.root);
        cache::write_status_with_change_sets(
            &project,
            VERSION,
            git_status_change_sets
                .as_ref()
                .map(|(changed_or_added, removed)| (changed_or_added, removed)),
        )?;
    }
    let cache_write_ms = cache_write_started.elapsed().as_millis();
    project.timings = ProjectTimings {
        root_ms,
        cache_probe_ms: 0,
        scan_ms,
        facts_ms,
        reverse_index_ms,
        cache_artifact_ms,
        cache_write_ms,
        total_ms: total_started.elapsed().as_millis(),
    };
    Ok(project)
}

fn scan_inventory_boundaries(stats: &ScanStats) -> BTreeSet<crate::model::ScanInventoryBoundary> {
    stats.inventory_boundaries.iter().copied().collect()
}

fn incremental_file_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    config_path: Option<&str>,
) -> Option<cache::fingerprints::CacheFileDelta> {
    git_status_cache_delta(root, cache_dir, version)
        .or_else(|| git_head_cache_delta(root, cache_dir, version))
        .or_else(|| cached_index_cache_delta(root, cache_dir, version))
        .or_else(|| {
            let current_files = cache_candidate_files(root);
            cache::file_delta(root, cache_dir, version, &current_files, config_path)
        })
}

fn cached_file_facts_match_delta(
    file: &FileInfo,
    delta: &cache::fingerprints::CacheFileDelta,
    rel: &str,
) -> bool {
    delta
        .cached_content_hashes
        .get(rel)
        .is_some_and(|expected| file.content_hash == *expected)
}
