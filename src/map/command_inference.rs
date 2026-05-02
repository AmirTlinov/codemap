fn impacted_domains<'a>(project: &'a Project, files: &[String]) -> Vec<&'a Domain> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for file in files {
        if let Some(domain) = domain_by_rel(project, file)
            && seen.insert(domain.id.clone())
        {
            out.push(domain);
        }
    }
    out
}

fn infer_minimal_commands(
    project: &Project,
    domains: &[&Domain],
    files: &[String],
    changed: &[String],
) -> Vec<String> {
    let root_test = find_script(project, &["test"]);
    let role_aware = role_aware_minimal_commands(project, files, changed);
    if !role_aware.is_empty() {
        return role_aware;
    }
    let changed_source_package = single_source_package_for_files(project, changed);
    let changed_domains = impacted_domains(project, changed);
    if let Some(package) = changed_source_package
        && let Some(command) =
            package_minimal_command(project, package, &changed_domains, root_test.as_deref())
    {
        return vec![command];
    }
    if changed_source_package.is_some()
        && let Some(package) = single_package_for_files(project, files)
        && let Some(command) =
            package_minimal_command(project, package, domains, root_test.as_deref())
    {
        return vec![command];
    }
    if let Some(test) = root_test {
        if (changed.is_empty() || changed_source_package.is_some())
            && domains.len() == 1
            && domains[0].path != "."
            && project.package_manager != "bun"
        {
            return vec![format!("{test} {}", domains[0].path)];
        }
        return vec![test];
    }
    match project.package_manager.as_str() {
        "cargo" => vec!["cargo test".to_string()],
        "go" => vec!["go test ./...".to_string()],
        "python" => vec!["pytest".to_string()],
        _ => vec!["run the nearest domain tests for the changed files".to_string()],
    }
}

fn single_source_package_for_files<'a>(
    project: &'a Project,
    files: &[String],
) -> Option<&'a crate::model::PackageInfo> {
    if files.is_empty() {
        return None;
    }
    let package = single_package_for_files(project, files)?;
    files
        .iter()
        .all(|file| {
            project
                .files
                .get(file)
                .map(|info| is_package_implementation_source(file, info, package))
                .unwrap_or(false)
        })
        .then_some(package)
}

fn is_package_implementation_source(
    rel: &str,
    info: &crate::model::FileInfo,
    package: &crate::model::PackageInfo,
) -> bool {
    if rel == package.manifest || !repo::is_source_ext(&info.ext) {
        return false;
    }
    if [
        "generated",
        "build_ci",
        "semantic_anchor",
        "agent_bootstrap",
    ]
    .iter()
    .any(|role| info.roles.contains(*role))
    {
        return false;
    }
    let name = std::path::Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    !is_tooling_config_source_name(&name)
}

fn is_tooling_config_source_name(name: &str) -> bool {
    name.contains(".config.")
        || name.ends_with(".config")
        || name.contains(".conf.")
        || name.ends_with(".conf")
        || name.starts_with(".eslintrc.")
        || name.starts_with(".prettierrc.")
        || name.starts_with(".babelrc.")
        || matches!(
            name,
            "gulpfile.js"
                | "gulpfile.ts"
                | "gruntfile.js"
                | "gruntfile.ts"
                | "karma.conf.js"
                | "karma.conf.ts"
        )
}

fn single_package_for_files<'a>(
    project: &'a Project,
    files: &[String],
) -> Option<&'a crate::model::PackageInfo> {
    let mut selected: Option<&crate::model::PackageInfo> = None;
    for file in files {
        let package = package_for_rel(project, file)?;
        match selected {
            Some(current) if current.path != package.path => return None,
            Some(_) => {}
            None => selected = Some(package),
        }
    }
    selected
}

fn package_minimal_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    domains: &[&Domain],
    root_test: Option<&str>,
) -> Option<String> {
    match package.ecosystem.as_str() {
        "javascript" => javascript_package_test_command(project, package, domains, root_test),
        "rust" => Some(if package.path == "." {
            "cargo test".to_string()
        } else if root_cargo_workspace_includes(project, &package.path) {
            format!("cargo test -p {}", shell_quote(&package.name))
        } else {
            format!("cd {} && cargo test", shell_quote(&package.path))
        }),
        "go" => Some(if package.path == "." {
            "go test ./...".to_string()
        } else {
            format!("cd {} && go test ./...", shell_quote(&package.path))
        }),
        "python" => Some(if package.path == "." {
            "pytest".to_string()
        } else {
            format!("cd {} && pytest", shell_quote(&package.path))
        }),
        "swift" => Some(if package.path == "." {
            "swift test".to_string()
        } else {
            format!("cd {} && swift test", shell_quote(&package.path))
        }),
        _ => None,
    }
}

fn root_cargo_workspace_includes(project: &Project, package_path: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(project.root.join("Cargo.toml")) else {
        return false;
    };
    cargo_workspace_values(&text, "members")
        .into_iter()
        .any(|pattern| cargo_workspace_pattern_matches(&pattern, package_path))
        && !cargo_workspace_values(&text, "exclude")
            .into_iter()
            .any(|pattern| cargo_workspace_pattern_matches(&pattern, package_path))
}

fn cargo_workspace_values(text: &str, wanted_key: &str) -> Vec<String> {
    toml::from_str::<toml::Value>(text)
        .ok()
        .and_then(|value| value.get("workspace").cloned())
        .and_then(|workspace| workspace.get(wanted_key).cloned())
        .and_then(|value| toml_string_array(&value))
        .unwrap_or_default()
}

fn cargo_workspace_pattern_matches(pattern: &str, package_path: &str) -> bool {
    let pattern = repo::normalize_rel_path(pattern.trim().trim_start_matches("./"));
    !pattern.is_empty() && (pattern == package_path || glob_match(&pattern, package_path))
}

fn toml_string_array(value: &toml::Value) -> Option<Vec<String>> {
    Some(
        value
            .as_array()?
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

fn javascript_package_test_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    domains: &[&Domain],
    root_test: Option<&str>,
) -> Option<String> {
    if !javascript_package_has_script(project, package, "test") {
        return None;
    }
    if is_javascript_package_manager(&project.package_manager)
        && let Some(test) = root_test
        && domains.len() == 1
        && domains[0].path == package.path
        && package.path != "."
        && project.package_manager != "bun"
    {
        return Some(format!("{test} {}", package.path));
    }
    let runner = javascript_runner_for_package(project, package);
    let command = javascript_test_command(&runner);
    Some(if package.path == "." {
        command
    } else {
        format!("cd {} && {command}", shell_quote(&package.path))
    })
}

fn javascript_package_has_script(
    project: &Project,
    package: &crate::model::PackageInfo,
    script: &str,
) -> bool {
    let Ok(text) = std::fs::read_to_string(project.root.join(&package.manifest)) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    value
        .get("scripts")
        .and_then(|scripts| scripts.get(script))
        .and_then(|value| value.as_str())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn javascript_runner_for_package(project: &Project, package: &crate::model::PackageInfo) -> String {
    for rel in ancestor_paths(&package.path) {
        let dir = if rel == "." {
            project.root.clone()
        } else {
            project.root.join(&rel)
        };
        if dir.join("pnpm-workspace.yaml").exists() || dir.join("pnpm-lock.yaml").exists() {
            return "pnpm".to_string();
        }
        if dir.join("yarn.lock").exists() {
            return "yarn".to_string();
        }
        if dir.join("bun.lockb").exists() {
            return "bun".to_string();
        }
        if dir.join("package-lock.json").exists() {
            return "npm".to_string();
        }
    }
    if is_javascript_package_manager(&project.package_manager) {
        project.package_manager.clone()
    } else {
        "npm".to_string()
    }
}

fn ancestor_paths(rel: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = repo::normalize_rel_path(rel);
    loop {
        out.push(if current.is_empty() {
            ".".to_string()
        } else {
            current.clone()
        });
        if current.is_empty() || current == "." {
            break;
        }
        let parent = Path::new(&current)
            .parent()
            .map(|path| repo::normalize_rel_path(&path.to_string_lossy()))
            .unwrap_or_else(|| ".".to_string());
        if parent == current {
            break;
        }
        current = parent;
    }
    if !out.iter().any(|path| path == ".") {
        out.push(".".to_string());
    }
    out
}

fn is_javascript_package_manager(value: &str) -> bool {
    matches!(value, "npm" | "pnpm" | "yarn" | "bun")
}

fn javascript_test_command(runner: &str) -> String {
    match runner {
        "yarn" => "yarn test".to_string(),
        "bun" => "bun test".to_string(),
        "pnpm" => "pnpm test".to_string(),
        _ => "npm test".to_string(),
    }
}

fn find_script(project: &Project, names: &[&str]) -> Option<String> {
    project
        .scripts
        .iter()
        .filter_map(|script| {
            script_match_rank(script, names).map(|rank| (rank, script.command.clone()))
        })
        .min_by(|(left_rank, left_command), (right_rank, right_command)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| left_command.cmp(right_command))
        })
        .map(|(_, command)| command)
}

fn script_match_rank(script: &crate::model::ScriptInfo, names: &[&str]) -> Option<usize> {
    let script_name = script.name.to_ascii_lowercase();
    let script_command = script.command.to_ascii_lowercase();
    let wanted: Vec<String> = names.iter().map(|name| name.to_ascii_lowercase()).collect();

    for (index, name) in wanted.iter().enumerate() {
        if script_name == name.as_str() {
            return Some(index);
        }
    }
    for (index, name) in wanted.iter().enumerate() {
        if script_name.contains(name) {
            return Some(10 + index);
        }
    }
    for (index, name) in wanted.iter().enumerate() {
        if script_command.contains(name) {
            return Some(20 + index);
        }
    }
    None
}
