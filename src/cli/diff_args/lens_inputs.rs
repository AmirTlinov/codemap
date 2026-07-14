// Responsibility: cli-diff-lens-inputs
use crate::cli::{
    ChangedArgs, DiffMapArgs, ImpactArgs, SinceKind, classify_since, files_selector, parse_files,
    shell_quote_arg, since_delta_or_git_ref, snapshot_not_found_unknown,
};
use crate::{map, repo};
use anyhow::Result;
use anyhow::bail;

pub(crate) fn impact_inputs(
    project: &crate::model::Project,
    args: &ImpactArgs,
) -> Result<(Vec<String>, String)> {
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
        ));
    }
    if args.staged {
        return Ok((
            repo::changed_files(&project.root, true, None),
            "--staged".to_string(),
        ));
    }
    if let Some(since) = &args.since {
        return Ok((
            since_delta_or_git_ref(project, since),
            format!("--since {}", shell_quote_arg(since)),
        ));
    }
    let files = parse_files(project, args.files.as_deref(), &args.positional_files)?;
    Ok((files.clone(), files_selector(&files)))
}

pub(crate) fn diff_map_inputs(
    project: &crate::model::Project,
    args: &DiffMapArgs,
) -> Result<(Vec<String>, String, map::DiffMapMode)> {
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
        ));
    }
    if args.staged {
        return Ok((
            repo::changed_files(&project.root, true, None),
            "--staged".to_string(),
            map::DiffMapMode::Staged,
        ));
    }
    if let Some(since) = &args.since {
        let (changed, mode) = match classify_since(project, since) {
            SinceKind::Snapshot { changed, .. } => (changed, map::DiffMapMode::WorkingTree),
            _ => (
                repo::changed_files(&project.root, false, Some(since)),
                map::DiffMapMode::Since(since.clone()),
            ),
        };
        return Ok((changed, format!("--since {}", shell_quote_arg(since)), mode));
    }
    let files = parse_files(project, args.files.as_deref(), &args.positional_files)?;
    Ok((
        files.clone(),
        files_selector(&files),
        map::DiffMapMode::WorkingTree,
    ))
}

type ChangedInputs = (
    Vec<String>,
    String,
    map::DiffMapMode,
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
        return Ok((
            changed,
            "--staged".to_string(),
            map::DiffMapMode::Staged,
            git_state,
            None,
        ));
    }
    if let Some(since) = &args.since {
        return Ok(match classify_since(project, since) {
            SinceKind::Snapshot { changed, git_state } => (
                changed,
                format!("--since {}", shell_quote_arg(since)),
                map::DiffMapMode::WorkingTree,
                git_state,
                None,
            ),
            SinceKind::GitRef => (
                repo::changed_files(&project.root, false, Some(since)),
                format!("--since {}", shell_quote_arg(since)),
                map::DiffMapMode::Since(since.clone()),
                repo::git_changes(&project.root, false, Some(since)),
                None,
            ),
            SinceKind::FailOpen => (
                repo::changed_files(&project.root, false, None),
                "--changed".to_string(),
                map::DiffMapMode::WorkingTree,
                repo::git_changes(&project.root, false, None),
                Some(snapshot_not_found_unknown(since)),
            ),
        });
    }
    let explicit = parse_files(project, args.files.as_deref(), &args.positional_files)?;
    if !explicit.is_empty() {
        let selector = explicit
            .iter()
            .map(|file| shell_quote_arg(file))
            .collect::<Vec<_>>()
            .join(",");
        let git_state = explicit
            .iter()
            .map(|file| crate::model::GitChange {
                path: file.clone(),
                old_path: None,
                status: "selected".to_string(),
                staged: false,
                unstaged: false,
            })
            .collect();
        return Ok((
            explicit,
            format!("--files {selector}"),
            map::DiffMapMode::WorkingTree,
            git_state,
            None,
        ));
    }
    let changed = repo::changed_files(&project.root, false, None);
    let git_state = repo::git_changes(&project.root, false, None);
    Ok((
        changed,
        "--changed".to_string(),
        map::DiffMapMode::WorkingTree,
        git_state,
        None,
    ))
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
