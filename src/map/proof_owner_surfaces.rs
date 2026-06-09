fn owner_surface_proof_surfaces(project: &Project, anchor: &str) -> Vec<ProofSurface> {
    let Some(file) = project.files.get(anchor) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if file.has_role("manifest") {
        out.extend(manifest_script_proof_surfaces(project, file));
        out.extend(manifest_ci_reference_proof_surfaces(project, file));
    }
    if file.has_role("schema_contract") || schema_owner_path(&file.rel) {
        out.extend(schema_script_proof_surfaces(project, file));
        out.extend(schema_ci_reference_proof_surfaces(project, file));
    }
    if file.has_role("env_config") {
        out.extend(env_consumer_proof_surfaces(project, file));
        out.extend(env_ci_reference_proof_surfaces(project, file));
    }
    if file.has_role("build_ci") {
        out.extend(ci_owner_proof_surfaces(project, file));
    }
    out
}

fn ci_owner_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    ci_run_steps(&text)
        .into_iter()
        .filter_map(|step| ci_owner_proof_surface_for_step(project, &file.rel, step))
        .collect()
}

fn manifest_script_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    if workspace_manifest_file(&file.rel) {
        return workspace_manifest_script_proof_surfaces(project, file);
    }
    if manifest_file_name(&file.rel) == "Cargo.toml" {
        return cargo_manifest_builtin_proof_surfaces(project, file);
    }
    if manifest_file_name(&file.rel) == "Package.swift" {
        return swift_manifest_builtin_proof_surfaces(project, file);
    }
    let Some(package) = project
        .packages
        .iter()
        .find(|package| package.manifest == file.rel)
    else {
        return Vec::new();
    };
    if package.ecosystem != "javascript" {
        return Vec::new();
    }
    package_json_scripts(project, &package.manifest)
        .into_iter()
        .filter(|(name, command, _)| manifest_script_is_proof_or_support_relevant(name, command))
        .map(|(name, command, line)| ProofSurface {
            command: package_script_command(project, package, &name),
            path: Some(package.manifest.clone()),
            target_anchor: Some(file.rel.clone()),
            evidence: manifest_script_evidence(&name, &command).to_string(),
            strength: EvidenceStrength::Hard,
            reason: format!("package manifest defines `{name}` script: {command}"),
            locations: vec![EvidenceLocation::line(&package.manifest, line, "package_script")],
        })
        .collect()
}

fn schema_script_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    let Some(package) = package_for_rel(project, &file.rel) else {
        return Vec::new();
    };
    if package.ecosystem != "javascript" {
        return Vec::new();
    }
    package_json_scripts(project, &package.manifest)
        .into_iter()
        .filter(|(name, command, _)| schema_script_is_proof_relevant(name, command, &file.rel))
        .map(|(name, command, line)| ProofSurface {
            command: package_script_command(project, package, &name),
            path: Some(package.manifest.clone()),
            target_anchor: Some(file.rel.clone()),
            evidence: "schema_package_script".to_string(),
            strength: EvidenceStrength::Hard,
            reason: format!("package script references schema tooling: `{name}` -> {command}"),
            locations: vec![EvidenceLocation::line(&package.manifest, line, "package_script")],
        })
        .collect()
}

fn env_consumer_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    let keys = env_declared_keys(project, &file.rel);
    if keys.is_empty() {
        return Vec::new();
    }
    let key_set = keys
        .iter()
        .map(|(key, _)| key.as_str())
        .collect::<BTreeSet<_>>();
    let mut out = Vec::new();
    for candidate in project.files.values() {
        if candidate.rel == file.rel
            || candidate.has_role("generated")
            || candidate.has_role("fixture")
            || candidate.has_role("archive")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&candidate.rel)) else {
            continue;
        };
        for (line_number, line) in text.lines().enumerate() {
            if !line_may_contain_static_env_reference(line) {
                continue;
            }
            let mut names = static_env_names(line);
            names.extend(prisma_env_names(line));
            names.sort();
            names.dedup();
            for name in names {
                if !key_set.contains(name.as_str()) {
                    continue;
                }
                out.push(ProofSurface {
                    command: None,
                    path: Some(candidate.rel.clone()),
                    target_anchor: Some(file.rel.clone()),
                    evidence: "env_consumer_reference".to_string(),
                    strength: EvidenceStrength::High,
                    reason: format!("source reads env key `{name}` declared in {}", file.rel),
                    locations: vec![EvidenceLocation::line(
                        &candidate.rel,
                        line_number + 1,
                        "env_reference",
                    )],
                });
            }
        }
    }
    unique_proof_surfaces(out)
}

fn manifest_ci_reference_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    if workspace_manifest_file(&file.rel) {
        return workspace_manifest_ci_reference_proof_surfaces(project, file);
    }
    let Some(package) = project
        .packages
        .iter()
        .find(|package| package.manifest == file.rel)
    else {
        return Vec::new();
    };
    let scripts = package_json_scripts(project, &package.manifest);
    ci_run_reference_proof_surfaces(project, file, "manifest_ci_reference", |command| {
        manifest_ci_run_match_reason(package, &scripts, command)
    })
}

fn schema_ci_reference_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    let owner_package = package_for_rel(project, &file.rel);
    ci_run_reference_proof_surfaces(project, file, "schema_ci_reference", |command| {
        schema_ci_run_match_reason(owner_package, &file.rel, command)
    })
}

fn env_ci_reference_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    let keys = env_declared_keys(project, &file.rel)
        .into_iter()
        .map(|(key, _)| key)
        .collect::<BTreeSet<_>>();
    if keys.is_empty() {
        return Vec::new();
    }
    ci_line_reference_proof_surfaces(project, file, "env_ci_reference", |line| {
        keys.iter()
            .find(|key| line.contains(key.as_str()))
            .map(|key| format!("CI line references env key `{key}`"))
    })
}

fn ci_run_reference_proof_surfaces<F>(
    project: &Project,
    file: &FileInfo,
    evidence: &str,
    matcher: F,
) -> Vec<ProofSurface>
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = Vec::new();
    for ci in project
        .files
        .values()
        .filter(|candidate| candidate.has_role("build_ci"))
    {
        let Ok(text) = std::fs::read_to_string(project.root.join(&ci.rel)) else {
            continue;
        };
        for step in ci_run_steps(&text) {
            let Some(reason) = matcher(&step.command) else {
                continue;
            };
            out.push(ProofSurface {
                command: Some(step.command.clone()),
                path: Some(ci.rel.clone()),
                target_anchor: Some(file.rel.clone()),
                evidence: evidence.to_string(),
                strength: EvidenceStrength::High,
                reason: format!("{reason} for {}", file.rel),
                locations: vec![EvidenceLocation::line(&ci.rel, step.line, "ci_step")],
            });
        }
    }
    unique_proof_surfaces(out)
}

fn ci_line_reference_proof_surfaces<F>(
    project: &Project,
    file: &FileInfo,
    evidence: &str,
    matcher: F,
) -> Vec<ProofSurface>
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = Vec::new();
    for ci in project
        .files
        .values()
        .filter(|candidate| candidate.has_role("build_ci"))
    {
        let Ok(text) = std::fs::read_to_string(project.root.join(&ci.rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let line_without_comments = strip_inline_shell_comment(line);
            if line_without_comments.trim_start().starts_with('#') {
                continue;
            }
            let Some(reason) = matcher(&line_without_comments) else {
                continue;
            };
            out.push(ProofSurface {
                command: ci_inline_run_command(line),
                path: Some(ci.rel.clone()),
                target_anchor: Some(file.rel.clone()),
                evidence: evidence.to_string(),
                strength: EvidenceStrength::High,
                reason: format!("{reason} for {}", file.rel),
                locations: vec![EvidenceLocation::line(&ci.rel, index + 1, "ci_step")],
            });
        }
    }
    unique_proof_surfaces(out)
}

fn package_json_scripts(project: &Project, manifest: &str) -> Vec<(String, String, usize)> {
    let Ok(text) = std::fs::read_to_string(project.root.join(manifest)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(scripts) = value.get("scripts").and_then(|scripts| scripts.as_object()) else {
        return Vec::new();
    };
    let mut out = scripts
        .iter()
        .filter_map(|(name, value)| {
            let command = value.as_str()?.trim();
            if command.is_empty() {
                return None;
            }
            Some((
                name.clone(),
                command.to_string(),
                json_key_line(&text, name).unwrap_or(1),
            ))
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn manifest_script_is_proof_relevant(name: &str, command: &str) -> bool {
    manifest_script_command_body_is_run_safe(command)
        && (manifest_script_name_is_proof_relevant(name)
            || manifest_script_command_is_proof_relevant(command))
}

fn manifest_script_is_proof_or_support_relevant(name: &str, command: &str) -> bool {
    manifest_script_name_is_proof_relevant(name) || manifest_script_command_is_proof_relevant(command)
}

fn manifest_script_evidence(name: &str, command: &str) -> &'static str {
    if manifest_script_is_proof_relevant(name, command) {
        "manifest_script"
    } else {
        "manifest_script_setup"
    }
}

fn manifest_script_name_is_proof_relevant(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "test" | "tests" | "lint" | "typecheck" | "type-check" | "type_check" | "check"
            | "build" | "verify" | "e2e"
    ) {
        return true;
    }
    lower
        .split([':', '-', '_', '.', ' '])
        .filter(|part| !part.is_empty())
        .any(|part| {
            matches!(
                part,
                "test" | "tests" | "lint" | "typecheck" | "check" | "build" | "verify" | "e2e"
            )
        })
}

fn manifest_script_command_is_proof_relevant(command: &str) -> bool {
    let tokens = command_tokens(command);
    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "vitest"
                | "jest"
                | "mocha"
                | "uvu"
                | "playwright"
                | "cypress"
                | "eslint"
                | "biome"
                | "tsc"
                | "svelte-check"
        )
    }) {
        return true;
    }
    tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "test" | "lint" | "typecheck" | "check" | "build" | "verify" | "e2e"
        )
    })
}

fn manifest_script_command_body_is_run_safe(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    if crate::proof_classification::proof_text_is_readonly_migration_status(&lower) {
        return true;
    }
    ![
        "--watch",
        " watch",
        ":watch",
        " dev",
        " start",
        " serve",
        " preview",
        " install",
        " codegen",
        " generate",
        " seed",
        " studio",
        " deploy",
        " release",
        " publish",
        " migrate",
        " db push",
        " db:push",
        " db:normalize",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn schema_script_is_proof_relevant(name: &str, command: &str, rel: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let command = command.to_ascii_lowercase();
    let hay = format!("{name} {command}");
    let schema_name = Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(rel)
        .to_ascii_lowercase();
    [
        "prisma",
        "migrate",
        "migration",
        "db:",
        "db-",
        "generate",
        "seed",
    ]
    .iter()
    .any(|needle| hay.contains(needle))
        || (!schema_name.is_empty() && command.contains(&schema_name))
}

fn package_script_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    script_name: &str,
) -> Option<String> {
    match package.ecosystem.as_str() {
        "javascript" => {
            let runner = javascript_runner_for_package(project, package);
            let command = javascript_script_command(&runner, script_name);
            Some(if package.path == "." {
                command
            } else {
                format!("cd {} && {command}", shell_quote(&package.path))
            })
        }
        _ => None,
    }
}

fn javascript_script_command(runner: &str, script_name: &str) -> String {
    if script_name == "test" {
        return javascript_test_command(runner);
    }
    match runner {
        "npm" => format!("npm run {}", shell_quote(script_name)),
        "yarn" => format!("yarn {}", shell_quote(script_name)),
        "bun" => format!("bun run {}", shell_quote(script_name)),
        _ => format!("pnpm run {}", shell_quote(script_name)),
    }
}

fn json_key_line(text: &str, key: &str) -> Option<usize> {
    let quoted = format!("\"{key}\"");
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(&quoted))
        .map(|(index, _)| index + 1)
}

fn first_line_containing(project: &Project, rel: &str, needles: &[&str]) -> Option<usize> {
    let text = std::fs::read_to_string(project.root.join(rel)).ok()?;
    text.lines()
        .enumerate()
        .find(|(_, line)| needles.iter().any(|needle| line.contains(needle)))
        .map(|(index, _)| index + 1)
}

fn schema_owner_path(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with(".prisma")
        || lower.ends_with(".graphql")
        || lower.ends_with(".gql")
        || lower.ends_with(".proto")
        || lower.ends_with(".avsc")
        || lower.ends_with(".sql")
        || lower.starts_with("migrations/")
        || lower.contains("/migrations/")
}

fn env_declared_keys(project: &Project, rel: &str) -> Vec<(String, usize)> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, _)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim().trim_start_matches("export ").trim();
        if !name.is_empty()
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            out.push((name.to_string(), index + 1));
        }
    }
    out
}
