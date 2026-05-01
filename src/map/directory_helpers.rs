fn sort_edges(edges: &mut Vec<StructuralEdge>) {
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.edge_type.cmp(&b.edge_type))
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
}

fn balanced_edge_prefix_by_source(edges: &[StructuralEdge], limit: usize) -> Vec<StructuralEdge> {
    if edges.len() <= limit {
        return edges.to_vec();
    }

    let mut buckets: BTreeMap<String, VecDeque<StructuralEdge>> = BTreeMap::new();
    for edge in edges {
        buckets
            .entry(edge.from.clone())
            .or_default()
            .push_back(edge.clone());
    }

    let mut balanced = Vec::with_capacity(limit);
    while balanced.len() < limit && !buckets.is_empty() {
        let keys = buckets.keys().cloned().collect::<Vec<_>>();
        let mut progressed = false;

        for key in keys {
            if balanced.len() == limit {
                break;
            }

            let mut empty = false;
            if let Some(bucket) = buckets.get_mut(&key) {
                if let Some(edge) = bucket.pop_front() {
                    balanced.push(edge);
                    progressed = true;
                }
                empty = bucket.is_empty();
            }
            if empty {
                buckets.remove(&key);
            }
        }

        if !progressed {
            break;
        }
    }

    balanced
}

fn limit_edge_section(
    edges: &mut Vec<StructuralEdge>,
    hidden: &mut Vec<HiddenGroup>,
    include_hidden: bool,
    limit: usize,
    reason: &str,
    expand: &str,
) {
    if include_hidden {
        return;
    }
    let count = edges.len();
    edges.truncate(limit);
    if count > edges.len() {
        hidden.push(HiddenGroup {
            reason: reason.to_string(),
            count: count - edges.len(),
            expand: expand_with_concrete_limit(expand, count),
        });
    }
}

fn directory_has_files(project: &Project, rel: &str) -> bool {
    if rel == "." {
        return !project.files.is_empty();
    }
    let prefix = format!("{}/", rel.trim_end_matches('/'));
    project.files.keys().any(|file| file.starts_with(&prefix))
}

fn parent_anchor_for_missing(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .map(|parent| repo::normalize_rel_path(&parent.to_string_lossy()))
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn files_under_directory<'a>(project: &'a Project, rel: &str) -> Vec<&'a FileInfo> {
    let prefix = (rel != ".").then(|| format!("{}/", rel.trim_end_matches('/')));
    project
        .files
        .values()
        .filter(|file| {
            prefix
                .as_ref()
                .map(|prefix| file.rel.starts_with(prefix))
                .unwrap_or(true)
        })
        .collect()
}

fn direct_files_under_directory<'a>(project: &'a Project, rel: &str) -> Vec<&'a FileInfo> {
    project
        .files
        .values()
        .filter(|file| direct_child_name(rel, &file.rel).is_some_and(|name| !name.ends_with('/')))
        .collect()
}

fn immediate_child_dirs(project: &Project, rel: &str) -> Vec<String> {
    let mut dirs = BTreeSet::new();
    for file in project.files.values() {
        if let Some(name) = direct_child_name(rel, &file.rel)
            && let Some(dir) = name.strip_suffix('/')
        {
            dirs.insert(if rel == "." {
                format!("{dir}/")
            } else {
                format!("{}/{dir}/", rel.trim_end_matches('/'))
            });
        }
    }
    dirs.into_iter().collect()
}

fn direct_child_name(scope: &str, path: &str) -> Option<String> {
    let scope = repo::normalize_rel_path(scope);
    let path = repo::normalize_rel_path(path);
    let rest = if scope == "." {
        path.as_str()
    } else {
        path.strip_prefix(&format!("{}/", scope.trim_end_matches('/')))?
    };
    if rest.is_empty() {
        return None;
    }
    if let Some((dir, _)) = rest.split_once('/') {
        return Some(format!("{dir}/"));
    }
    Some(rest.to_string())
}

fn directory_role_surface(project: &Project, dir: &str) -> Option<String> {
    if let Some(role) = directory_container_role_surface(project, dir) {
        return Some(role);
    }
    let prefix = dir.trim_end_matches('/');
    let files = direct_files_under_directory(project, prefix);
    if files.is_empty() {
        return None;
    }
    for role in [
        "e2e_test",
        "test_support",
        "fixture",
        "schema_contract",
        "build_ci",
        "docs",
        "test",
        "repo_discovery",
        "cache",
    ] {
        if files.iter().any(|file| file.has_role(role)) {
            return Some(role.to_string());
        }
    }
    None
}

fn directory_container_role_surface(project: &Project, dir: &str) -> Option<String> {
    let normalized = repo::normalize_rel_path(dir.trim_end_matches('/'));
    let name = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(normalized.as_str())
        .to_ascii_lowercase();
    let test_container = path_is_test_container(&normalized);
    if matches!(name.as_str(), "e2e" | "e2e-tests" | "playwright") {
        return Some("e2e_test".to_string());
    }
    if matches!(
        name.as_str(),
        "support" | "supports" | "helpers" | "page-objects" | "page_objects"
    ) && test_container
    {
        return Some("test_support".to_string());
    }
    if matches!(
        name.as_str(),
        "test" | "tests" | "__tests__" | "spec" | "specs"
    ) {
        let files = files_under_directory(project, &normalized);
        if files.iter().any(|file| file.has_role("e2e_test")) {
            return Some("e2e_test".to_string());
        }
        return Some("test".to_string());
    }
    if matches!(
        name.as_str(),
        "fixture" | "fixtures" | "example" | "examples" | "sample" | "samples"
    ) {
        return Some("fixture".to_string());
    }
    if matches!(
        name.as_str(),
        "schema" | "schemas" | "contract" | "contracts" | "migration" | "migrations"
    ) {
        return Some("schema_contract".to_string());
    }
    if matches!(name.as_str(), "doc" | "docs" | "documentation") {
        return Some("docs".to_string());
    }
    if matches!(
        name.as_str(),
        ".github" | "workflows" | ".circleci" | ".buildkite"
    ) {
        return Some("build_ci".to_string());
    }
    if matches!(name.as_str(), ".agents" | ".codex" | ".claude") {
        return Some("agent_support".to_string());
    }
    None
}

fn path_is_test_container(path: &str) -> bool {
    path.split('/').any(|part| {
        matches!(
            part,
            "test" | "tests" | "__tests__" | "e2e" | "spec" | "specs"
        )
    })
}
