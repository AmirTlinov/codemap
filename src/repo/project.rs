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
    let mut files = scan_files(&root)?;
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
    let vcs = if is_git_repo(&root) {
        Some("git".to_string())
    } else {
        None
    };
    let cache_dir = cache::project_cache_dir(&root, remote.as_deref(), VERSION);

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
    };
    let fingerprint = cache::fingerprint(&project, None);
    let cache_artifacts = cache::artifact_statuses(&project, &fingerprint);
    project.cache_state = cache::cache_state(&cache_artifacts);
    project.cache_artifacts = cache_artifacts;
    if cache_write == CacheWriteMode::Enabled {
        cache::write_status(&project, VERSION)?;
    }
    Ok(project)
}
