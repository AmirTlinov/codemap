fn ensure_valid_config(project: &crate::model::Project) -> Result<()> {
    let semantic_problems = semantic_anchor_problems(project);
    if project.config_errors.is_empty() && semantic_problems.is_empty() {
        return Ok(());
    }
    for error in &project.config_errors {
        eprintln!(
            "codemap: invalid semantic anchor `{}`: {}",
            error.path, error.error
        );
    }
    for problem in semantic_problems {
        eprintln!("codemap: invalid semantic anchor: {problem}");
    }
    bail!("invalid .ctx semantic anchors; run `codemap anchors validate`")
}

fn changed_from_args(project: &crate::model::Project, args: &ImpactArgs) -> Result<Vec<String>> {
    ensure_single_diff_selector(
        args.changed,
        args.staged,
        args.since.as_deref(),
        args.files.as_deref(),
        &args.positional_files,
    )?;
    if args.changed {
        return Ok(repo::changed_files(&project.root, false, None));
    }
    if args.staged {
        return Ok(repo::changed_files(&project.root, true, None));
    }
    if let Some(since) = &args.since {
        return Ok(repo::changed_files(&project.root, false, Some(since)));
    }
    parse_files(project, args.files.as_deref(), &args.positional_files)
}

fn proof_inputs(
    project: &crate::model::Project,
    args: &ProofArgs,
) -> Result<(Option<String>, Vec<String>)> {
    ensure_single_proof_selector(args)?;
    if let Some(target) = args.target.as_deref() {
        return Ok((Some(project_relative_arg(project, target)?), Vec::new()));
    }
    if args.changed {
        return Ok((None, repo::changed_files(&project.root, false, None)));
    }
    if args.staged {
        return Ok((None, repo::changed_files(&project.root, true, None)));
    }
    if let Some(since) = &args.since {
        return Ok((None, repo::changed_files(&project.root, false, Some(since))));
    }
    let files = parse_files(project, args.files.as_deref(), &[])?;
    if !files.is_empty() {
        return Ok((None, files));
    }
    bail!("codemap proof needs an exact target, --changed, --staged, --since, or --files");
}

fn ensure_single_proof_selector(args: &ProofArgs) -> Result<()> {
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
        bail!("choose only one proof selector: target, --changed, --staged, --since, or --files");
    }
    Ok(())
}

fn ensure_single_diff_selector(
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

fn parse_files(
    project: &crate::model::Project,
    files: Option<&str>,
    positional: &[String],
) -> Result<Vec<String>> {
    let mut out = Vec::new();
    if let Some(files) = files {
        for file in files.split(',') {
            out.push(project_relative_arg(project, file)?);
        }
    }
    for file in positional {
        out.push(project_relative_arg(project, file)?);
    }
    Ok(out.into_iter().filter(|s| s != ".").collect())
}

fn project_relative_arg(project: &crate::model::Project, value: &str) -> Result<String> {
    let path = Path::new(value);
    let root = normalize_absolute_arg(&project.root);
    let absolute = if path.is_absolute() {
        normalize_absolute_arg(path)
    } else {
        normalize_absolute_arg(&root.join(path))
    };
    absolute
        .strip_prefix(root)
        .map(|rel| repo::normalize_rel_path(&rel.to_string_lossy()))
        .map_err(|_| anyhow::anyhow!("path is outside project root: {value}"))
}

fn normalize_absolute_arg(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    let mut tail = Vec::new();
    let mut cursor = path;
    loop {
        if cursor.exists() {
            let mut out = cursor
                .canonicalize()
                .unwrap_or_else(|_| lexical_normalize_absolute(cursor));
            for part in tail.iter().rev() {
                out.push(part);
            }
            return lexical_normalize_absolute(&out);
        }
        let Some(parent) = cursor.parent() else {
            return lexical_normalize_absolute(path);
        };
        if parent == cursor {
            return lexical_normalize_absolute(path);
        }
        let Some(name) = cursor.file_name() else {
            return lexical_normalize_absolute(path);
        };
        tail.push(PathBuf::from(name));
        cursor = parent;
    }
}

fn lexical_normalize_absolute(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            std::path::Component::RootDir => out.push(std::path::MAIN_SEPARATOR.to_string()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn scoped_project_path(project: &crate::model::Project, value: &str) -> Result<PathBuf> {
    project_relative_arg(project, value)
        .map(|rel| project.root.join(rel))
        .map_err(|_| anyhow::anyhow!("refusing to write outside project root: {value}"))
}

