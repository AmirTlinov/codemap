fn artifact_paths_for_command_or_file(
    project: &Project,
    command: &str,
    path: Option<&str>,
) -> Vec<String> {
    let mut out = BTreeSet::new();
    collect_artifact_path_tokens(command, &mut out);
    if let Some(parsed) = parse_static_command(command) {
        collect_artifact_paths_from_resolved_runner(project, &parsed, command, path, &mut out);
        for arg in &parsed.args {
            let rel = repo::normalize_rel_path(unquote_shell_token(arg));
            if project.files.contains_key(&rel)
                && let Ok(text) = std::fs::read_to_string(project.root.join(&rel))
            {
                collect_artifact_path_tokens(&text, &mut out);
            }
        }
    }
    if let Some(path) = path
        && let Ok(text) = std::fs::read_to_string(project.root.join(path))
    {
        collect_artifact_path_tokens(&text, &mut out);
    }
    out.into_iter().collect()
}

fn collect_artifact_paths_from_resolved_runner(
    project: &Project,
    parsed: &ParsedProofCommand,
    command: &str,
    proof_path: Option<&str>,
    out: &mut BTreeSet<String>,
) {
    if matches!(parsed.runner.as_str(), "pnpm" | "npm" | "yarn" | "bun")
        && let Some(package) = package_for_parsed_command(
            project,
            parsed,
            &ProofSurface {
                command: Some(command.to_string()),
                path: proof_path.map(str::to_string),
                target_anchor: proof_path.map(str::to_string),
                evidence: "artifact_runner_resolution".to_string(),
                strength: EvidenceStrength::Medium,
                reason: "temporary package resolution surface".to_string(),
                locations: Vec::new(),
            },
        )
        && let Some(script_name) = package_script_name_from_command(parsed)
    {
        for (name, body, _) in package_json_scripts(project, &package.manifest) {
            if name == script_name {
                collect_artifact_path_tokens(&body, out);
            }
        }
    }
    if matches!(parsed.runner.as_str(), "make" | "just")
        && let Some(target) = parsed.args.first().map(|value| unquote_shell_token(value))
        && let Some(script) = project.scripts.iter().find(|script| {
            script.name == target && script.command.starts_with(parsed.runner.as_str())
        })
        && let Some(path) = script.path.as_deref()
        && let Ok(text) = std::fs::read_to_string(project.root.join(path))
    {
        collect_artifact_path_tokens(
            &make_like_target_body(&text, script.line_start.unwrap_or(1)),
            out,
        );
    }
}

fn make_like_target_body(text: &str, target_line_start: usize) -> String {
    let mut body = String::new();
    for line in text.lines().skip(target_line_start) {
        let trimmed = line.trim();
        if !line.starts_with(char::is_whitespace) && trimmed.contains(':') {
            break;
        }
        body.push_str(line);
        body.push('\n');
    }
    body
}

fn collect_artifact_path_tokens(text: &str, out: &mut BTreeSet<String>) {
    for token in text.split(|ch: char| {
        ch.is_whitespace() || matches!(ch, '"' | '\'' | '`' | ',' | ')' | '(')
    }) {
        if token_is_artifact_path(token) {
            out.insert(repo::normalize_rel_path(token.trim_matches(['\'', '"', '`'])));
        }
    }
}

fn token_is_artifact_path(token: &str) -> bool {
    let token = token.trim_matches(['\'', '"', '`', ';']);
    (token.contains("receipt")
        || token.contains("witness")
        || token.contains("artifact")
        || token.contains("report"))
        && matches!(
            token.rsplit('.').next().unwrap_or(""),
            "json" | "jsonl" | "md" | "txt"
        )
}
