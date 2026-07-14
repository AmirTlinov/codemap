// A snapshot-token `--since` must skip the git-status fast paths: they would treat
// the token as an empty git diff and report "clean". Fall through to the full path
// so snapshot_delta runs.
fn since_is_snapshot_token(since: Option<&str>) -> bool {
    since.is_some_and(crate::cache::looks_like_snapshot_token)
}

fn try_cached_ls_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Ls(args) = command else {
        return Ok(None);
    };
    accept_depth_compat(args.depth, "ls")?;
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let path = root_relative_arg(&root, &args.path)?;
    let git_state = repo::git_changes(&root, false, None);
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    set_cached_map_snapshot(&root, &cache_dir);
    let Some(report) = crate::cache::read_ls_report(crate::cache::LsLensKey {
        cache_dir: &cache_dir,
        version: repo::VERSION,
        root: &root,
        path: &path,
        include_hidden: args.include_hidden,
        limit: args.limit,
    }) else {
        return Ok(None);
    };
    let prelude = repo::map_prelude(&root);
    output_with_prelude(output_format_with_json_alias(args.format, args.json), &report, &prelude, || {
        render::ls(&report, ls_section_name(args.section))
    })?;
    Ok(Some(()))
}

fn try_cached_cone_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Cone(args) = command else {
        return Ok(None);
    };
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let path = root_relative_arg(&root, &args.path)?;
    let git_state = repo::git_changes(&root, false, None);
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    set_cached_map_snapshot(&root, &cache_dir);
    let Some(report) = crate::cache::read_cone_report(crate::cache::ConeLensKey {
        cache_dir: &cache_dir,
        version: repo::VERSION,
        root: &root,
        path: &path,
        depth: args.depth,
        include_hidden: args.include_hidden,
        limit: args.limit,
    }) else {
        return Ok(None);
    };
    let prelude = repo::map_prelude(&root);
    output_with_prelude(output_format_with_json_alias(args.format, args.json), &report, &prelude, || {
        render::cone(&report, cone_section_name(args.section))
    })?;
    Ok(Some(()))
}

fn try_clean_changed_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Changed(args) = command else {
        return Ok(None);
    };
    accept_depth_compat(args.depth, "changed")?;
    ensure_single_diff_selector(
        args.changed,
        args.staged,
        args.since.as_deref(),
        args.files.as_deref(),
        &args.positional_files,
    )?;
    if since_is_snapshot_token(args.since.as_deref()) {
        return Ok(None);
    }
    if changed_has_explicit_files(args) {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let (selector, git_state) = changed_selector_state(args, &root);
    if !git_state.is_empty() {
        return Ok(None);
    }
    let limit = changed_limit(args);
    let report = map::clean_changed_report(selector, limit);
    set_inventory_map_snapshot(&root);
    let prelude = repo::map_prelude(&root);
    output_with_prelude(output_format_with_json_alias(args.format, args.json), &report, &prelude, || {
        render::changed(&report, changed_section_name(args.section))
    })?;
    Ok(Some(()))
}

fn try_cached_changed_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Changed(args) = command else {
        return Ok(None);
    };
    accept_depth_compat(args.depth, "changed")?;
    ensure_single_diff_selector(
        args.changed,
        args.staged,
        args.since.as_deref(),
        args.files.as_deref(),
        &args.positional_files,
    )?;
    if since_is_snapshot_token(args.since.as_deref()) {
        return Ok(None);
    }
    if changed_has_explicit_files(args) {
        return Ok(None);
    }
    if changed_requires_section_report(args) {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let (selector, git_state) = changed_selector_state(args, &root);
    if git_state.is_empty() {
        return Ok(None);
    }
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    set_cached_map_snapshot(&root, &cache_dir);
    let limit = changed_limit(args);
    let Some(report) =
        crate::cache::read_changed_report(&cache_dir, repo::VERSION, &root, &selector, limit)
    else {
        return Ok(None);
    };
    let prelude = repo::map_prelude(&root);
    output_with_prelude(output_format_with_json_alias(args.format, args.json), &report, &prelude, || {
        render::changed(&report, changed_section_name(args.section))
    })?;
    Ok(Some(()))
}

fn try_clean_proof_changed_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Proof(args) = command else {
        return Ok(None);
    };
    ensure_single_proof_selector(args)?;
    if proof_has_explicit_target_or_files(args) || args.run || args.include_hidden {
        return Ok(None);
    }
    if since_is_snapshot_token(args.since.as_deref()) {
        return Ok(None);
    }
    if args.target.as_deref() != Some("changed") && !args.staged && args.since.is_none() {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let (selector, git_state) = proof_selector_state(args, &root);
    if !git_state.is_empty() {
        return Ok(None);
    }
    let report = map::clean_proof_report(selector);
    set_inventory_map_snapshot(&root);
    let prelude = repo::map_prelude(&root);
    output_with_prelude(output_format_with_json_alias(args.format, args.json), &report, &prelude, || {
        render::proof(&report, proof_section_name(args.section))
    })?;
    Ok(Some(()))
}

fn try_cached_proof_changed_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Proof(args) = command else {
        return Ok(None);
    };
    ensure_single_proof_selector(args)?;
    if proof_has_explicit_target_or_files(args) || args.run || args.include_hidden {
        return Ok(None);
    }
    if since_is_snapshot_token(args.since.as_deref()) {
        return Ok(None);
    }
    if args.target.as_deref() != Some("changed") && !args.staged && args.since.is_none() {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let (selector, git_state) = proof_selector_state(args, &root);
    if git_state.is_empty() {
        return Ok(None);
    }
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    set_cached_map_snapshot(&root, &cache_dir);
    let Some(report) = crate::cache::read_proof_changed_report(
        &cache_dir,
        repo::VERSION,
        &root,
        &selector,
        args.depth,
        args.limit,
    ) else {
        return Ok(None);
    };
    let prelude = repo::map_prelude(&root);
    output_with_prelude(output_format_with_json_alias(args.format, args.json), &report, &prelude, || {
        render::proof(&report, proof_section_name(args.section))
    })?;
    Ok(Some(()))
}

fn try_runtime_root_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Runtime(args) = command else {
        return Ok(None);
    };
    if args.scope != "." || args.include_hidden || args.limit != 20 {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    let git_state = repo::git_changes(&root, false, None);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    set_cached_map_snapshot(&root, &cache_dir);
    let Some(report) = crate::cache::read_runtime_root_report(&cache_dir, repo::VERSION, &root)
    else {
        return Ok(None);
    };
    output(args.format, &report, || render::runtime(&report))?;
    Ok(Some(()))
}

fn maybe_write_changed_lens_cache(
    project: &crate::model::Project,
    args: &ChangedArgs,
    limit: usize,
    report: &crate::model::ChangedReport,
) {
    if changed_has_explicit_files(args) {
        return;
    }
    let selector = changed_selector(args);
    let _ = crate::cache::write_changed_report(
        &project.cache_dir,
        repo::VERSION,
        &project.root,
        &selector,
        limit,
        report,
    );
}

fn maybe_write_proof_changed_lens_cache_from_changed(
    project: &crate::model::Project,
    args: &ChangedArgs,
    report: &crate::model::ChangedReport,
) {
    if changed_has_explicit_files(args) {
        return;
    }
    let Some(proof_report) = report.proof_plan_cache.as_deref() else {
        return;
    };
    let selector = changed_selector(args);
    let proof_selector = if selector == "--changed" {
        "changed".to_string()
    } else {
        selector
    };
    let _ = crate::cache::write_proof_changed_report(
        &project.cache_dir,
        repo::VERSION,
        &project.root,
        &proof_selector,
        1,
        DEFAULT_PROOF_LIMIT,
        proof_report,
    );
}

fn maybe_write_proof_map_lens_cache_from_changed(
    project: &crate::model::Project,
    args: &ChangedArgs,
    limit: usize,
    report: &crate::model::ChangedReport,
) {
    if changed_has_explicit_files(args) || args.include_hidden {
        return;
    }
    let Some(proof_map) = report.proof_map_cache.as_deref() else {
        return;
    };
    let selector = changed_selector(args);
    let _ = crate::cache::write_proof_map_report(
        &project.cache_dir,
        repo::VERSION,
        &project.root,
        &selector,
        limit,
        false,
        proof_map,
    );
}

fn maybe_write_ls_lens_cache(
    project: &crate::model::Project,
    path: &str,
    args: &LsArgs,
    report: &crate::model::LsReport,
) {
    let _ = crate::cache::write_ls_report(
        crate::cache::LsLensKey {
            cache_dir: &project.cache_dir,
            version: repo::VERSION,
            root: &project.root,
            path,
            include_hidden: args.include_hidden,
            limit: args.limit,
        },
        report,
    );
}

fn maybe_write_cone_lens_cache(
    project: &crate::model::Project,
    path: &str,
    args: &ConeArgs,
    report: &crate::model::ConeReport,
) {
    let _ = crate::cache::write_cone_report(
        crate::cache::ConeLensKey {
            cache_dir: &project.cache_dir,
            version: repo::VERSION,
            root: &project.root,
            path,
            depth: args.depth,
            include_hidden: args.include_hidden,
            limit: args.limit,
        },
        report,
    );
}

fn lens_cache_matches_current(
    root: &Path,
    cache_dir: &Path,
    git_state: &[crate::model::GitChange],
) -> bool {
    let (changed_or_added, removed) =
        repo::git_status_cache_change_sets(root).unwrap_or_else(|| git_change_sets(git_state));
    crate::cache::file_delta_for_known_changes(
        root,
        cache_dir,
        repo::VERSION,
        &changed_or_added,
        &removed,
    )
    .is_some_and(|delta| delta.is_exact_hit())
}
