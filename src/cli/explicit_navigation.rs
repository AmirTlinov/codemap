// Responsibility: exact navigation into files outside the bounded inventory
use crate::cli::{
    CommandKind, project_relative_arg, root_relative_arg, try_cached_cone_fast_path,
    try_cached_ls_fast_path, try_cached_where_fast_path, try_cold_root_ls_fast_path,
};
use crate::{map, repo};
use anyhow::Result;
use std::collections::BTreeSet;
use std::env;

pub(crate) fn try_navigation_fast_paths(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<bool> {
    if try_cold_root_ls_fast_path(command, root_selection)?.is_some() {
        return Ok(true);
    }
    let bypass_cached_report = explicit_tracked_file(command, root_selection)?.is_some();
    if !bypass_cached_report
        && (try_cached_ls_fast_path(command, root_selection)?.is_some()
            || try_cached_cone_fast_path(command, root_selection)?.is_some()
            || try_cached_where_fast_path(command, root_selection)?.is_some())
    {
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn hydrate_explicit_navigation(
    project: &mut crate::model::Project,
    command: &CommandKind,
) -> Result<()> {
    let Some(value) = navigation_value(command) else {
        return Ok(());
    };
    let rel = project_relative_arg(project, value)?;
    let file = navigation_file(rel);
    if !repo::should_ignore_rel(&file) {
        return Ok(());
    }
    repo::hydrate_explicit_project_files(project, &BTreeSet::from([file]));
    Ok(())
}

fn explicit_tracked_file(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<String>> {
    let Some(value) = navigation_value(command) else {
        return Ok(None);
    };
    let portable_value = value.replace('\\', "/");
    let raw_file = portable_value
        .split_once('#')
        .map_or(portable_value.as_str(), |(file, _)| file);
    if !repo::should_ignore_rel(raw_file) {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let rel = root_relative_arg(&root, value)?;
    let file = navigation_file(rel);
    Ok(repo::is_explicit_tracked_file(&root, &file).then_some(file))
}

fn navigation_value(command: &CommandKind) -> Option<&str> {
    match command {
        CommandKind::Ls(args) => Some(&args.path),
        CommandKind::Cone(args) => Some(&args.path),
        _ => None,
    }
}

fn navigation_file(rel: String) -> String {
    map::split_symbol_anchor(&rel).map_or(rel, |(file, _)| file)
}
