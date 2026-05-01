fn detect_package_edges(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    packages: &[PackageInfo],
) -> Vec<PackageDependency> {
    let mut edges = Vec::new();
    let by_name: BTreeMap<String, &PackageInfo> = packages
        .iter()
        .map(|package| (package.name.clone(), package))
        .collect();
    let by_path: BTreeMap<String, &PackageInfo> = packages
        .iter()
        .map(|package| (package.path.clone(), package))
        .collect();
    let cargo_workspaces = cargo_workspace_infos(root, files, packages, &by_path);

    for package in packages {
        match package.ecosystem.as_str() {
            "javascript" => {
                edges.extend(js_package_edges(root, package, &by_name, &by_path));
            }
            "rust" => {
                edges.extend(cargo_package_edges(
                    root,
                    package,
                    &by_path,
                    &cargo_workspaces,
                ));
            }
            "go" => {
                edges.extend(go_package_edges(root, package, &by_name, &by_path));
            }
            "python" => {
                edges.extend(python_package_edges(root, package, &by_path));
            }
            "swift" => {
                edges.extend(swift_package_edges(root, package, &by_path));
            }
            _ => {}
        }
    }
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.dependency.cmp(&b.dependency))
    });
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.dependency == b.dependency && a.source == b.source
    });
    edges
}

fn js_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_name: &BTreeMap<String, &PackageInfo>,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        let Some(map) = value.get(section).and_then(|v| v.as_object()) else {
            continue;
        };
        for (dep, spec) in map {
            if let Some(spec) = spec.as_str() {
                if let Some(path) = js_local_dependency_path(spec) {
                    if let Some(target_path) = resolve_repo_relative_path(base, &path)
                        && let Some(target) = by_path.get(&target_path)
                    {
                        edges.push(PackageDependency {
                            from: package.path.clone(),
                            from_manifest: package.manifest.clone(),
                            to: target.path.clone(),
                            to_manifest: Some(target.manifest.clone()),
                            workspace_manifest: None,
                            dependency: dep.clone(),
                            source: format!("package.json {section} local path"),
                        });
                    }
                    continue;
                }
                if js_dependency_spec_is_local_protocol(spec) {
                    continue;
                }
            }
            if let Some(target) = by_name.get(dep) {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    workspace_manifest: None,
                    dependency: dep.clone(),
                    source: format!("package.json {section}"),
                });
            }
        }
    }
    edges
}

fn js_local_dependency_path(spec: &str) -> Option<String> {
    let spec = spec.trim();
    for prefix in ["file:", "link:", "portal:", "workspace:"] {
        if let Some(path) = spec.strip_prefix(prefix) {
            let path = path.trim();
            if path.starts_with("./") || path.starts_with("../") || path == "." || path == ".." {
                return Some(path.to_string());
            }
        }
    }
    None
}

fn js_dependency_spec_is_local_protocol(spec: &str) -> bool {
    let spec = spec.trim();
    if ["file:", "link:", "portal:"]
        .iter()
        .any(|prefix| spec.starts_with(prefix))
    {
        return true;
    }
    let Some(path) = spec.strip_prefix("workspace:") else {
        return false;
    };
    let path = path.trim().replace('\\', "/");
    path.starts_with("./")
        || path.starts_with("../")
        || path == "."
        || path == ".."
        || path_is_absolute_like(&path)
}

fn cargo_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
    workspaces: &[CargoWorkspaceInfo],
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges: Vec<PackageDependency> = cargo_path_dependencies(&text)
        .into_iter()
        .filter_map(|(name, path)| {
            let target_path = resolve_repo_relative_path(base, &path)?;
            let target = by_path.get(&target_path)?;
            Some(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: name,
                source: "Cargo.toml path dependency".to_string(),
            })
        })
        .collect();
    let workspace = cargo_workspace_for_package(package, workspaces);
    let workspace_dependencies = workspace
        .map(|workspace| &workspace.dependencies)
        .cloned()
        .unwrap_or_default();
    let workspace_manifest = workspace.map(|workspace| workspace.manifest.clone());
    for name in cargo_workspace_dependency_names(&text) {
        let Some(path) = workspace_dependencies.get(&name) else {
            continue;
        };
        let Some(target) = by_path.get(path) else {
            continue;
        };
        edges.push(PackageDependency {
            from: package.path.clone(),
            from_manifest: package.manifest.clone(),
            to: target.path.clone(),
            to_manifest: Some(target.manifest.clone()),
            workspace_manifest: workspace_manifest.clone(),
            dependency: name,
            source: "Cargo.toml workspace dependency".to_string(),
        });
    }
    edges
}

#[derive(Debug, Clone)]
struct CargoWorkspaceInfo {
    manifest: String,
    path: String,
    dependencies: BTreeMap<String, String>,
    members: Vec<String>,
    exclude: Vec<String>,
    member_paths: BTreeSet<String>,
}

fn cargo_workspace_infos(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    packages: &[PackageInfo],
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<CargoWorkspaceInfo> {
    let mut workspaces = Vec::new();
    for rel in files.keys() {
        if Path::new(rel).file_name().and_then(|name| name.to_str()) != Some("Cargo.toml") {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue;
        };
        if !cargo_workspace_declared(&text) {
            continue;
        }
        let path = manifest_dir(rel);
        let dependencies = cargo_workspace_path_dependencies(&text)
            .into_iter()
            .filter_map(|(name, dependency_path)| {
                resolve_repo_relative_path(Path::new(&path), &dependency_path)
                    .map(|resolved| (name, resolved))
            })
            .collect();
        workspaces.push(CargoWorkspaceInfo {
            manifest: rel.clone(),
            path,
            dependencies,
            members: cargo_workspace_array_values(&text, "members"),
            exclude: cargo_workspace_array_values(&text, "exclude"),
            member_paths: BTreeSet::new(),
        });
    }
    workspaces.sort_by(|a, b| a.path.cmp(&b.path));
    for workspace in &mut workspaces {
        workspace.member_paths = cargo_workspace_member_paths(root, workspace, packages, by_path);
    }
    workspaces
}

fn cargo_workspace_for_package<'a>(
    package: &PackageInfo,
    workspaces: &'a [CargoWorkspaceInfo],
) -> Option<&'a CargoWorkspaceInfo> {
    workspaces
        .iter()
        .filter(|workspace| cargo_workspace_contains_package(workspace, &package.path))
        .max_by_key(|workspace| workspace.path.len())
}

fn cargo_workspace_contains_package(workspace: &CargoWorkspaceInfo, package_path: &str) -> bool {
    workspace.member_paths.contains(package_path)
}

fn cargo_workspace_member_paths(
    root: &Path,
    workspace: &CargoWorkspaceInfo,
    packages: &[PackageInfo],
    by_path: &BTreeMap<String, &PackageInfo>,
) -> BTreeSet<String> {
    let mut members: BTreeSet<String> = packages
        .iter()
        .filter(|package| cargo_workspace_explicitly_contains_package(workspace, &package.path))
        .map(|package| package.path.clone())
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for package_path in members.iter().cloned().collect::<Vec<_>>() {
            let Some(package) = by_path.get(&package_path) else {
                continue;
            };
            let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
                continue;
            };
            let base = Path::new(&package.manifest)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            for (_, path) in cargo_path_dependencies(&text) {
                let Some(target_path) = resolve_repo_relative_path(base, &path) else {
                    continue;
                };
                if by_path.contains_key(&target_path)
                    && cargo_workspace_path_allowed(workspace, &target_path)
                    && members.insert(target_path)
                {
                    changed = true;
                }
            }
            for name in cargo_workspace_dependency_names(&text) {
                let Some(target_path) = workspace.dependencies.get(&name) else {
                    continue;
                };
                if by_path.contains_key(target_path)
                    && cargo_workspace_path_allowed(workspace, target_path)
                    && members.insert(target_path.clone())
                {
                    changed = true;
                }
            }
        }
    }
    members
}

fn cargo_workspace_explicitly_contains_package(
    workspace: &CargoWorkspaceInfo,
    package_path: &str,
) -> bool {
    let rel = match path_relative_to(package_path, &workspace.path) {
        Some(rel) => rel,
        None => return false,
    };
    if !cargo_workspace_rel_allowed(workspace, &rel) {
        return false;
    }
    if rel == "." {
        return true;
    }
    workspace
        .members
        .iter()
        .any(|pattern| cargo_workspace_member_pattern_matches(&rel, pattern))
}

fn cargo_workspace_path_allowed(workspace: &CargoWorkspaceInfo, package_path: &str) -> bool {
    path_relative_to(package_path, &workspace.path)
        .map(|rel| cargo_workspace_rel_allowed(workspace, &rel))
        .unwrap_or(false)
}

fn cargo_workspace_rel_allowed(workspace: &CargoWorkspaceInfo, rel: &str) -> bool {
    !workspace
        .exclude
        .iter()
        .any(|pattern| cargo_workspace_member_pattern_matches(rel, pattern))
}

fn path_relative_to(path: &str, base: &str) -> Option<String> {
    let path = normalize_rel_path(path);
    let base = normalize_rel_path(base);
    if base == "." {
        return Some(path);
    }
    if path == base {
        return Some(".".to_string());
    }
    let prefix = format!("{}/", base.trim_end_matches('/'));
    path.strip_prefix(&prefix).map(str::to_string)
}

fn cargo_workspace_member_pattern_matches(rel: &str, pattern: &str) -> bool {
    let rel = normalize_rel_path(rel.trim().trim_start_matches("./"));
    let Some(pattern) = cargo_normalize_workspace_member_pattern(pattern) else {
        return false;
    };
    if pattern == "." {
        return rel == ".";
    }
    let mut builder = GlobBuilder::new(&pattern);
    builder.literal_separator(true);
    builder
        .build()
        .map(|glob| glob.compile_matcher().is_match(&rel))
        .unwrap_or(rel == pattern)
}

fn resolve_repo_relative_path(base: &Path, path: &str) -> Option<String> {
    let raw = path.trim().replace('\\', "/");
    if raw.is_empty() || path_is_absolute_like(&raw) {
        return None;
    }
    let base = normalize_rel_path(&base.to_string_lossy());
    let mut parts: Vec<String> = if base == "." {
        Vec::new()
    } else {
        base.split('/').map(str::to_string).collect()
    };
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other.to_string()),
        }
    }
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

fn cargo_normalize_workspace_member_pattern(pattern: &str) -> Option<String> {
    let raw = pattern.trim().trim_start_matches("./").replace('\\', "/");
    if raw.is_empty() || path_is_absolute_like(&raw) {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    for part in raw.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other.to_string()),
        }
    }
    Some(if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    })
}

fn path_is_absolute_like(path: &str) -> bool {
    path.starts_with('/')
        || path.starts_with("//")
        || path
            .split('/')
            .next()
            .is_some_and(|part| part.ends_with(':'))
}

