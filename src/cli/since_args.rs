// `--since <token>` resolution. A 16-hex token is tried as an agent snapshot
// first; otherwise (or on snapshot miss that is a real git object) it is a git
// ref. A 16-hex token that is neither a snapshot nor a git object fails open.
enum SinceKind {
    Snapshot {
        changed: Vec<String>,
        git_state: Vec<crate::model::GitChange>,
    },
    GitRef,
    FailOpen,
}

fn classify_since(project: &crate::model::Project, since: &str) -> SinceKind {
    if !crate::cache::looks_like_snapshot_token(since) {
        return SinceKind::GitRef;
    }
    let current_files: Vec<String> = project.files.keys().cloned().collect();
    let Some(delta) = crate::cache::snapshot_delta(
        &project.root,
        &project.cache_dir,
        repo::VERSION,
        since,
        &current_files,
    ) else {
        return if repo::git_ref_exists(&project.root, since) {
            SinceKind::GitRef
        } else {
            SinceKind::FailOpen
        };
    };
    let mut changed: Vec<String> = delta.changed_or_added.iter().cloned().collect();
    changed.sort();
    SinceKind::Snapshot {
        changed,
        git_state: snapshot_git_state(&delta),
    }
}

fn snapshot_git_state(delta: &crate::cache::fingerprints::CacheFileDelta) -> Vec<crate::model::GitChange> {
    let mut git_state = Vec::new();
    for path in &delta.changed_or_added {
        let status = if delta.cached_content_hashes.contains_key(path) {
            "modified"
        } else {
            "added"
        };
        git_state.push(crate::model::GitChange {
            path: path.clone(),
            old_path: None,
            status: status.to_string(),
            staged: false,
            unstaged: true,
        });
    }
    for path in &delta.removed {
        git_state.push(crate::model::GitChange {
            path: path.clone(),
            old_path: None,
            status: "deleted".to_string(),
            staged: false,
            unstaged: true,
        });
    }
    git_state.sort_by(|a, b| a.path.cmp(&b.path));
    git_state
}

// For lenses without a fail-open notice slot (impact/diff-map/proof-map): a snapshot
// token resolves to its delta set; anything else stays a git ref.
fn since_delta_or_git_ref(project: &crate::model::Project, since: &str) -> Vec<String> {
    match classify_since(project, since) {
        SinceKind::Snapshot { changed, .. } => changed,
        _ => repo::changed_files(&project.root, false, Some(since)),
    }
}

fn proof_since_inputs(project: &crate::model::Project, since: &str) -> ProofInputs {
    match classify_since(project, since) {
        SinceKind::Snapshot { changed, .. } => (
            None,
            changed,
            format!("--since {}", shell_quote_arg(since)),
            None,
        ),
        SinceKind::GitRef => (
            None,
            repo::changed_files(&project.root, false, Some(since)),
            format!("--since {}", shell_quote_arg(since)),
            None,
        ),
        SinceKind::FailOpen => (
            None,
            repo::changed_files(&project.root, false, None),
            "changed".to_string(),
            Some(snapshot_not_found_unknown(since)),
        ),
    }
}

fn snapshot_not_found_unknown(token: &str) -> crate::model::Unknown {
    crate::model::Unknown {
        kind: "snapshot_not_found".to_string(),
        path: None,
        line_start: None,
        reason: format!("snapshot {token} not found; showing full git worktree changed set"),
        effect: "the --since snapshot token could not be resolved in the external cache \
                 (cleared cache or different machine); results fall back to the full \
                 worktree changed set"
            .to_string(),
        expand: Some("codemap changed".to_string()),
    }
}
