#[derive(Debug)]
struct ParsedProofCommand {
    cwd: Option<String>,
    runner: String,
    args: Vec<String>,
}

fn parse_static_command(command: &str) -> Option<ParsedProofCommand> {
    let mut tokens = shell_words(command);
    let mut cwd = None;
    if tokens.len() >= 4 && tokens.first().is_some_and(|token| token == "cd") {
        cwd = tokens.get(1).cloned();
        if tokens.get(2).is_none_or(|token| token != "&&") {
            return None;
        }
        tokens.drain(0..3);
    }
    while tokens.first().is_some_and(|token| token.contains('=') && !token.starts_with('-')) {
        tokens.remove(0);
    }
    let runner = tokens.first()?.to_ascii_lowercase();
    Some(ParsedProofCommand {
        cwd,
        runner,
        args: tokens.into_iter().skip(1).collect(),
    })
}

fn shell_words(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for ch in command.chars() {
        if matches!(quote, Some(q) if q == ch) {
            quote = None;
            continue;
        }
        if quote.is_none() && matches!(ch, '\'' | '"') {
            quote = Some(ch);
            continue;
        }
        if quote.is_none() && ch.is_whitespace() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn unquote_shell_token(token: &str) -> &str {
    token
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .or_else(|| token.strip_prefix('"').and_then(|value| value.strip_suffix('"')))
        .unwrap_or(token)
}

fn package_for_parsed_command<'a>(
    project: &'a Project,
    parsed: &ParsedProofCommand,
    proof: &ProofSurface,
) -> Option<&'a crate::model::PackageInfo> {
    if let Some(filter) = package_filter_arg(&parsed.args) {
        return project
            .packages
            .iter()
            .find(|package| package.name == filter || package.name.ends_with(&format!("/{filter}")));
    }
    if let Some(cwd) = parsed.cwd.as_deref() {
        let cwd = repo::normalize_rel_path(cwd);
        return project
            .packages
            .iter()
            .find(|package| package.path == cwd || package.manifest.starts_with(&format!("{cwd}/")));
    }
    proof
        .path
        .as_deref()
        .and_then(|path| package_for_rel(project, path))
        .or_else(|| project.packages.iter().find(|package| package.path == "."))
}

fn package_filter_arg(args: &[String]) -> Option<String> {
    for pair in args.windows(2) {
        if pair[0] == "--filter" {
            return Some(pair[1].trim_start_matches('@').to_string());
        }
    }
    args.iter()
        .find_map(|arg| arg.strip_prefix("--filter=").map(|value| value.trim_start_matches('@').to_string()))
}

fn package_script_name_from_command(parsed: &ParsedProofCommand) -> Option<String> {
    match parsed.runner.as_str() {
        "npm" | "pnpm" | "bun" => {
            let args = &parsed.args;
            for (index, arg) in args.iter().enumerate() {
                if arg == "run" {
                    return args.get(index + 1).cloned();
                }
            }
            if args.first().is_some_and(|arg| arg == "test") {
                return Some("test".to_string());
            }
            None
        }
        "yarn" => parsed.args.first().cloned(),
        _ => None,
    }
}

fn proof_wiring_needs_artifact_chain(project: &Project, proof: &ProofSurface) -> bool {
    let Some(path) = proof.path.as_deref() else {
        return proof.evidence.contains("receipt") || proof.evidence.contains("witness");
    };
    project.files.get(path).is_some_and(|file| {
        file.has_role("receipt")
            || file.has_role("witness")
            || file.has_role("owner_doc")
            || proof.evidence.contains("receipt")
            || proof.evidence.contains("witness")
    })
}

fn command_mentions_artifact(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("receipt")
        || lower.contains("witness")
        || lower.contains("artifact")
        || lower.contains(".json")
        || lower.contains(".jsonl")
}

fn artifact_consumers(project: &Project, artifact: &str) -> Vec<(String, usize)> {
    let mut consumers = Vec::new();
    let basename = artifact.rsplit('/').next().unwrap_or(artifact);
    for file in project.files.values() {
        if file.rel == artifact || !proof_wiring_file_can_consume_artifact(file) {
            continue;
        }
        if !proof_wiring_file_can_consume_evidence(file) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if line.contains(artifact) || (!basename.is_empty() && line.contains(basename)) {
                consumers.push((file.rel.clone(), index + 1));
                break;
            }
        }
    }
    consumers
}

fn field_consumers(project: &Project, owner: &str, field: &str) -> Vec<(String, usize)> {
    let needle = format!("\"{field}\"");
    let mut consumers = Vec::new();
    for file in project.files.values() {
        if file.rel == owner || !proof_wiring_file_can_consume_artifact(file) {
            continue;
        }
        if !proof_wiring_file_can_consume_evidence(file) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if line.contains(&needle) || line.contains(field) {
                consumers.push((file.rel.clone(), index + 1));
                break;
            }
        }
    }
    consumers
}

fn proof_wiring_file_can_consume_artifact(file: &FileInfo) -> bool {
    if file.language != "markdown" {
        return true;
    }
    let rel = file.rel.to_ascii_lowercase();
    rel.contains("review")
        || rel.contains("report")
        || rel.contains("receipt")
        || rel.contains("witness")
}

fn proof_wiring_file_can_consume_evidence(file: &FileInfo) -> bool {
    if file.has_role("proof_runner")
        || file.has_role("doctor")
        || file.has_role("test")
        || file.has_role("schema")
        || file.has_role("schema_contract")
        || file.has_role("receipt")
        || file.has_role("witness")
    {
        return true;
    }
    let rel = file.rel.to_ascii_lowercase();
    let name = rel.rsplit('/').next().unwrap_or(&rel);
    rel.starts_with("reviews/")
        || rel.contains("/reviews/")
        || rel.starts_with("reports/")
        || rel.contains("/reports/")
        || rel.starts_with("validators/")
        || rel.contains("/validators/")
        || rel.starts_with("checks/")
        || rel.contains("/checks/")
        || name.contains("review")
        || name.contains("report")
        || name.contains("predicate")
        || name.contains("validator")
        || name.contains("validate")
        || name.contains("doctor")
        || name.contains("check")
        || name.contains("assert")
}

fn declared_receipt_fields(rel: &str, text: &str) -> Vec<(String, usize)> {
    if rel.ends_with(".json")
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(text)
        && let Some(object) = value.as_object()
    {
        return object
            .keys()
            .filter(|key| receipt_field_should_track(key))
            .map(|key| (key.clone(), json_key_line(text, key).unwrap_or(1)))
            .collect();
    }
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            let (key, _) = trimmed.split_once(':')?;
            let key = key.trim().trim_matches(['"', '\'', '`', '-']).to_string();
            receipt_field_should_track(&key).then_some((key, index + 1))
        })
        .collect()
}

fn json_string_field(text: &str, key: &str) -> Option<String> {
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn field_line_locations(text: &str, rel: &str, keys: &[&str]) -> Vec<EvidenceLocation> {
    let mut locations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if keys
            .iter()
            .any(|key| line.contains(&format!("\"{key}\"")) || line.contains(&format!("{key}:")))
        {
            locations.push(EvidenceLocation::line(rel, index + 1, "field"));
        }
    }
    if locations.is_empty() {
        locations.push(EvidenceLocation::path(rel, "artifact"));
    }
    locations
}

fn receipt_field_should_track(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "status",
        "pass",
        "passed",
        "success",
        "ok",
        "exit_code",
        "exit",
        "schema",
        "schema_version",
        "evidence",
        "controls",
        "control",
        "command",
        "artifact",
        "receipt",
    ]
    .iter()
    .any(|part| lower == *part || lower.contains(part))
}

fn markdown_declared_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    for part in line.split('`').skip(1).step_by(2) {
        if receipt_field_should_track(part) || field_name_looks_structural(part) {
            fields.push(part.to_string());
        }
    }
    let trimmed = line.trim().trim_start_matches('-').trim();
    if let Some((left, _)) = trimmed.split_once(':') {
        let key = left.trim().trim_matches(['`', '"', '\'']);
        if receipt_field_should_track(key) || field_name_looks_structural(key) {
            fields.push(key.to_string());
        }
    }
    fields.sort();
    fields.dedup();
    fields
}

fn field_name_looks_structural(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.chars().any(|ch| matches!(ch, '_' | '-'))
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn receipt_field_is_execution(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    matches!(lower.as_str(), "exit" | "exit_code" | "exit_status")
}

fn receipt_field_is_schema(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    matches!(lower.as_str(), "schema" | "schema_version" | "receipt_version")
}

fn file_has_predicate_language(project: &Project, rel: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    [
        "pass",
        "passed",
        "fail",
        "success",
        "exit_code",
        "assert",
        "expect",
        "validate",
        "predicate",
        "control",
        "required",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn proof_wiring_fact(
    state: (&str, &str),
    subject: impl Into<String>,
    path: Option<String>,
    evidence: impl Into<String>,
    effect: impl Into<String>,
    locations: Vec<EvidenceLocation>,
    expand: Option<String>,
) -> ProofWiringFact {
    let (stage, status) = state;
    ProofWiringFact {
        stage: stage.to_string(),
        status: status.to_string(),
        subject: subject.into(),
        path,
        evidence: evidence.into(),
        effect: effect.into(),
        locations,
        expand,
    }
}

fn proof_wiring_expand_for_proof(selector: &str, proof: &ProofSurface) -> Option<String> {
    proof
        .path
        .as_ref()
        .map(|path| format!("codemap cone {} --depth 2", shell_quote(path)))
        .or_else(|| Some(format!("codemap proof {selector} --section proof")))
}

fn proof_wiring_unknown_kind(fact: &ProofWiringFact) -> &str {
    match fact.stage.as_str() {
        "declared_command" => "proof_command_missing",
        "runner" => "proof_runner_unresolved",
        "artifact" => "artifact_write_not_found",
        "evidence_consumption" => "consumer_not_found",
        "contract_field" => "predicate_not_found",
        _ => "proof_wiring_unknown",
    }
}

fn proof_wiring_status_rank(status: &str) -> usize {
    match status {
        "missing" => 0,
        "unknown" => 1,
        "validated" => 2,
        "load_bearing" => 3,
        "executed" => 4,
        "wired" => 5,
        "soft" => 6,
        _ => 7,
    }
}
