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
    let (anchors, config_path, config_errors) = load_ctx_configs(&root);
    let nearest_agents = nearest_agents(&cwd, &root);
    let root_ms = root_started.elapsed().as_millis();
    let vcs = if is_git_repo(&root) {
        Some("git".to_string())
    } else {
        None
    };
    let cache_dir = cache::project_cache_dir(&root, remote.as_deref(), VERSION);

    if cache::cache_enabled() {
        let cache_artifact_started = Instant::now();
        if let Some(delta) = incremental_file_delta(&root, &cache_dir, VERSION, config_path.as_deref())
            && let Some(mut cached) =
                cache::read_cached_project(&cache_dir, VERSION, &delta.cached_fingerprint)
        {
            let mut files = BTreeMap::new();
            let mut cache_complete = true;
            for rel in &delta.unchanged {
                if let Some(file) = cached.files.remove(rel) {
                    files.insert(rel.clone(), file);
                } else {
                    cache_complete = false;
                    break;
                }
            }
            if cache_complete {
                let scan_started = Instant::now();
                let (rescanned, rescanned_stats) =
                    scan_selected_files(&root, &delta.changed_or_added);
                let scan_ms = scan_started.elapsed().as_millis();
                files.extend(rescanned);
                if files.len() == delta.current_file_count() {
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
                        cache_strategy: if delta.is_exact_hit() {
                            "warm_load".to_string()
                        } else {
                            "partial_rescan".to_string()
                        },
                        files_reused: delta.unchanged.len(),
                    });
                    let fingerprint = cache::fingerprint(&project, None);
                    let cache_artifacts = cache::artifact_statuses(&project, &fingerprint);
                    project.cache_state = cache::cache_state(&cache_artifacts);
                    project.cache_artifacts = cache_artifacts;
                    let cache_artifact_ms = cache_artifact_started.elapsed().as_millis();
                    let cache_write_started = Instant::now();
                    if cache_write == CacheWriteMode::Enabled && !delta.is_exact_hit() {
                        cache::write_status(&project, VERSION)?;
                    }
                    let cache_write_ms = cache_write_started.elapsed().as_millis();
                    project.timings.root_ms = root_ms;
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
    let packages = detect_packages(&root, &files);
    let ts_path_aliases = detect_ts_path_aliases(&root, &files);
    resolve_imports(&root, &mut files, &packages, &ts_path_aliases);
    enrich_accessible_surfaces_from_component_contracts(&root, &mut files);
    let reverse_imports = build_reverse_imports(&files);
    let package_edges = detect_package_edges(&root, &files, &packages);
    let scripts = detect_scripts(&root);
    let package_manager = detect_package_manager(&root);
    let languages = detect_languages(&files);
    let domains = discover_domains(&root, &files, &anchors, config_path.as_deref());
    let facts_ms = facts_started.elapsed().as_millis();
    let mut project = Project {
        root,
        cwd,
        vcs,
        cache_dir,
        config_path,
        config_errors,
        nearest_agents,
        files,
        reverse_imports,
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
        cache::write_status(&project, VERSION)?;
    }
    let cache_write_ms = cache_write_started.elapsed().as_millis();
    project.timings = ProjectTimings {
        root_ms,
        scan_ms,
        facts_ms,
        cache_artifact_ms,
        cache_write_ms,
        total_ms: total_started.elapsed().as_millis(),
    };
    Ok(project)
}

fn incremental_file_delta(
    root: &Path,
    cache_dir: &Path,
    version: &str,
    config_path: Option<&str>,
) -> Option<cache::fingerprints::CacheFileDelta> {
    git_status_cache_delta(root, cache_dir, version).or_else(|| {
        let current_files = cache_candidate_files(root);
        cache::file_delta(root, cache_dir, version, &current_files, config_path)
    })
}

struct ProjectBuildInput {
    root: PathBuf,
    cwd: PathBuf,
    vcs: Option<String>,
    cache_dir: PathBuf,
    config_path: Option<String>,
    config_errors: Vec<ConfigLoadError>,
    nearest_agents: Option<String>,
    anchors: CtxConfig,
    files: BTreeMap<String, FileInfo>,
    scan_stats: ScanStats,
    cache_strategy: String,
    files_reused: usize,
}

fn build_project_from_files(input: ProjectBuildInput) -> Project {
    let facts_started = Instant::now();
    let mut files = input.files;
    let packages = detect_packages(&input.root, &files);
    let ts_path_aliases = detect_ts_path_aliases(&input.root, &files);
    resolve_imports(&input.root, &mut files, &packages, &ts_path_aliases);
    enrich_accessible_surfaces_from_component_contracts(&input.root, &mut files);
    let reverse_imports = build_reverse_imports(&files);
    let package_edges = detect_package_edges(&input.root, &files, &packages);
    let scripts = detect_scripts(&input.root);
    let package_manager = detect_package_manager(&input.root);
    let languages = detect_languages(&files);
    let domains = discover_domains(
        &input.root,
        &files,
        &input.anchors,
        input.config_path.as_deref(),
    );
    let facts_ms = facts_started.elapsed().as_millis();
    Project {
        root: input.root,
        cwd: input.cwd,
        vcs: input.vcs,
        cache_dir: input.cache_dir,
        config_path: input.config_path,
        config_errors: input.config_errors,
        nearest_agents: input.nearest_agents,
        files,
        reverse_imports,
        packages,
        package_edges,
        domains,
        package_manager,
        scripts,
        languages,
        anchors: input.anchors,
        cache_state: String::new(),
        cache_artifacts: Vec::new(),
        cache_strategy: input.cache_strategy,
        files_reused: input.files_reused,
        scan_stats: input.scan_stats,
        timings: ProjectTimings {
            facts_ms,
            ..ProjectTimings::default()
        },
    }
}

fn cached_scan_stats(
    cached: &ScanStats,
    rescanned: ScanStats,
    files: &BTreeMap<String, FileInfo>,
) -> ScanStats {
    ScanStats {
        files_visited: rescanned.files_visited,
        files_scanned: rescanned.files_scanned,
        files_skipped: rescanned.files_skipped,
        bytes_scanned: rescanned.bytes_scanned,
        ignored: cached.ignored.clone(),
        generated: generated_scan_groups(files),
    }
}

fn generated_scan_groups(files: &BTreeMap<String, FileInfo>) -> Vec<ScanGroup> {
    let generated = files
        .values()
        .filter(|file| file.has_role("generated"))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    if generated.is_empty() {
        Vec::new()
    } else {
        vec![ScanGroup {
            reason: "generated_path_or_header".to_string(),
            count: generated.len(),
            examples: generated.into_iter().take(5).collect(),
        }]
    }
}
