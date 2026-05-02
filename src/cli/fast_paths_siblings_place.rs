fn try_cached_siblings_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Siblings(args) = command else {
        return Ok(None);
    };
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let scope = root_relative_arg(&root, &args.scope)?;
    let git_state = repo::git_changes(&root, false, None);
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    let Some(report) = crate::cache::read_siblings_report(crate::cache::SiblingsLensKey {
        cache_dir: &cache_dir,
        version: repo::VERSION,
        root: &root,
        scope: &scope,
        include_hidden: args.include_hidden,
        limit: args.limit,
    }) else {
        return Ok(None);
    };
    output(args.format, &report, || render::siblings(&report))?;
    Ok(Some(()))
}

fn try_cached_place_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Place(args) = command else {
        return Ok(None);
    };
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let scope = root_relative_arg(&root, &args.scope)?;
    let git_state = repo::git_changes(&root, false, None);
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    let Some(report) = crate::cache::read_place_report(crate::cache::PlaceLensKey {
        cache_dir: &cache_dir,
        version: repo::VERSION,
        root: &root,
        scope: &scope,
        kind: &args.kind,
        include_hidden: args.include_hidden,
        limit: args.limit,
    }) else {
        return Ok(None);
    };
    output(args.format, &report, || render::place(&report))?;
    Ok(Some(()))
}

fn maybe_write_siblings_lens_cache(
    project: &crate::model::Project,
    scope: &str,
    args: &SiblingsArgs,
    report: &crate::model::SiblingsReport,
) {
    let _ = crate::cache::write_siblings_report(
        crate::cache::SiblingsLensKey {
            cache_dir: &project.cache_dir,
            version: repo::VERSION,
            root: &project.root,
            scope,
            include_hidden: args.include_hidden,
            limit: args.limit,
        },
        report,
    );
}

fn maybe_write_place_lens_cache(
    project: &crate::model::Project,
    scope: &str,
    args: &PlaceArgs,
    report: &crate::model::PlaceReport,
) {
    let _ = crate::cache::write_place_report(
        crate::cache::PlaceLensKey {
            cache_dir: &project.cache_dir,
            version: repo::VERSION,
            root: &project.root,
            scope,
            kind: &args.kind,
            include_hidden: args.include_hidden,
            limit: args.limit,
        },
        report,
    );
}
