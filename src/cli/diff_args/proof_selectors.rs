// Responsibility: cli-proof-selector-inputs
use crate::cli::{
    ProofArgs, ProofMapArgs, SinceKind, classify_since, parse_files, project_relative_arg,
    proof_since_inputs, shell_quote_arg, snapshot_not_found_unknown,
};
use crate::repo;
use anyhow::Result;

pub(crate) type ProofMapInputs = (
    Option<String>,
    Vec<String>,
    String,
    Option<crate::model::Unknown>,
);

pub(crate) fn proof_map_inputs(
    project: &crate::model::Project,
    args: &ProofMapArgs,
) -> Result<ProofMapInputs> {
    let explicit_files = args
        .files
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let count = [
        args.target.is_some(),
        args.changed,
        args.staged,
        args.since.is_some(),
        explicit_files,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    if count > 1 {
        return Err(crate::cli::invalid_input(
            "choose only one proof-map selector: target, --changed, --staged, --since, or --files",
        ));
    }
    if let Some(target) = args.target.as_deref() {
        let target = project_relative_arg(project, target)?;
        let selector = shell_quote_arg(&target);
        return Ok((Some(target), Vec::new(), selector, None));
    }
    if args.changed {
        return Ok((
            None,
            repo::changed_files(&project.root, false, None),
            "--changed".to_string(),
            None,
        ));
    }
    if args.staged {
        return Ok((
            None,
            repo::changed_files(&project.root, true, None),
            "--staged".to_string(),
            None,
        ));
    }
    if let Some(since) = &args.since {
        return Ok(match classify_since(project, since) {
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
                let notice = snapshot_not_found_unknown(since, changed.len());
                (None, changed, "--changed".to_string(), Some(notice))
            }
        });
    }
    let files = parse_files(project, args.files.as_deref(), &[])?;
    if files.is_empty() {
        return Ok((
            None,
            repo::changed_files(&project.root, false, None),
            "--changed".to_string(),
            None,
        ));
    }
    let files_arg = files
        .iter()
        .map(|file| shell_quote_arg(file))
        .collect::<Vec<_>>()
        .join(",");
    Ok((None, files, format!("--files {files_arg}"), None))
}

pub(crate) type ProofInputs = (
    Option<String>,
    Vec<String>,
    String,
    Option<crate::model::Unknown>,
);

pub(crate) fn proof_inputs(
    project: &crate::model::Project,
    args: &ProofArgs,
) -> Result<ProofInputs> {
    ensure_single_proof_selector(args)?;
    if let Some(target) = args.target.as_deref() {
        if target == "changed" {
            if let Some(since) = &args.since {
                return Ok(proof_since_inputs(project, since));
            }
            return Ok((
                None,
                repo::changed_files(&project.root, false, None),
                "changed".to_string(),
                None,
            ));
        }
        let target = project_relative_arg(project, target)?;
        let selector = shell_quote_arg(&target);
        return Ok((Some(target), Vec::new(), selector, None));
    }
    if args.staged {
        return Ok((
            None,
            repo::changed_files(&project.root, true, None),
            "--staged".to_string(),
            None,
        ));
    }
    if let Some(since) = &args.since {
        return Ok(proof_since_inputs(project, since));
    }
    let files = parse_files(project, args.files.as_deref(), &[])?;
    if !files.is_empty() {
        let files_arg = files
            .iter()
            .map(|file| shell_quote_arg(file))
            .collect::<Vec<_>>()
            .join(",");
        return Ok((None, files, format!("--files {files_arg}"), None));
    }
    Err(crate::cli::invalid_input(
        "codemap proof needs an exact target, changed, --staged, --since, or --files",
    ))
}

pub(crate) fn ensure_single_proof_selector(args: &ProofArgs) -> Result<()> {
    if args.changed {
        return Err(crate::cli::unsupported_request(
            "`codemap proof --changed` was replaced by `codemap proof changed`",
        ));
    }
    let explicit_files = args
        .files
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    // `proof changed --since <token>` is one selector: proof over the since-delta.
    let target_is_changed_since = args.target.as_deref() == Some("changed") && args.since.is_some();
    let count = [
        args.target.is_some() && !target_is_changed_since,
        args.staged,
        args.since.is_some(),
        explicit_files,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    if count > 1 {
        return Err(crate::cli::invalid_input(
            "choose only one proof selector: target, changed, --staged, --since, or --files",
        ));
    }
    Ok(())
}
