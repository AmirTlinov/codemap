fn proof_command_for_test(project: &Project, test: &str) -> Option<String> {
    let Some(package) = package_for_rel(project, test) else {
        return project.files.get(test).and_then(|file| {
            (file.language == "python").then(|| format!("pytest {}", shell_quote(test)))
        });
    };
    match package.ecosystem.as_str() {
        "javascript" => javascript_test_file_command(project, package, test),
        "python" => Some(if package.path == "." {
            format!("pytest {}", shell_quote(test))
        } else {
            format!(
                "cd {} && pytest {}",
                shell_quote(&package.path),
                shell_quote(&strip_package_prefix(test, &package.path))
            )
        }),
        "swift" => Some(if package.path == "." {
            "swift test".to_string()
        } else {
            format!("cd {} && swift test", shell_quote(&package.path))
        }),
        "rust" => package_minimal_command(
            project,
            package,
            &[domain_for_path(project, test)],
            find_script(project, &["test"]).as_deref(),
        ),
        "go" => package_minimal_command(
            project,
            package,
            &[domain_for_path(project, test)],
            find_script(project, &["test"]).as_deref(),
        ),
        _ => package_minimal_command(
            project,
            package,
            &[domain_for_path(project, test)],
            find_script(project, &["test"]).as_deref(),
        ),
    }
}

fn javascript_test_file_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    test: &str,
) -> Option<String> {
    let runner = javascript_runner_for_package(project, package);
    let test_arg = shell_quote(&strip_package_prefix(test, &package.path));
    if project
        .files
        .get(test)
        .map(|file| file.has_role("e2e_test"))
        .unwrap_or(false)
        && let Some(command) =
            javascript_e2e_test_file_command(project, package, &runner, &test_arg)
    {
        return Some(if package.path == "." {
            command
        } else {
            format!("cd {} && {command}", shell_quote(&package.path))
        });
    }
    if !javascript_package_has_script(project, package, "test") {
        return None;
    }
    let command = javascript_package_script(project, package, "test")
        .and_then(|script| javascript_test_file_command_for_script(&runner, &script, &test_arg))
        .unwrap_or_else(|| javascript_test_file_command_for_runner(&runner, &test_arg));
    Some(if package.path == "." {
        command
    } else {
        format!("cd {} && {command}", shell_quote(&package.path))
    })
}

fn javascript_e2e_test_file_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    runner: &str,
    test_arg: &str,
) -> Option<String> {
    let candidates = [
        "test:e2e",
        "e2e",
        "playwright",
        "test:playwright",
        "test:e2e:ui",
        "test:e2e:ui-rails",
    ];
    if let Some((name, _)) = javascript_package_script_by_names(project, package, &candidates) {
        return Some(javascript_package_script_invocation(
            runner, &name, test_arg,
        ));
    }
    javascript_package_script_matching(project, package, |name, command| {
        let name = name.to_ascii_lowercase();
        let command = command.to_ascii_lowercase();
        (name.contains("e2e") || name.contains("playwright")) && command.contains("playwright")
    })
    .map(|(name, _)| javascript_package_script_invocation(runner, &name, test_arg))
}

fn javascript_package_script_by_names(
    project: &Project,
    package: &crate::model::PackageInfo,
    names: &[&str],
) -> Option<(String, String)> {
    for name in names {
        if let Some(command) = javascript_package_script(project, package, name) {
            return Some(((*name).to_string(), command));
        }
    }
    None
}

fn javascript_package_script_matching<F>(
    project: &Project,
    package: &crate::model::PackageInfo,
    predicate: F,
) -> Option<(String, String)>
where
    F: Fn(&str, &str) -> bool,
{
    let text = std::fs::read_to_string(project.root.join(&package.manifest)).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    let scripts = value
        .get("scripts")
        .and_then(|scripts| scripts.as_object())?;
    scripts.iter().find_map(|(name, value)| {
        let command = value.as_str()?.trim();
        (!command.is_empty() && predicate(name, command))
            .then(|| (name.to_string(), command.to_string()))
    })
}

fn javascript_package_script_invocation(runner: &str, script_name: &str, test_arg: &str) -> String {
    if script_name == "test" {
        return javascript_test_file_command_for_runner(runner, test_arg);
    }
    match runner {
        "npm" => format!("npm run {} -- {test_arg}", shell_quote(script_name)),
        "yarn" => format!("yarn {} {test_arg}", shell_quote(script_name)),
        "bun" => format!("bun run {} {test_arg}", shell_quote(script_name)),
        _ => format!("pnpm run {} -- {test_arg}", shell_quote(script_name)),
    }
}

fn javascript_test_file_command_for_script(
    runner: &str,
    script: &str,
    test_arg: &str,
) -> Option<String> {
    let script = script.trim();
    if script.is_empty() || !is_simple_javascript_test_script(script) {
        return None;
    }
    let known = [
        "vitest",
        "jest",
        "uvu",
        "ava",
        "mocha",
        "playwright test",
        "node --test",
        "tsx",
    ];
    if !known
        .iter()
        .any(|prefix| script_starts_with(script, prefix))
    {
        return None;
    }
    Some(javascript_exec_command(runner, script, test_arg))
}

fn is_simple_javascript_test_script(script: &str) -> bool {
    !["&&", "||", ";", "|", "\n"]
        .iter()
        .any(|marker| script.contains(marker))
}

fn script_starts_with(script: &str, prefix: &str) -> bool {
    script == prefix
        || script
            .strip_prefix(prefix)
            .map(|rest| rest.starts_with(char::is_whitespace))
            .unwrap_or(false)
}

fn javascript_exec_command(runner: &str, script: &str, test_arg: &str) -> String {
    if script_starts_with(script, "npm")
        || script_starts_with(script, "pnpm")
        || script_starts_with(script, "yarn")
        || script_starts_with(script, "bun")
    {
        return format!("{script} {test_arg}");
    }
    match runner {
        "pnpm" => format!("pnpm exec {script} {test_arg}"),
        "yarn" => format!("yarn {script} {test_arg}"),
        "bun" => format!("bunx {script} {test_arg}"),
        _ => format!("npx {script} {test_arg}"),
    }
}

fn javascript_test_file_command_for_runner(runner: &str, test_arg: &str) -> String {
    match runner {
        "npm" => format!("npm test -- {test_arg}"),
        "yarn" => format!("yarn test {test_arg}"),
        "bun" => format!("bun test {test_arg}"),
        _ => format!("pnpm test {test_arg}"),
    }
}

fn javascript_package_script(
    project: &Project,
    package: &crate::model::PackageInfo,
    script: &str,
) -> Option<String> {
    let text = std::fs::read_to_string(project.root.join(&package.manifest)).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    value
        .get("scripts")
        .and_then(|scripts| scripts.get(script))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn strip_package_prefix(rel: &str, package_path: &str) -> String {
    let prefix = package_path.trim_end_matches('/');
    if prefix == "." {
        return rel.to_string();
    }
    rel.strip_prefix(&format!("{prefix}/"))
        .unwrap_or(rel)
        .to_string()
}

fn proof_fallback_commands(
    project: &Project,
    anchors: &[String],
    changed: &[String],
    proofs: &[ProofSurface],
) -> Vec<String> {
    if anchors.is_empty() && changed.is_empty() {
        return Vec::new();
    }
    if proofs.iter().any(proof_surface_command_closes_fallback) {
        return Vec::new();
    }
    let proof_commands = proofs
        .iter()
        .filter_map(|proof| proof.command.as_ref())
        .cloned()
        .collect::<Vec<_>>();
    let all_files = if anchors.is_empty() {
        changed.to_vec()
    } else {
        anchors
            .iter()
            .map(|anchor| anchor_file_rel(anchor))
            .collect()
    };
    if all_files
        .iter()
        .all(|file| proof_fallback_target_is_support_artifact(project, file))
    {
        return Vec::new();
    }
    let impacted = if changed.is_empty() {
        Vec::new()
    } else {
        let impact = impact_report(project, changed.to_vec(), "--changed".to_string(), 1, 30);
        impact
            .clusters
            .iter()
            .flat_map(|cluster| {
                cluster
                    .direct_consumers
                    .iter()
                    .map(|edge| edge.from.clone())
                    .chain(
                        cluster
                            .contract_links
                            .iter()
                            .filter(|edge| edge.from != edge.to)
                            .map(|edge| edge.from.clone()),
                    )
            })
            .collect::<Vec<_>>()
    };
    let plan = verification_plan(project, &all_files, &impacted);
    unique(plan.minimal)
        .into_iter()
        .filter(|command| !proof_commands.iter().any(|existing| existing == command))
        .take(3)
        .collect()
}

fn anchor_file_rel(anchor: &str) -> String {
    split_symbol_anchor(anchor)
        .map(|(file_rel, _)| file_rel)
        .unwrap_or_else(|| anchor.to_string())
}

fn proof_fallback_target_is_support_artifact(project: &Project, rel: &str) -> bool {
    if is_support_artifact_path(rel) {
        return true;
    }
    project.files.get(rel).is_some_and(|file| {
        [
            "receipt",
            "witness",
            "fixture",
            "generated",
            "archive",
            "build_output",
        ]
        .iter()
        .any(|role| file.has_role(role))
    })
}
