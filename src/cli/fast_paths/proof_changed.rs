// Responsibility: cli-fast-paths-proof-changed
use crate::cli::{
    CommandKind, ensure_single_proof_selector, lens_cache_matches_current,
    output_format_with_json_alias, output_with_prelude, proof_has_explicit_target_or_files,
    proof_section_name, proof_selector_state, set_cached_map_snapshot, set_inventory_map_snapshot,
    since_is_snapshot_token,
};
use crate::{map, render, repo};
use anyhow::Result;
use std::env;

pub(crate) fn try_clean_proof_changed_fast_path(
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
    output_with_prelude(
        output_format_with_json_alias(args.format, args.json),
        &report,
        &prelude,
        || render::proof(&report, proof_section_name(args.section)),
    )?;
    Ok(Some(()))
}

pub(crate) fn try_cached_proof_changed_fast_path(
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
    if args.target.as_deref() != Some("changed") && !args.staged && args.since.is_none() {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let (selector, git_state) = proof_selector_state(args, &root);
    if git_state.is_empty() && !since_is_snapshot_token(args.since.as_deref()) {
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
    output_with_prelude(
        output_format_with_json_alias(args.format, args.json),
        &report,
        &prelude,
        || render::proof(&report, proof_section_name(args.section)),
    )?;
    Ok(Some(()))
}
