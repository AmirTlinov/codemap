// Responsibility: cli-fast-paths-proof-map
use crate::cli::{
    CommandKind, ProofMapArgs, lens_cache_matches_current, output, root_relative_arg,
    set_cached_map_snapshot, shell_quote_arg,
};
use crate::{render, repo};
use anyhow::Result;
use anyhow::bail;
use std::collections::BTreeSet;
use std::env;
use std::path::Path;

pub(crate) fn try_cached_proof_map_fast_path(
    command: &CommandKind,
    root_selection: &repo::RootSelection,
) -> Result<Option<()>> {
    let CommandKind::ProofMap(args) = command else {
        return Ok(None);
    };
    ensure_single_proof_map_selector(args)?;
    if proof_map_has_explicit_files(args) {
        return Ok(None);
    }
    let cwd = env::current_dir()?;
    let root = repo::resolve_root(root_selection, &cwd)?;
    let selector_state = proof_map_selector_state(args, &root)?;
    let remote = repo::git_remote(&root);
    let cache_dir = crate::cache::project_cache_dir(&root, remote.as_deref(), repo::VERSION);
    if !lens_cache_matches_current(&root, &cache_dir, &selector_state.git_state) {
        return Ok(None);
    }
    set_cached_map_snapshot(&root, &cache_dir);
    let Some(report) = crate::cache::read_proof_map_report(
        &cache_dir,
        repo::VERSION,
        &root,
        selector_state.scope.as_deref(),
        &selector_state.selector,
        args.limit,
        args.raw_sensors,
    ) else {
        return Ok(None);
    };
    if !same_string_set(&report.changed, &selector_state.changed) {
        return Ok(None);
    }
    output(args.format, &report, || render::proof_map(&report))?;
    Ok(Some(()))
}

pub(crate) fn maybe_write_proof_map_lens_cache(
    project: &crate::model::Project,
    scope: Option<&str>,
    selector: &str,
    args: &ProofMapArgs,
    report: &crate::model::ProofMapReport,
) {
    if proof_map_has_explicit_files(args) {
        return;
    }
    if scope != report.scope.as_deref() {
        return;
    }
    let _ = crate::cache::write_proof_map_report(
        &project.cache_dir,
        repo::VERSION,
        &project.root,
        selector,
        args.limit,
        args.raw_sensors,
        report,
    );
}

struct ProofMapCacheSelector {
    scope: Option<String>,
    selector: String,
    git_state: Vec<crate::model::GitChange>,
    changed: Vec<String>,
}

fn proof_map_selector_state(args: &ProofMapArgs, root: &Path) -> Result<ProofMapCacheSelector> {
    if let Some(target) = args.target.as_deref() {
        let target = root_relative_arg(root, target)?;
        let selector = shell_quote_arg(&target);
        let git_state = repo::git_changes(root, false, None);
        return Ok(ProofMapCacheSelector {
            scope: Some(target),
            selector,
            git_state,
            changed: Vec::new(),
        });
    }
    if args.staged {
        let changed = repo::changed_files(root, true, None);
        return Ok(ProofMapCacheSelector {
            scope: None,
            selector: "--staged".to_string(),
            git_state: repo::git_changes(root, true, None),
            changed,
        });
    }
    if let Some(since) = args.since.as_deref() {
        let changed = repo::changed_files(root, false, Some(since));
        return Ok(ProofMapCacheSelector {
            scope: None,
            selector: format!("--since {}", shell_quote_arg(since)),
            git_state: repo::git_changes(root, false, Some(since)),
            changed,
        });
    }
    let changed = repo::changed_files(root, false, None);
    Ok(ProofMapCacheSelector {
        scope: None,
        selector: "--changed".to_string(),
        git_state: repo::git_changes(root, false, None),
        changed,
    })
}

pub(crate) fn ensure_single_proof_map_selector(args: &ProofMapArgs) -> Result<()> {
    let count = [
        args.target.is_some(),
        args.changed,
        args.staged,
        args.since.is_some(),
        proof_map_has_explicit_files(args),
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    if count > 1 {
        bail!(
            "choose only one proof-map selector: target, --changed, --staged, --since, or --files"
        );
    }
    Ok(())
}

pub(crate) fn proof_map_has_explicit_files(args: &ProofMapArgs) -> bool {
    args.files
        .as_deref()
        .is_some_and(|files| !files.trim().is_empty())
}

fn same_string_set(left: &[String], right: &[String]) -> bool {
    left.iter().collect::<BTreeSet<_>>() == right.iter().collect::<BTreeSet<_>>()
}
