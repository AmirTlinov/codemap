// Responsibility: cli-fast-paths-changed
use crate::cli::{
    ChangedArgs, CommandKind, DEFAULT_PROOF_LIMIT, accept_depth_compat, changed_has_explicit_files,
    changed_limit, changed_requires_section_report, changed_section_name, changed_selector,
    changed_selector_state, ensure_single_diff_selector, lens_cache_matches_current,
    output_format_with_json_alias, output_with_prelude, set_cached_map_snapshot,
    set_inventory_map_snapshot, since_is_snapshot_token,
};
use crate::{map, render, repo};
use anyhow::Result;
use std::env;

pub(crate) fn try_clean_changed_fast_path(
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
    output_with_prelude(
        output_format_with_json_alias(args.format, args.json),
        &report,
        &prelude,
        || render::changed(&report, changed_section_name(args.section)),
    )?;
    Ok(Some(()))
}

pub(crate) fn try_cached_changed_fast_path(
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
    output_with_prelude(
        output_format_with_json_alias(args.format, args.json),
        &report,
        &prelude,
        || render::changed(&report, changed_section_name(args.section)),
    )?;
    Ok(Some(()))
}

pub(crate) fn maybe_write_changed_lens_cache(
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

pub(crate) fn maybe_write_proof_changed_lens_cache_from_changed(
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

pub(crate) fn maybe_write_proof_map_lens_cache_from_changed(
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
