// Responsibility: cli-navigation-fast-paths
use crate::cli::{
    CommandKind, ConeArgs, accept_depth_compat, cone_section_name, lens_cache_matches_current,
    ls_section_name, output, output_format_with_json_alias, output_with_prelude, root_relative_arg,
    set_cached_map_snapshot,
};
use crate::{render, repo};
use anyhow::Result;
use std::env;

pub(crate) fn try_cached_ls_fast_path(
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
    let format = output_format_with_json_alias(args.format, args.json);
    let exact_file = root.join(&path).is_file();
    let (include_hidden, limit, complete_file_projection, complete_directory_projection) =
        args.effective_projection(&path, format, exact_file);
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
        include_hidden,
        limit,
        complete_file_projection,
        complete_directory_projection,
    }) else {
        return Ok(None);
    };
    let prelude = repo::map_prelude(&root);
    output_with_prelude(format, &report, &prelude, || {
        render::ls(&report, ls_section_name(args.section))
    })?;
    Ok(Some(()))
}

pub(crate) fn try_cached_cone_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Cone(args) = command else {
        return Ok(None);
    };
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let path = root_relative_arg(&root, &args.path)?;
    let format = output_format_with_json_alias(args.format, args.json);
    let include_hidden = args.include_hidden || format == crate::cli::OutputFormat::Json;
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
        include_hidden,
        limit: args.limit,
    }) else {
        return Ok(None);
    };
    let prelude = repo::map_prelude(&root);
    output_with_prelude(format, &report, &prelude, || {
        render::cone(&report, cone_section_name(args.section))
    })?;
    Ok(Some(()))
}

pub(crate) fn try_cached_where_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::Where(args) = command else {
        return Ok(None);
    };
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let format = output_format_with_json_alias(args.format, args.json);
    let include_hidden = args.include_hidden || format == crate::cli::OutputFormat::Json;
    let limit = if include_hidden {
        usize::MAX / 2
    } else {
        args.limit
    };
    let kind_filter = args
        .kind
        .as_deref()
        .map(|kind| kind.strip_prefix("symbol:").unwrap_or(kind));
    let git_state = repo::git_changes(&root, false, None);
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    if !lens_cache_matches_current(&root, &cache_dir, &git_state) {
        return Ok(None);
    }
    set_cached_map_snapshot(&root, &cache_dir);
    let Some(report) = crate::cache::read_where_report(crate::cache::WhereLensKey {
        cache_dir: &cache_dir,
        version: repo::VERSION,
        root: &root,
        query: args.query.trim(),
        kind_filter,
        include_hidden,
        limit,
    }) else {
        return Ok(None);
    };
    output(format, &report, || render::where_locator(&report))?;
    Ok(Some(()))
}

pub(crate) fn maybe_write_ls_lens_cache(
    project: &crate::model::Project,
    path: &str,
    include_hidden: bool,
    limit: usize,
    complete_file_projection: bool,
    complete_directory_projection: bool,
    report: &crate::model::LsReport,
) {
    let _ = crate::cache::write_ls_report(
        crate::cache::LsLensKey {
            cache_dir: &project.cache_dir,
            version: repo::VERSION,
            root: &project.root,
            path,
            include_hidden,
            limit,
            complete_file_projection,
            complete_directory_projection,
        },
        report,
    );
}

pub(crate) fn maybe_write_cone_lens_cache(
    project: &crate::model::Project,
    path: &str,
    args: &ConeArgs,
    include_hidden: bool,
    report: &crate::model::ConeReport,
) {
    let _ = crate::cache::write_cone_report(
        crate::cache::ConeLensKey {
            cache_dir: &project.cache_dir,
            version: repo::VERSION,
            root: &project.root,
            path,
            depth: args.depth,
            include_hidden,
            limit: args.limit,
        },
        report,
    );
}

pub(crate) fn maybe_write_where_lens_cache(
    project: &crate::model::Project,
    include_hidden: bool,
    limit: usize,
    report: &crate::model::WhereReport,
) {
    let _ = crate::cache::write_where_report(
        crate::cache::WhereLensKey {
            cache_dir: &project.cache_dir,
            version: repo::VERSION,
            root: &project.root,
            query: &report.query,
            kind_filter: report.kind_filter.as_deref(),
            include_hidden,
            limit,
        },
        report,
    );
}
