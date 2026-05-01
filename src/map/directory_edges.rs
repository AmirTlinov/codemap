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
    sort_edges(&mut edges);
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
