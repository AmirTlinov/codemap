fn go_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_name: &BTreeMap<String, &PackageInfo>,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let replaces = go_replaces(&text);
    let mut edges = Vec::new();
    for dep in go_requires(&text) {
        if let Some(replacement) = replaces.get(&dep) {
            if let Some(target) = by_name.get(replacement) {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    workspace_manifest: None,
                    dependency: dep,
                    dependency_kind: "runtime".to_string(),
                    source: "go.mod replace".to_string(),
                });
                continue;
            }
            if let Some(target_path) = resolve_repo_relative_path(base, replacement)
                && let Some(target) = by_path.get(&target_path)
            {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    workspace_manifest: None,
                    dependency: dep,
                    dependency_kind: "runtime".to_string(),
                    source: "go.mod local replace".to_string(),
                });
                continue;
            }
        }
        if let Some(target) = by_name.get(&dep) {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: dep,
                dependency_kind: "runtime".to_string(),
                source: "go.mod require".to_string(),
            });
        }
    }
    edges
}

fn python_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for (dep, path) in pyproject_path_dependencies(&text) {
        if let Some(target_path) = resolve_repo_relative_path(base, &path)
            && let Some(target) = by_path.get(&target_path)
        {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: dep,
                dependency_kind: "runtime".to_string(),
                source: "pyproject local path dependency".to_string(),
            });
        }
    }
    edges
}

fn swift_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for path in swift_package_path_dependencies(&text) {
        if let Some(target_path) = resolve_repo_relative_path(base, &path)
            && let Some(target) = by_path.get(&target_path)
        {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: package_name_from_path(&target.path),
                dependency_kind: "runtime".to_string(),
                source: "Package.swift local path dependency".to_string(),
            });
        }
    }
    edges
}

fn manifest_dir(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .map(|p| normalize_rel_path(&p.to_string_lossy()))
        .filter(|p| p != ".")
        .unwrap_or_else(|| ".".to_string())
}

fn package_name_from_path(path: &str) -> String {
    if path == "." {
        "repo".to_string()
    } else {
        Path::new(path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(path)
            .to_string()
    }
}

fn cargo_package_name(text: &str) -> Option<String> {
    parse_toml_value(text)?
        .get("package")?
        .get("name")?
        .as_str()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

fn cargo_path_dependencies(text: &str) -> Vec<(String, String, String)> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    for (table, kind) in cargo_dependency_tables(&value) {
        deps.extend(cargo_table_path_dependencies(table, kind));
    }
    unique_triples(deps)
}

fn cargo_workspace_path_dependencies(text: &str) -> BTreeMap<String, String> {
    let Some(value) = parse_toml_value(text) else {
        return BTreeMap::new();
    };
    let mut deps = BTreeMap::new();
    if let Some(table) = value
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.insert(name.to_string(), path);
            }
        }
    }
    deps
}

fn cargo_workspace_dependency_names(text: &str) -> Vec<(String, String)> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    for (table, kind) in cargo_dependency_tables(&value) {
        for (name, dependency) in table {
            if toml_workspace_field(dependency) == Some(true) {
                deps.push((name.to_string(), kind.to_string()));
            }
        }
    }
    unique_pairs(deps)
}

fn cargo_workspace_declared(text: &str) -> bool {
    parse_toml_value(text).is_some_and(|value| value.get("workspace").is_some())
}

fn cargo_workspace_array_values(text: &str, key: &str) -> Vec<String> {
    parse_toml_value(text)
        .and_then(|value| value.get("workspace").cloned())
        .and_then(|workspace| workspace.get(key).cloned())
        .and_then(|value| toml_string_array(&value))
        .unwrap_or_default()
}

fn parse_toml_value(text: &str) -> Option<toml::Value> {
    toml::from_str::<toml::Value>(text).ok()
}

fn cargo_dependency_tables(value: &toml::Value) -> Vec<(&toml::Table, &'static str)> {
    let mut tables = Vec::new();
    collect_cargo_dependency_tables(value, &mut tables);
    if let Some(targets) = value.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            collect_cargo_dependency_tables(target, &mut tables);
        }
    }
    tables
}

fn collect_cargo_dependency_tables<'a>(
    value: &'a toml::Value,
    out: &mut Vec<(&'a toml::Table, &'static str)>,
) {
    for (section, kind) in [
        ("dependencies", "runtime"),
        ("dev-dependencies", "dev"),
        ("build-dependencies", "build"),
    ] {
        if let Some(table) = value.get(section).and_then(toml::Value::as_table) {
            out.push((table, kind));
        }
    }
}

fn cargo_table_path_dependencies(
    table: &toml::Table,
    kind: &str,
) -> Vec<(String, String, String)> {
    table
        .iter()
        .filter_map(|(name, dependency)| {
            toml_path_field(dependency).map(|path| (name.to_string(), path, kind.to_string()))
        })
        .collect()
}

fn toml_path_field(value: &toml::Value) -> Option<String> {
    value
        .get("path")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .filter(|path| !path.is_empty())
}

fn toml_workspace_field(value: &toml::Value) -> Option<bool> {
    value.get("workspace").and_then(toml::Value::as_bool)
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

fn go_module_name(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if let Some(value) = trimmed.strip_prefix("module ") {
            return value
                .split_whitespace()
                .next()
                .map(str::to_string)
                .filter(|value| !value.is_empty());
        }
    }
    None
}

fn go_requires(text: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some(module) = trimmed.split_whitespace().next()
                && !module.is_empty()
            {
                deps.push(module.to_string());
            }
            continue;
        }
        if trimmed == "require (" {
            in_block = true;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("require ") {
            if value.trim_start().starts_with('(') {
                in_block = true;
                continue;
            }
            if let Some(module) = value.split_whitespace().next()
                && !module.is_empty()
            {
                deps.push(module.to_string());
            }
        }
    }
    unique_strings(deps)
}

fn go_replaces(text: &str) -> BTreeMap<String, String> {
    let mut replaces = BTreeMap::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some((from, to)) = parse_go_replace(trimmed) {
                replaces.insert(from, to);
            }
            continue;
        }
        if trimmed == "replace (" {
            in_block = true;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("replace ") {
            if value.trim_start().starts_with('(') {
                in_block = true;
                continue;
            }
            if let Some((from, to)) = parse_go_replace(value) {
                replaces.insert(from, to);
            }
        }
    }
    replaces
}

fn parse_go_replace(value: &str) -> Option<(String, String)> {
    let (from, to) = value.split_once("=>")?;
    let from = from.split_whitespace().next()?.to_string();
    let to = to.split_whitespace().next()?.to_string();
    (!from.is_empty() && !to.is_empty()).then_some((from, to))
}

fn strip_go_mod_comment(line: &str) -> &str {
    line.split_once("//").map(|(head, _)| head).unwrap_or(line)
}

fn extract_go_imports(text: &str) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some(path) = quoted_go_import(trimmed) {
                imports.insert(path);
            }
            continue;
        }
        let Some(value) = trimmed.strip_prefix("import") else {
            continue;
        };
        let value = value.trim_start();
        if value.starts_with('(') {
            in_block = true;
            let rest = value.trim_start_matches('(').trim();
            if !rest.is_empty()
                && !rest.starts_with(')')
                && let Some(path) = quoted_go_import(rest)
            {
                imports.insert(path);
            }
            continue;
        }
        if let Some(path) = quoted_go_import(value) {
            imports.insert(path);
        }
    }
    imports
}

fn quoted_go_import(value: &str) -> Option<String> {
    let quote_start = value.find('"')?;
    let tail = &value[quote_start + 1..];
    let quote_end = tail.find('"')?;
    let path = &tail[..quote_end];
    (!path.is_empty()).then_some(path.to_string())
}

fn pyproject_package_name(text: &str) -> Option<String> {
    let value = parse_toml_value(text)?;
    value
        .get("project")
        .and_then(|project| project.get("name"))
        .or_else(|| {
            value
                .get("tool")
                .and_then(|tool| tool.get("poetry"))
                .and_then(|poetry| poetry.get("name"))
        })
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .filter(|name| !name.is_empty())
}

fn pyproject_path_dependencies(text: &str) -> Vec<(String, String)> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    if let Some(table) = value
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("sources"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.push((name.to_string(), path));
            }
        }
    }
    if let Some(table) = value
        .get("tool")
        .and_then(|tool| tool.get("poetry"))
        .and_then(|poetry| poetry.get("dependencies"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.push((name.to_string(), path));
            }
        }
    }
    unique_pairs(deps)
}

fn swift_package_name(text: &str) -> Option<String> {
    swift_package_name_re()
        .captures(text)?
        .get(1)
        .map(|m| m.as_str().to_string())
        .filter(|name| !name.is_empty())
}

fn swift_package_path_dependencies(text: &str) -> Vec<String> {
    let mut deps = swift_package_path_dependency_re()
        .captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();
    deps.sort();
    deps.dedup();
    deps
}

fn unquote(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches(',');
    trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
        })
        .map(str::to_string)
}
