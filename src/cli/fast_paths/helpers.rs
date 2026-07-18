// Responsibility: cli-fast-paths-helpers
use crate::cli::{
    ChangedArgs, ProofArgs, changed_section_name, repository_relative_arg, shell_quote_arg,
};
use crate::{render, repo};
use anyhow::Result;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn set_cached_map_snapshot(root: &Path, cache_dir: &Path) {
    let fingerprint = crate::cache::cached_status_fingerprint(cache_dir).unwrap_or_else(|| {
        let files = repo::structural_inventory_candidate_files(root);
        crate::cache::inventory_fingerprint(root, &files)
    });
    render::set_cached_map_snapshot_parts(root, Some(&fingerprint), cache_dir);
}

pub(crate) fn set_inventory_map_snapshot(root: &Path) {
    let remote = repo::git_remote(root);
    let cache_dir = crate::cache::project_cache_dir(root, remote.as_deref(), repo::VERSION);
    // Prefer the cached full project fingerprint (the snapshot save-key) so the shown
    // snapshot token resolves under `--since`. Fall back to the inventory fingerprint
    // only on a cold cache, where no snapshot has been saved anyway.
    let fingerprint = crate::cache::cached_status_fingerprint(&cache_dir).unwrap_or_else(|| {
        let files = repo::structural_inventory_candidate_files(root);
        crate::cache::inventory_fingerprint(root, &files)
    });
    render::set_inventory_map_snapshot_parts(root, Some(&fingerprint), &cache_dir);
}

pub(crate) fn set_inventory_map_snapshot_with_fingerprint(root: &Path, fingerprint: &str) {
    let remote = repo::git_remote(root);
    let cache_dir = crate::cache::project_cache_dir(root, remote.as_deref(), repo::VERSION);
    render::set_inventory_map_snapshot_parts(root, Some(fingerprint), &cache_dir);
}

pub(crate) fn root_inventory_has_codemap_config(root: &Path, files: &[String]) -> bool {
    if [".codemap.yml", ".codemap.yaml", ".codemap.json"]
        .iter()
        .any(|name| root.join(name).exists())
    {
        return true;
    }
    files.iter().any(|rel| {
        Path::new(rel)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| matches!(name, ".codemap.yml" | ".codemap.yaml" | ".codemap.json"))
    })
}

pub(crate) fn root_relative_arg(root: &Path, value: &str) -> Result<String> {
    repository_relative_arg(root, value)
        .ok_or_else(|| anyhow::anyhow!("path is outside project root: {value}"))
}

pub(crate) fn changed_selector_state(
    args: &ChangedArgs,
    root: &Path,
) -> (String, Vec<crate::model::GitChange>) {
    (changed_selector(args), changed_git_state(args, root))
}

pub(crate) fn proof_selector_state(
    args: &ProofArgs,
    root: &Path,
) -> (String, Vec<crate::model::GitChange>) {
    (proof_selector(args), proof_git_state(args, root))
}

pub(crate) fn changed_selector(args: &ChangedArgs) -> String {
    if args.staged {
        "--staged".to_string()
    } else if let Some(since) = args.since.as_deref() {
        format!("--since {}", shell_quote_arg(since))
    } else {
        "--changed".to_string()
    }
}

pub(crate) fn proof_selector(args: &ProofArgs) -> String {
    if args.staged {
        "--staged".to_string()
    } else if let Some(since) = args.since.as_deref() {
        format!("--since {}", shell_quote_arg(since))
    } else {
        "changed".to_string()
    }
}

fn changed_git_state(args: &ChangedArgs, root: &Path) -> Vec<crate::model::GitChange> {
    if args.staged {
        repo::git_changes(root, true, None)
    } else if let Some(since) = args.since.as_deref() {
        if crate::cache::looks_like_snapshot_token(since) {
            repo::git_changes(root, false, None)
        } else {
            repo::git_changes(root, false, Some(since))
        }
    } else {
        repo::git_changes(root, false, None)
    }
}

fn proof_git_state(args: &ProofArgs, root: &Path) -> Vec<crate::model::GitChange> {
    if args.target.as_deref() == Some("changed") {
        repo::git_changes(root, false, None)
    } else if args.staged {
        repo::git_changes(root, true, None)
    } else if let Some(since) = args.since.as_deref() {
        if crate::cache::looks_like_snapshot_token(since) {
            repo::git_changes(root, false, None)
        } else {
            repo::git_changes(root, false, Some(since))
        }
    } else {
        repo::git_changes(root, false, None)
    }
}

pub(crate) fn changed_limit(args: &ChangedArgs) -> usize {
    if args.include_hidden {
        usize::MAX / 2
    } else {
        args.limit
    }
}

pub(crate) fn changed_has_explicit_files(args: &ChangedArgs) -> bool {
    args.files
        .as_deref()
        .is_some_and(|files| !files.trim().is_empty())
        || !args.positional_files.is_empty()
}

pub(crate) fn changed_requires_section_report(args: &ChangedArgs) -> bool {
    changed_section_name(args.section)
        .is_some_and(|section| !args.include_hidden && section != "hidden")
}

pub(crate) fn proof_has_explicit_target_or_files(args: &ProofArgs) -> bool {
    args.target
        .as_deref()
        .is_some_and(|target| target != "changed")
        || args
            .files
            .as_deref()
            .is_some_and(|files| !files.trim().is_empty())
}

pub(crate) fn git_change_sets(
    git_state: &[crate::model::GitChange],
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut changed_or_added = BTreeSet::new();
    let mut removed = BTreeSet::new();
    for change in git_state {
        match change.status.as_str() {
            "deleted" => {
                removed.insert(change.path.clone());
            }
            "renamed" => {
                changed_or_added.insert(change.path.clone());
                if let Some(old_path) = &change.old_path {
                    removed.insert(old_path.clone());
                }
            }
            _ => {
                changed_or_added.insert(change.path.clone());
            }
        }
    }
    (changed_or_added, removed)
}
