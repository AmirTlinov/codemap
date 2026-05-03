fn directory_edges(project: &Project, rel: &str, include_hidden: bool) -> Vec<StructuralEdge> {
    directory_edges_at_depth(project, rel, include_hidden, 1)
}

fn directory_edges_at_depth(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut grouped: BTreeMap<(String, String, String, String, EvidenceStrength), usize> =
        BTreeMap::new();
    let scope_is_support = is_support_artifact_path(rel);
    for file in files_under_directory(project, rel) {
        for target in &file.resolved_imports {
            if !include_hidden
                && !scope_is_support
                && (is_support_artifact_path(&file.rel) || is_support_artifact_path(target))
            {
                continue;
            }
            let from = directory_edge_endpoint_at_depth(project, rel, &file.rel, endpoint_depth);
            let to = directory_edge_endpoint_at_depth(project, rel, target, endpoint_depth);
            if from != to {
                add_directory_edge(
                    &mut grouped,
                    from,
                    to,
                    "outgoing_import",
                    "resolved_import",
                    EvidenceStrength::High,
                );
            }
        }
        if let Some(importers) = project.reverse_imports.get(&file.rel) {
            for importer in importers {
                if path_under_scope(importer, rel) {
                    continue;
                }
                if !include_hidden
                    && !scope_is_support
                    && (is_support_artifact_path(&file.rel) || is_support_artifact_path(importer))
                {
                    continue;
                }
                let from = directory_edge_endpoint_at_depth(project, rel, importer, endpoint_depth);
                let to = directory_edge_endpoint_at_depth(project, rel, &file.rel, endpoint_depth);
                if from != to {
                    add_directory_edge(
                        &mut grouped,
                        from,
                        to,
                        "incoming_import",
                        "reverse_import",
                        EvidenceStrength::High,
                    );
                }
            }
        }
    }
    for edge in &project.package_edges {
        if !include_hidden
            && !scope_is_support
            && (is_support_artifact_path(&edge.from_manifest)
                || edge
                    .to_manifest
                    .as_ref()
                    .map(|to| is_support_artifact_path(to))
                    .unwrap_or_else(|| is_support_artifact_path(&edge.to)))
        {
            continue;
        }
        let from_in = path_under_scope(&edge.from_manifest, rel);
        let to_in = edge
            .to_manifest
            .as_ref()
            .map(|to| path_under_scope(to, rel))
            .unwrap_or_else(|| path_under_scope(&edge.to, rel));
        if from_in || to_in {
            add_directory_edge(
                &mut grouped,
                directory_edge_endpoint_at_depth(project, rel, &edge.from_manifest, endpoint_depth),
                directory_edge_endpoint_at_depth(
                    project,
                    rel,
                    &edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone()),
                    endpoint_depth,
                ),
                if from_in && to_in {
                    "package_internal"
                } else if from_in {
                    "package_outgoing"
                } else {
                    "package_incoming"
                },
                &format!("package_manifest:{}", edge.dependency),
                EvidenceStrength::High,
            );
        }
    }
    let mut edges = grouped
        .into_iter()
        .map(
            |((from, to, edge_type, evidence, strength), count)| {
                edge_with_aggregate_location(
                    from,
                    to,
                    edge_type,
                    if count > 1 {
                        format!("{evidence}:{count}")
                    } else {
                        evidence
                    },
                    strength,
                    "directory_edge_aggregate",
                )
            },
        )
        .collect::<Vec<_>>();
    edges.extend(current_level_owner_edges(
        project,
        rel,
        include_hidden,
        endpoint_depth,
    ));
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    edges
}

fn add_directory_edge(
    grouped: &mut BTreeMap<(String, String, String, String, EvidenceStrength), usize>,
    from: String,
    to: String,
    edge_type: &str,
    evidence: &str,
    strength: EvidenceStrength,
) {
    if from == to {
        return;
    }
    *grouped
        .entry((
            from,
            to,
            edge_type.to_string(),
            evidence.to_string(),
            strength,
        ))
        .or_insert(0) += 1;
}

fn directory_edge_endpoint_at_depth(
    project: &Project,
    scope: &str,
    path: &str,
    depth: usize,
) -> String {
    let scope = repo::normalize_rel_path(scope);
    let path = repo::normalize_rel_path(path);
    let depth = depth.max(1);
    if scope == "." {
        if let Some(endpoint) = package_endpoint_at_depth(project, &path, depth) {
            return endpoint;
        }
        return top_level_endpoint_at_depth(&path, depth);
    }
    if let Some(rest) = path.strip_prefix(&format!("{}/", scope.trim_end_matches('/'))) {
        let mut parts = rest.split('/').collect::<Vec<_>>();
        if parts.len() <= 1 {
            return format!("{}/", scope.trim_end_matches('/'));
        }
        parts.pop();
        let dirs = parts.into_iter().take(depth).collect::<Vec<_>>();
        if dirs.is_empty() {
            format!("{}/", scope.trim_end_matches('/'))
        } else {
            format!("{}/{}/", scope.trim_end_matches('/'), dirs.join("/"))
        }
    } else {
        if let Some(endpoint) = package_endpoint_at_depth(project, &path, depth) {
            return endpoint;
        }
        top_level_endpoint_at_depth(&path, depth)
    }
}

fn package_endpoint_at_depth(project: &Project, path: &str, depth: usize) -> Option<String> {
    let path = repo::normalize_rel_path(path);
    // The outermost package is the map envelope; depth expands inside that envelope.
    let package = project
        .packages
        .iter()
        .filter(|package| package.path != "." && path_under_scope(&path, &package.path))
        .min_by_key(|package| package.path.len())?;
    let rest = path_relative_to_map(&path, &package.path).unwrap_or_else(|| ".".to_string());
    if depth <= 1 || rest == "." {
        return Some(format!("{}/", package.path.trim_end_matches('/')));
    }
    let mut parts = rest.split('/').collect::<Vec<_>>();
    if parts.len() <= 1 {
        return Some(format!("{}/", package.path.trim_end_matches('/')));
    }
    parts.pop();
    let dirs = parts
        .into_iter()
        .take(depth.saturating_sub(1))
        .collect::<Vec<_>>();
    if dirs.is_empty() {
        Some(format!("{}/", package.path.trim_end_matches('/')))
    } else {
        Some(format!(
            "{}/{}/",
            package.path.trim_end_matches('/'),
            dirs.join("/")
        ))
    }
}

fn path_relative_to_map(path: &str, base: &str) -> Option<String> {
    let path = repo::normalize_rel_path(path);
    let base = repo::normalize_rel_path(base);
    if base == "." {
        return Some(path);
    }
    if path == base {
        return Some(".".to_string());
    }
    path.strip_prefix(&format!("{}/", base.trim_end_matches('/')))
        .map(str::to_string)
}

fn top_level_endpoint_at_depth(path: &str, depth: usize) -> String {
    let mut parts = path.split('/');
    let depth = depth.max(1);
    let segments = path.split('/').collect::<Vec<_>>();
    if segments.len() <= 1 {
        return path.to_string();
    }
    if let (Some(first), Some(_second)) = (parts.next(), parts.next())
        && matches!(
            first,
            "apps" | "packages" | "services" | "domains" | "crates" | "modules"
        )
    {
        let take = (2 + depth.saturating_sub(1)).min(segments.len().saturating_sub(1));
        return format!("{}/", segments[..take].join("/"));
    }
    let take = depth.min(segments.len().saturating_sub(1));
    format!("{}/", segments[..take].join("/"))
}

fn path_under_scope(path: &str, scope: &str) -> bool {
    let path = repo::normalize_rel_path(path);
    let scope = repo::normalize_rel_path(scope);
    scope == "." || path == scope || path.starts_with(&format!("{}/", scope.trim_end_matches('/')))
}

fn script_target_for_path(path: &str, name: &str) -> String {
    let scope = manifest_dir_for_rel(path);
    if scope == "." {
        format!("script:{name}")
    } else {
        format!("script:{scope}:{name}")
    }
}

fn script_target_for_package(package: &crate::model::PackageInfo, name: &str) -> String {
    if package.path == "." {
        format!("script:{name}")
    } else {
        format!("script:{}:{name}", package.path)
    }
}

fn command_target(command: &str) -> String {
    format!("command:{}", command.trim())
}

fn include_package_script_command_edge(scope: &str) -> bool {
    repo::normalize_rel_path(scope) != "."
}

fn command_invokes_script_surface(command: &str, script: &crate::model::ScriptInfo) -> bool {
    if command_invokes_script(command, &script.name) {
        return true;
    }
    let command_parts = command_tokens(command);
    let script_tokens = command_tokens(&script.command);
    !script_tokens.is_empty()
        && command_parts
            .windows(script_tokens.len())
            .any(|window| window == script_tokens.as_slice())
}

fn validation_command_like(command: &str) -> bool {
    let tokens = command_tokens(command);
    if tokens.is_empty() {
        return false;
    }
    tokens.windows(2).any(|window| {
        matches!(
            (window[0].as_str(), window[1].as_str()),
            ("cargo", "test")
                | ("cargo", "check")
                | ("cargo", "clippy")
                | ("cargo", "fmt")
                | ("go", "test")
                | ("swift", "test")
                | ("swift", "build")
                | ("python", "-m")
        )
    }) || tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "pytest"
                | "vitest"
                | "jest"
                | "eslint"
                | "biome"
                | "typecheck"
                | "svelte-check"
                | "doctor"
        )
    }) || command_uses_e2e_runner(&tokens)
        || tokens.iter().enumerate().any(|(index, token)| {
        matches!(token.as_str(), "test" | "lint" | "check" | "build" | "verify" | "e2e")
            && token_invokes_package_script(&tokens, index)
    })
}

fn command_uses_e2e_runner(tokens: &[String]) -> bool {
    tokens
        .iter()
        .enumerate()
        .any(|(index, token)| matches!(token.as_str(), "playwright" | "cypress")
            && tokens[index + 1..]
                .iter()
                .any(|later| matches!(later.as_str(), "test" | "e2e" | "run")))
}

fn workspace_edge_directory_target(
    project: &Project,
    scope: &str,
    target: &str,
    endpoint_depth: usize,
) -> String {
    if project.files.contains_key(target)
        || project
            .packages
            .iter()
            .any(|package| package.path == target || package.manifest == target)
    {
        directory_edge_endpoint_at_depth(project, scope, target, endpoint_depth)
    } else {
        target.to_string()
    }
}

fn lockfiles_for_package<'a>(
    project: &'a Project,
    package: &crate::model::PackageInfo,
) -> Vec<&'a FileInfo> {
    let package_dir = package.path.trim_end_matches('/');
    project
        .files
        .values()
        .filter(|file| file.has_role("lockfile"))
        .filter(|file| match package.ecosystem.as_str() {
            "rust" => {
                if package.path == "." {
                    file.rel == "Cargo.lock"
                } else {
                    file.rel == format!("{package_dir}/Cargo.lock")
                }
            }
            "javascript" => {
                matches!(
                    Path::new(&file.rel).file_name().and_then(|name| name.to_str()),
                    Some(
                        "pnpm-lock.yaml"
                            | "pnpm-lock.yml"
                            | "package-lock.json"
                            | "yarn.lock"
                            | "bun.lock"
                            | "bun.lockb"
                    )
                ) && ((package.path == "." && !file.rel.contains('/'))
                    || file.rel == format!("{package_dir}/pnpm-lock.yaml")
                    || file.rel == format!("{package_dir}/package-lock.json")
                    || file.rel == format!("{package_dir}/yarn.lock"))
            }
            "python" => matches!(
                Path::new(&file.rel).file_name().and_then(|name| name.to_str()),
                Some("poetry.lock" | "pdm.lock" | "uv.lock")
            ),
            _ => false,
        })
        .collect()
}

fn should_hide_owner_edge_path(path: &str, scope_is_support: bool) -> bool {
    !scope_is_support && is_support_artifact_path(path)
}

fn schema_owner_directory(rel: &str) -> String {
    let dir = manifest_dir_for_rel(rel);
    if dir.ends_with("/prisma") || dir.ends_with("/db") || dir.ends_with("/schema") {
        dir
    } else if rel.contains("/migrations/") {
        rel.split("/migrations/")
            .next()
            .map(repo::normalize_rel_path)
            .unwrap_or(dir)
    } else {
        dir
    }
}
