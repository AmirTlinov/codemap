fn set_cached_map_snapshot(root: &Path, cache_dir: &Path) {
    let fingerprint = crate::cache::cached_status_fingerprint(cache_dir).unwrap_or_else(|| {
        let files = repo::structural_inventory_candidate_files(root);
        crate::cache::inventory_fingerprint(root, &files)
    });
    render::set_cached_map_snapshot_parts(root, Some(&fingerprint), cache_dir);
}

fn set_inventory_map_snapshot(root: &Path) {
    let files = repo::structural_inventory_candidate_files(root);
    let fingerprint = crate::cache::inventory_fingerprint(root, &files);
    set_inventory_map_snapshot_with_fingerprint(root, &fingerprint);
}

fn set_inventory_map_snapshot_with_fingerprint(root: &Path, fingerprint: &str) {
    let remote = repo::git_remote(root);
    let cache_dir = crate::cache::project_cache_dir(root, remote.as_deref(), repo::VERSION);
    render::set_inventory_map_snapshot_parts(root, Some(fingerprint), &cache_dir);
}

fn root_relative_arg(root: &Path, value: &str) -> Result<String> {
    let path = Path::new(value);
    let normalized_root = normalize_absolute_arg(root);
    let absolute = if path.is_absolute() {
        normalize_absolute_arg(path)
    } else {
        normalize_absolute_arg(&normalized_root.join(path))
    };
    absolute
        .strip_prefix(normalized_root)
        .map(|rel| repo::normalize_rel_path(&rel.to_string_lossy()))
        .map_err(|_| anyhow::anyhow!("path is outside project root: {value}"))
}

fn changed_selector_state(args: &ChangedArgs, root: &Path) -> (String, Vec<crate::model::GitChange>) {
    (changed_selector(args), changed_git_state(args, root))
}

fn proof_selector_state(args: &ProofArgs, root: &Path) -> (String, Vec<crate::model::GitChange>) {
    (proof_selector(args), proof_git_state(args, root))
}

fn changed_selector(args: &ChangedArgs) -> String {
    if args.staged {
        "--staged".to_string()
    } else if let Some(since) = args.since.as_deref() {
        format!("--since {}", shell_quote_arg(since))
    } else {
        "--changed".to_string()
    }
}

fn proof_selector(args: &ProofArgs) -> String {
    if args.target.as_deref() == Some("changed") {
        "changed".to_string()
    } else if args.staged {
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
        repo::git_changes(root, false, Some(since))
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
        repo::git_changes(root, false, Some(since))
    } else {
        repo::git_changes(root, false, None)
    }
}

fn changed_limit(args: &ChangedArgs) -> usize {
    if args.include_hidden {
        usize::MAX / 2
    } else {
        args.limit
    }
}

fn changed_has_explicit_files(args: &ChangedArgs) -> bool {
    args.files
        .as_deref()
        .is_some_and(|files| !files.trim().is_empty())
        || !args.positional_files.is_empty()
}

fn changed_requires_section_report(args: &ChangedArgs) -> bool {
    changed_section_name(args.section)
        .is_some_and(|section| !args.include_hidden && section != "hidden")
}

fn proof_has_explicit_target_or_files(args: &ProofArgs) -> bool {
    args.target
        .as_deref()
        .is_some_and(|target| target != "changed")
        || args
            .files
            .as_deref()
            .is_some_and(|files| !files.trim().is_empty())
}

fn git_change_sets(
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
