// Responsibility: cli-diff-lens-inputs
use crate::cli::{
    ChangedArgs, DiffMapArgs, ImpactArgs, SinceKind, classify_since, files_selector, parse_files,
    shell_quote_arg, snapshot_not_found_unknown,
};
use crate::{map, repo};
use anyhow::Result;
use anyhow::bail;

pub(crate) fn impact_inputs(
    project: &crate::model::Project,
    args: &ImpactArgs,
) -> Result<(Vec<String>, String, Option<crate::model::Unknown>)> {
    ensure_single_diff_selector(
        args.changed,
        args.staged,
        args.since.as_deref(),
        args.files.as_deref(),
        &args.positional_files,
    )?;
    if args.changed {
        return Ok((
            repo::changed_files(&project.root, false, None),
            "--changed".to_string(),
            None,
        ));
    }
    if args.staged {
        return Ok((
            repo::changed_files(&project.root, true, None),
            "--staged".to_string(),
            None,
        ));
    }
    if let Some(since) = &args.since {
        return Ok(match classify_since(project, since) {
            SinceKind::Snapshot { changed, .. } => {
                (changed, format!("--since {}", shell_quote_arg(since)), None)
            }
            SinceKind::GitRef => (
                repo::changed_files(&project.root, false, Some(since)),
                format!("--since {}", shell_quote_arg(since)),
                None,
            ),
            SinceKind::FailOpen => {
                let changed = repo::changed_files(&project.root, false, None);
                let notice = snapshot_not_found_unknown(since, changed.len());
                (changed, "--changed".to_string(), Some(notice))
            }
        });
    }
    let files = parse_files(project, args.files.as_deref(), &args.positional_files)?;
    Ok((files.clone(), files_selector(&files), None))
}

pub(crate) fn diff_map_inputs(
    project: &crate::model::Project,
    args: &DiffMapArgs,
) -> Result<(
    Vec<String>,
    String,
    map::DiffMapMode,
    Option<crate::model::Unknown>,
)> {
    ensure_single_diff_selector(
        args.changed,
        args.staged,
        args.since.as_deref(),
        args.files.as_deref(),
        &args.positional_files,
    )?;
    if args.changed {
        return Ok((
            repo::changed_files(&project.root, false, None),
            "--changed".to_string(),
            map::DiffMapMode::WorkingTree,
            None,
        ));
    }
    if args.staged {
        return Ok((
            repo::changed_files(&project.root, true, None),
            "--staged".to_string(),
            map::DiffMapMode::Staged,
            None,
        ));
    }
    if let Some(since) = &args.since {
        let (changed, selector, mode, notice) = match classify_since(project, since) {
            SinceKind::Snapshot { changed, mode, .. } => (
                changed,
                format!("--since {}", shell_quote_arg(since)),
                mode,
                None,
            ),
            SinceKind::GitRef => (
                repo::changed_files(&project.root, false, Some(since)),
                format!("--since {}", shell_quote_arg(since)),
                map::DiffMapMode::Since(since.clone()),
                None,
            ),
            SinceKind::FailOpen => {
                let changed = repo::changed_files(&project.root, false, None);
                let notice = snapshot_not_found_unknown(since, changed.len());
                (
                    changed,
                    "--changed".to_string(),
                    map::DiffMapMode::WorkingTree,
                    Some(notice),
                )
            }
        };
        return Ok((changed, selector, mode, notice));
    }
    let files = parse_files(project, args.files.as_deref(), &args.positional_files)?;
    Ok((
        files.clone(),
        files_selector(&files),
        map::DiffMapMode::WorkingTree,
        None,
    ))
}

type ChangedInputs = (
    Vec<String>,
    String,
    map::ChangedDiffContext,
    Vec<crate::model::GitChange>,
    Option<crate::model::Unknown>,
);

pub(crate) fn changed_inputs(
    project: &crate::model::Project,
    args: &ChangedArgs,
) -> Result<ChangedInputs> {
    ensure_single_diff_selector(
        args.changed,
        args.staged,
        args.since.as_deref(),
        args.files.as_deref(),
        &args.positional_files,
    )?;
    if args.staged {
        let changed = repo::changed_files(&project.root, true, None);
        let git_state = repo::git_changes(&project.root, true, None);
        let selection = change_selection("staged", None, true, changed.len(), 0, true, None);
        return Ok((
            changed,
            "--staged".to_string(),
            changed_context(map::DiffMapMode::Staged, selection),
            git_state,
            None,
        ));
    }
    if let Some(since) = &args.since {
        return Ok(match classify_since(project, since) {
            SinceKind::Snapshot {
                changed,
                git_state,
                mode,
                metadata,
                content_complete,
            } => {
                let selected = changed.len();
                let baseline = map::session_snapshot_from_metadata(*metadata);
                let selection = change_selection(
                    "snapshot",
                    Some(since.clone()),
                    true,
                    selected,
                    0,
                    content_complete,
                    Some(baseline),
                );
                (
                    changed,
                    format!("--since {}", shell_quote_arg(since)),
                    changed_context(mode, selection),
                    git_state,
                    (!content_complete).then(|| crate::cli::snapshot_content_unknown(since)),
                )
            }
            SinceKind::GitRef => {
                let changed = repo::changed_files(&project.root, false, Some(since));
                let selected = changed.len();
                let selection = change_selection(
                    "git_ref",
                    Some(since.clone()),
                    true,
                    selected,
                    0,
                    true,
                    None,
                );
                (
                    changed,
                    format!("--since {}", shell_quote_arg(since)),
                    changed_context(map::DiffMapMode::Since(since.clone()), selection),
                    repo::git_changes(&project.root, false, Some(since)),
                    None,
                )
            }
            SinceKind::FailOpen => {
                let changed = repo::changed_files(&project.root, false, None);
                let fallback = changed.len();
                let selection = change_selection(
                    "snapshot_fallback",
                    Some(since.clone()),
                    false,
                    fallback,
                    fallback,
                    false,
                    None,
                );
                (
                    changed,
                    "--changed".to_string(),
                    changed_context(map::DiffMapMode::WorkingTree, selection),
                    repo::git_changes(&project.root, false, None),
                    Some(snapshot_not_found_unknown(since, fallback)),
                )
            }
        });
    }
    let explicit = parse_files(project, args.files.as_deref(), &args.positional_files)?;
    if !explicit.is_empty() {
        let selector = explicit
            .iter()
            .map(|file| shell_quote_arg(file))
            .collect::<Vec<_>>()
            .join(",");
        let git_state: Vec<crate::model::GitChange> = explicit
            .iter()
            .map(|file| crate::model::GitChange {
                path: file.clone(),
                old_path: None,
                status: "selected".to_string(),
                staged: false,
                unstaged: false,
                provenance: "explicit_selection".to_string(),
            })
            .collect();
        let selected = git_state.len();
        return Ok((
            explicit,
            format!("--files {selector}"),
            changed_context(
                map::DiffMapMode::WorkingTree,
                change_selection("explicit_files", None, true, selected, 0, true, None),
            ),
            git_state,
            None,
        ));
    }
    let changed = repo::changed_files(&project.root, false, None);
    let git_state = repo::git_changes(&project.root, false, None);
    let selection = change_selection("worktree", None, true, changed.len(), 0, true, None);
    Ok((
        changed,
        "--changed".to_string(),
        changed_context(map::DiffMapMode::WorkingTree, selection),
        git_state,
        None,
    ))
}

fn changed_context(
    mode: map::DiffMapMode,
    selection: crate::model::ChangeSelection,
) -> map::ChangedDiffContext {
    map::ChangedDiffContext { mode, selection }
}

fn change_selection(
    kind: &str,
    requested: Option<String>,
    resolved: bool,
    selected_files: usize,
    fallback_files: usize,
    content_complete: bool,
    baseline_snapshot: Option<crate::model::SessionSnapshot>,
) -> crate::model::ChangeSelection {
    crate::model::ChangeSelection {
        kind: kind.to_string(),
        requested,
        resolved,
        selected_files,
        fallback_files,
        content_complete,
        baseline_snapshot,
    }
}

pub(crate) fn ensure_single_diff_selector(
    changed: bool,
    staged: bool,
    since: Option<&str>,
    files: Option<&str>,
    positional_files: &[String],
) -> Result<()> {
    let explicit_files = files.map(|value| !value.trim().is_empty()).unwrap_or(false)
        || !positional_files.is_empty();
    let count = [changed, staged, since.is_some(), explicit_files]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();
    if count > 1 {
        bail!("choose only one diff selector: --changed, --staged, --since, or explicit files");
    }
    Ok(())
}
