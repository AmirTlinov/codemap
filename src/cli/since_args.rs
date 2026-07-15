// Responsibility: cli-since-args
use crate::cli::{ProofInputs, shell_quote_arg};
use crate::repo;

// `--since <token>` resolution. A 16-hex token is tried as an agent snapshot
// first; otherwise (or on snapshot miss that is a real git object) it is a git
// ref. A 16-hex token that is neither a snapshot nor a git object fails open.
pub(crate) enum SinceKind {
    Snapshot {
        changed: Vec<String>,
        git_state: Vec<crate::model::GitChange>,
        mode: crate::map::DiffMapMode,
        metadata: Box<crate::cache::SnapshotMetadata>,
        content_complete: bool,
    },
    GitRef,
    FailOpen,
}

pub(crate) fn classify_since(project: &crate::model::Project, since: &str) -> SinceKind {
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
    let mut changed: Vec<String> = delta.files.changed_or_added.iter().cloned().collect();
    changed.extend(delta.files.removed.iter().cloned());
    changed.sort();
    changed.dedup();
    let git_state = snapshot_git_state(project, &delta);
    let mode = crate::map::DiffMapMode::Snapshot(crate::map::SnapshotDiffBase {
        token: since.to_string(),
        texts: delta.base_texts,
        content_complete: delta.content_complete,
    });
    SinceKind::Snapshot {
        changed,
        git_state,
        mode,
        metadata: Box::new(delta.metadata),
        content_complete: delta.content_complete,
    }
}

fn snapshot_git_state(
    project: &crate::model::Project,
    delta: &crate::cache::SnapshotDelta,
) -> Vec<crate::model::GitChange> {
    let mut git_state = Vec::new();
    for path in &delta.files.changed_or_added {
        let existed_at_snapshot = delta.files.cached_content_hashes.contains_key(path);
        let exists_now = std::fs::symlink_metadata(project.root.join(path)).is_ok();
        let status = match (existed_at_snapshot, exists_now) {
            (true, true) => "modified",
            (true, false) => "deleted",
            (false, true) => "added",
            (false, false) => continue,
        };
        git_state.push(crate::model::GitChange {
            path: path.clone(),
            old_path: None,
            status: status.to_string(),
            staged: false,
            unstaged: true,
            provenance: "snapshot_delta".to_string(),
        });
    }
    for path in &delta.files.removed {
        git_state.push(crate::model::GitChange {
            path: path.clone(),
            old_path: None,
            status: "deleted".to_string(),
            staged: false,
            unstaged: true,
            provenance: "snapshot_delta".to_string(),
        });
    }
    apply_snapshot_identities(project, delta, &mut git_state);
    overlay_worktree_conflicts(project, &mut git_state);
    git_state.sort_by(|a, b| a.path.cmp(&b.path));
    git_state
}

fn apply_snapshot_identities(
    project: &crate::model::Project,
    delta: &crate::cache::SnapshotDelta,
    git_state: &mut Vec<crate::model::GitChange>,
) {
    for change in git_state
        .iter_mut()
        .filter(|change| change.status == "modified")
    {
        let old_kind = delta
            .base_files
            .get(&change.path)
            .map(|file| file.node_kind.as_str());
        let new_kind = snapshot_node_kind(project, &change.path);
        if old_kind.is_some_and(|kind| kind != new_kind) {
            change.status = "typechanged".to_string();
        }
    }
    let mut removed_by_hash = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut added_by_hash = std::collections::BTreeMap::<String, Vec<String>>::new();
    for change in git_state.iter().filter(|change| change.status == "deleted") {
        if let Some(hash) = delta
            .base_files
            .get(&change.path)
            .and_then(|file| file.content_hash.clone())
        {
            removed_by_hash
                .entry(hash)
                .or_default()
                .push(change.path.clone());
        }
    }
    for change in git_state.iter().filter(|change| change.status == "added") {
        if let Some(hash) = project
            .files
            .get(&change.path)
            .and_then(|file| file.content_hash.clone())
        {
            added_by_hash
                .entry(hash)
                .or_default()
                .push(change.path.clone());
        }
    }
    let mut renamed = Vec::new();
    for (hash, old_paths) in removed_by_hash {
        let Some(new_paths) = added_by_hash.get(&hash) else {
            continue;
        };
        // Content identity proves a rename only when both sides are unique.
        if old_paths.len() == 1 && new_paths.len() == 1 {
            renamed.push((old_paths[0].clone(), new_paths[0].clone()));
        }
    }
    if renamed.is_empty() {
        return;
    }
    git_state.retain(|change| {
        !renamed.iter().any(|(old, new)| {
            (change.status == "deleted" && &change.path == old)
                || (change.status == "added" && &change.path == new)
        })
    });
    git_state.extend(
        renamed
            .into_iter()
            .map(|(old_path, path)| crate::model::GitChange {
                path,
                old_path: Some(old_path),
                status: "renamed".to_string(),
                staged: false,
                unstaged: true,
                provenance: "snapshot_content_identity".to_string(),
            }),
    );
}

fn overlay_worktree_conflicts(
    project: &crate::model::Project,
    git_state: &mut [crate::model::GitChange],
) {
    let conflicts = repo::git_changes(&project.root, false, None)
        .into_iter()
        .filter(|change| change.status == "conflicted")
        .map(|change| change.path)
        .collect::<std::collections::BTreeSet<_>>();
    for change in git_state {
        if conflicts.contains(&change.path) {
            change.status = "conflicted".to_string();
            change.provenance = "git_status_conflict".to_string();
        }
    }
}

fn snapshot_node_kind(project: &crate::model::Project, rel: &str) -> String {
    match std::fs::symlink_metadata(project.root.join(rel)) {
        Ok(metadata) if metadata.file_type().is_symlink() => "symlink",
        Ok(metadata) if metadata.is_file() => "file",
        Ok(metadata) if metadata.is_dir() => "directory",
        Ok(_) => "other",
        Err(_) => "unavailable",
    }
    .to_string()
}

pub(crate) fn snapshot_content_unknown(token: &str) -> crate::model::Unknown {
    crate::model::Unknown {
        kind: "snapshot_content_unavailable".to_string(),
        path: None,
        line_start: None,
        reason: format!("snapshot {token} resolves its file set, but one or more baseline bodies are unavailable"),
        effect: "entity and relation deltas remain open for the affected files; file selection and identity provenance stay exact".to_string(),
        expand: Some(format!("codemap changed --since {token} --section unknown")),
    }
}

pub(crate) fn proof_since_inputs(project: &crate::model::Project, since: &str) -> ProofInputs {
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
        SinceKind::FailOpen => {
            let changed = repo::changed_files(&project.root, false, None);
            let fallback_files = changed.len();
            (
                None,
                changed,
                "changed".to_string(),
                Some(snapshot_not_found_unknown(since, fallback_files)),
            )
        }
    }
}

pub(crate) fn snapshot_not_found_unknown(
    token: &str,
    fallback_files: usize,
) -> crate::model::Unknown {
    crate::model::Unknown {
        kind: "snapshot_not_found".to_string(),
        path: None,
        line_start: None,
        reason: format!(
            "snapshot {token} not found; showing full git worktree changed set ({fallback_files} files)"
        ),
        effect: "the --since snapshot token could not be resolved in the external cache \
                 (cleared cache or different machine); results fall back to the full \
                 worktree changed set"
            .to_string(),
        expand: Some("codemap changed".to_string()),
    }
}
