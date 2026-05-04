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
    bail!("invalid .codemap semantic anchors; run `codemap anchors validate`")
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

fn changed_from_diff_map_args(
    project: &crate::model::Project,
    args: &DiffMapArgs,
) -> Result<Vec<String>> {
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

fn changed_inputs(
    project: &crate::model::Project,
    args: &ChangedArgs,
) -> Result<(Vec<String>, String, map::DiffMapMode, Vec<crate::model::GitChange>)> {
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
        ));
    }
    if let Some(since) = &args.since {
        let changed = repo::changed_files(&project.root, false, Some(since));
        let git_state = repo::git_changes(&project.root, false, Some(since));
        return Ok((
            changed,
            format!("--since {}", shell_quote_arg(since)),
            map::DiffMapMode::Since(since.clone()),
            git_state,
        ));
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
        ));
    }
    let changed = repo::changed_files(&project.root, false, None);
    let git_state = repo::git_changes(&project.root, false, None);
    Ok((
        changed,
        "--changed".to_string(),
        map::DiffMapMode::WorkingTree,
        git_state,
    ))
}

fn changed_section_name(section: Option<ChangedSection>) -> Option<&'static str> {
    match section {
        Some(ChangedSection::Observed) => Some("observed"),
        Some(ChangedSection::Links) => Some("links"),
        Some(ChangedSection::Roles) => Some("roles"),
        Some(ChangedSection::Proof) => Some("proof"),
        Some(ChangedSection::Unknown) => Some("unknown"),
        Some(ChangedSection::Hidden) => Some("hidden"),
        None => None,
    }
}

fn ls_section_name(section: Option<LsSection>) -> Option<&'static str> {
    match section {
        Some(LsSection::Observed) => Some("observed"),
        Some(LsSection::Links) => Some("links"),
        Some(LsSection::Roles) => Some("roles"),
        Some(LsSection::Proof) => Some("proof"),
        Some(LsSection::Unknown) => Some("unknown"),
        Some(LsSection::Hidden) => Some("hidden"),
        None => None,
    }
}

fn cone_section_name(section: Option<ConeSection>) -> Option<&'static str> {
    match section {
        Some(ConeSection::Observed) => Some("observed"),
        Some(ConeSection::Links) => Some("links"),
        Some(ConeSection::Roles) => Some("roles"),
        Some(ConeSection::Proof) => Some("proof"),
        Some(ConeSection::Unknown) => Some("unknown"),
        Some(ConeSection::Hidden) => Some("hidden"),
        None => None,
    }
}

fn proof_section_name(section: Option<ProofSection>) -> Option<&'static str> {
    match section {
        Some(ProofSection::Observed) => Some("observed"),
        Some(ProofSection::Links) => Some("links"),
        Some(ProofSection::Roles) => Some("roles"),
        Some(ProofSection::Proof) => Some("proof"),
        Some(ProofSection::Unknown) => Some("unknown"),
        Some(ProofSection::Hidden) => Some("hidden"),
        None => None,
    }
}

fn proof_map_inputs(
    project: &crate::model::Project,
    args: &ProofMapArgs,
) -> Result<(Option<String>, Vec<String>, String)> {
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
        bail!("choose only one proof-map selector: target, --changed, --staged, --since, or --files");
    }
    if let Some(target) = args.target.as_deref() {
        let target = project_relative_arg(project, target)?;
        let selector = shell_quote_arg(&target);
        return Ok((Some(target), Vec::new(), selector));
    }
    if args.changed {
        return Ok((
            None,
            repo::changed_files(&project.root, false, None),
            "--changed".to_string(),
        ));
    }
    if args.staged {
        return Ok((
            None,
            repo::changed_files(&project.root, true, None),
            "--staged".to_string(),
        ));
    }
    if let Some(since) = &args.since {
        return Ok((
            None,
            repo::changed_files(&project.root, false, Some(since)),
            format!("--since {}", shell_quote_arg(since)),
        ));
    }
    let files = parse_files(project, args.files.as_deref(), &[])?;
    if files.is_empty() {
        return Ok((
            None,
            repo::changed_files(&project.root, false, None),
            "--changed".to_string(),
        ));
    }
    let files_arg = files
        .iter()
        .map(|file| shell_quote_arg(file))
        .collect::<Vec<_>>()
        .join(",");
    Ok((None, files, format!("--files {files_arg}")))
}

fn proof_inputs(
    project: &crate::model::Project,
    args: &ProofArgs,
) -> Result<(Option<String>, Vec<String>, String)> {
    ensure_single_proof_selector(args)?;
    if let Some(target) = args.target.as_deref() {
        if target == "changed" {
            return Ok((
                None,
                repo::changed_files(&project.root, false, None),
                "changed".to_string(),
            ));
        }
        let target = project_relative_arg(project, target)?;
        let selector = shell_quote_arg(&target);
        return Ok((Some(target), Vec::new(), selector));
    }
    if args.staged {
        return Ok((
            None,
            repo::changed_files(&project.root, true, None),
            "--staged".to_string(),
        ));
    }
    if let Some(since) = &args.since {
        return Ok((
            None,
            repo::changed_files(&project.root, false, Some(since)),
            format!("--since {}", shell_quote_arg(since)),
        ));
    }
    let files = parse_files(project, args.files.as_deref(), &[])?;
    if !files.is_empty() {
        let files_arg = files
            .iter()
            .map(|file| shell_quote_arg(file))
            .collect::<Vec<_>>()
            .join(",");
        return Ok((None, files, format!("--files {files_arg}")));
    }
    bail!("codemap proof needs an exact target, changed, --staged, --since, or --files");
}

fn ensure_single_proof_selector(args: &ProofArgs) -> Result<()> {
    if args.changed {
        bail!("`codemap proof --changed` was replaced by `codemap proof changed`");
    }
    let explicit_files = args
        .files
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let count = [
        args.target.is_some(),
        args.staged,
        args.since.is_some(),
        explicit_files,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count();
    if count > 1 {
        bail!("choose only one proof selector: target, changed, --staged, --since, or --files");
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

fn flow_anchor_arg(project: &crate::model::Project, value: &str) -> Result<String> {
    let trimmed = value.trim();
    if cli_route_anchor(project, trimmed) {
        return Ok(trimmed.to_string());
    }
    project_relative_arg(project, trimmed)
}

fn cli_route_anchor(project: &crate::model::Project, value: &str) -> bool {
    if value.starts_with('/') {
        let root = normalize_absolute_arg(&project.root);
        let root = root.to_string_lossy();
        return !value.starts_with(root.as_ref());
    }
    let Some((method, path)) = value.split_once(' ') else {
        return false;
    };
    path.trim().starts_with('/')
        && matches!(
            method.trim().to_ascii_uppercase().as_str(),
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "ALL" | "HEAD" | "OPTIONS"
        )
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
