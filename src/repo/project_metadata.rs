fn detect_package_manager(root: &Path) -> String {
    if root.join("pnpm-lock.yaml").exists() || root.join("pnpm-workspace.yaml").exists() {
        "pnpm"
    } else if root.join("yarn.lock").exists() {
        "yarn"
    } else if root.join("bun.lockb").exists() {
        "bun"
    } else if root.join("package.json").exists() {
        "npm"
    } else if root.join("Cargo.toml").exists() {
        "cargo"
    } else if root.join("go.mod").exists() || root.join("go.work").exists() {
        "go"
    } else if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        "python"
    } else if root.join("Package.swift").exists() {
        "swift"
    } else {
        "unknown"
    }
    .to_string()
}

fn detect_scripts(root: &Path) -> Vec<ScriptInfo> {
    let mut scripts = Vec::new();
    if root.join("package.json").exists() {
        let pm = detect_package_manager(root);
        if let Ok(text) = fs::read_to_string(root.join("package.json"))
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(map) = value.get("scripts").and_then(|v| v.as_object())
        {
            for (name, command) in map {
                let name_l = name.to_ascii_lowercase();
                if !(name_l.contains("test")
                    || name_l.contains("type")
                    || name_l.contains("lint")
                    || name_l.contains("check"))
                {
                    continue;
                }
                let runner = if pm == "unknown" { "npm" } else { &pm };
                let invoke = if name == "test" {
                    match runner {
                        "npm" => "npm test".to_string(),
                        "yarn" => "yarn test".to_string(),
                        "bun" => "bun test".to_string(),
                        _ => format!("{runner} test"),
                    }
                } else {
                    format!("{runner} run {name}")
                };
                scripts.push(ScriptInfo {
                    name: name.clone(),
                    command: invoke,
                    reason: format!("package.json script: {}", command.as_str().unwrap_or("")),
                    path: Some("package.json".to_string()),
                    line_start: json_key_line(&text, name),
                });
            }
        }
    }
    if root.join("Cargo.toml").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "cargo test".to_string(),
            reason: "Cargo.toml detected".to_string(),
            path: Some("Cargo.toml".to_string()),
            line_start: Some(1),
        });
    }
    if root.join("go.mod").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "go test ./...".to_string(),
            reason: "go.mod detected".to_string(),
            path: Some("go.mod".to_string()),
            line_start: Some(1),
        });
    }
    if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "pytest".to_string(),
            reason: "Python project files detected".to_string(),
            path: ["pyproject.toml", "requirements.txt"]
                .into_iter()
                .find(|path| root.join(path).exists())
                .map(str::to_string),
            line_start: Some(1),
        });
    }
    if root.join("Package.swift").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "swift test".to_string(),
            reason: "Package.swift detected".to_string(),
            path: Some("Package.swift".to_string()),
            line_start: Some(1),
        });
    }
    scripts.extend(makefile_scripts(root));
    scripts.extend(justfile_scripts(root));
    scripts.sort_by(|a, b| a.command.cmp(&b.command));
    scripts.dedup_by(|a, b| a.command == b.command);
    scripts
}

fn json_key_line(text: &str, key: &str) -> Option<usize> {
    let quoted = format!("\"{key}\"");
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(&quoted))
        .map(|(index, _)| index + 1)
}

fn detect_languages(files: &BTreeMap<String, FileInfo>) -> BTreeSet<String> {
    files
        .values()
        .filter_map(|file| match file.language.as_str() {
            "unknown" | "config" | "markdown" => None,
            other => Some(other.to_string()),
        })
        .collect()
}

fn discover_domains(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    anchors: &CodemapConfig,
    config_path: Option<&str>,
) -> Vec<Domain> {
    let mut domains = BTreeMap::<String, Domain>::new();
    if let Some(domain) = &anchors.domain {
        let id = domain.id.clone().unwrap_or_else(|| "repo".to_string());
        let path = normalize_rel_path(domain.path.as_deref().unwrap_or("."));
        domains.insert(
            id.clone(),
            Domain {
                id,
                path,
                config_path: config_path.map(str::to_string),
            },
        );
    }
    for (id, domain) in &anchors.domains {
        let path = normalize_rel_path(domain.path.as_deref().unwrap_or(id));
        domains.insert(
            id.clone(),
            Domain {
                id: id.clone(),
                path,
                config_path: config_path.map(str::to_string),
            },
        );
    }

    for rel in workspace_domain_paths(root, files) {
        let id = Path::new(&rel)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&rel)
            .to_string();
        domains.entry(id.clone()).or_insert(Domain {
            id,
            path: rel,
            config_path: None,
        });
    }

    for hint in DOMAIN_HINT_DIRS {
        let base = root.join(hint);
        if !base.is_dir() {
            continue;
        }
        let Ok(children) = fs::read_dir(base) else {
            continue;
        };
        for child in children.flatten() {
            let path = child.path();
            if !path.is_dir() {
                continue;
            }
            let rel =
                normalize_rel_path(&path.strip_prefix(root).unwrap_or(&path).to_string_lossy());
            if should_ignore_rel(&rel) {
                continue;
            }
            let has_files = files
                .keys()
                .any(|file| file.starts_with(&format!("{rel}/")));
            let has_markers = [
                "src",
                "tests",
                "test",
                "package.json",
                "Cargo.toml",
                "go.mod",
                ".codemap.yml",
            ]
            .iter()
            .any(|marker| path.join(marker).exists());
            if has_files || has_markers {
                let id = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&rel)
                    .to_string();
                domains.entry(id.clone()).or_insert(Domain {
                    id,
                    path: rel,
                    config_path: None,
                });
            }
        }
    }

    if domains.is_empty() {
        let id = anchors
            .domain
            .as_ref()
            .and_then(|d| d.id.clone())
            .unwrap_or_else(|| "repo".to_string());
        domains.insert(
            id.clone(),
            Domain {
                id,
                path: ".".to_string(),
                config_path: config_path.map(str::to_string),
            },
        );
    }

    domains.into_values().collect()
}

fn workspace_domain_paths(root: &Path, files: &BTreeMap<String, FileInfo>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for pattern in workspace_patterns(root) {
        expand_workspace_pattern(root, files, &pattern, &mut out);
    }
    out
}

fn workspace_patterns(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();
    if let Ok(text) = fs::read_to_string(root.join("package.json"))
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(workspaces) = value.get("workspaces")
    {
        if let Some(array) = workspaces.as_array() {
            patterns.extend(
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string)),
            );
        } else if let Some(array) = workspaces.get("packages").and_then(|v| v.as_array()) {
            patterns.extend(
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string)),
            );
        }
    }
    if let Ok(text) = fs::read_to_string(root.join("pnpm-workspace.yaml")) {
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("- ") {
                patterns.push(unquote(value.trim()).unwrap_or_else(|| value.trim().to_string()));
            }
        }
    }
    if let Ok(text) = fs::read_to_string(root.join("Cargo.toml")) {
        patterns.extend(cargo_workspace_array_values(&text, "members"));
    }
    if let Ok(text) = fs::read_to_string(root.join("go.work")) {
        patterns.extend(go_work_uses(&text));
    }
    if let Ok(text) = fs::read_to_string(root.join("pyproject.toml")) {
        patterns.extend(pyproject_workspace_patterns(&text));
    }
    patterns
        .into_iter()
        .map(|pattern| normalize_rel_path(pattern.trim().trim_start_matches("./")))
        .filter(|pattern| !pattern.is_empty() && pattern != ".")
        .collect()
}

fn expand_workspace_pattern(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    pattern: &str,
    out: &mut BTreeSet<String>,
) {
    if pattern.starts_with('!') || pattern.contains("**") || pattern.contains('{') {
        return;
    }
    if let Some(base) = pattern.strip_suffix("/*") {
        let base = normalize_rel_path(base);
        let Ok(children) = fs::read_dir(root.join(&base)) else {
            return;
        };
        for child in children.flatten() {
            let child_path = child.path();
            if child_path.is_dir() {
                let rel = normalize_rel_path(
                    &child_path
                        .strip_prefix(root)
                        .unwrap_or(&child_path)
                        .to_string_lossy(),
                );
                if workspace_path_has_project(root, files, &rel) {
                    out.insert(rel);
                }
            }
        }
        return;
    }
    if !pattern.contains('*') && workspace_path_has_project(root, files, pattern) {
        out.insert(normalize_rel_path(pattern));
    }
}

fn workspace_path_has_project(root: &Path, files: &BTreeMap<String, FileInfo>, rel: &str) -> bool {
    let rel = normalize_rel_path(rel);
    if !root.join(&rel).is_dir() || should_ignore_rel(&rel) {
        return false;
    }
    let prefix = format!("{}/", rel.trim_end_matches('/'));
    files.keys().any(|file| file.starts_with(&prefix))
        || [
            "package.json",
            "Cargo.toml",
            "go.mod",
            "pyproject.toml",
            "src",
        ]
        .iter()
        .any(|marker| root.join(&rel).join(marker).exists())
}

fn pyproject_workspace_patterns(text: &str) -> Vec<String> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in ["members", "packages"] {
        if let Some(values) = value.get(key).and_then(toml_string_array) {
            out.extend(values);
        }
        if let Some(values) = value
            .get("project")
            .and_then(|project| project.get(key))
            .and_then(toml_string_array)
        {
            out.extend(values);
        }
    }
    if let Some(values) = value
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("workspace"))
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml_string_array)
    {
        out.extend(values);
    }
    unique_strings(out)
}

fn go_work_uses(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("use (") {
            in_block = true;
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
            } else if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("use ") {
            out.push(value.trim().to_string());
        }
    }
    out
}
