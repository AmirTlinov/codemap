// Responsibility: map-package-consumers
use crate::model::Project;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::path::Path;

pub(crate) fn package_consumer_manifests(
    project: &Project,
    changed: &[String],
    depth: usize,
    limit: usize,
) -> Vec<String> {
    if depth == 0 || limit == 0 {
        return Vec::new();
    }
    let mut roots = BTreeSet::new();
    for rel in changed {
        if !requires_package_consumer_expansion(project, rel) {
            continue;
        }
        if let Some(package) = package_for_rel(project, rel) {
            roots.insert(package.path.clone());
        }
    }
    if roots.is_empty() {
        let workspace_roots = workspace_manifest_consumers(project, changed, depth, limit);
        return workspace_roots;
    }
    let mut traversal = PackageConsumerTraversal {
        seen: roots.clone(),
        queue: roots.into_iter().map(|path| (path, 0)).collect(),
        out: Vec::new(),
        out_seen: BTreeSet::new(),
    };
    seed_workspace_manifest_consumers(project, changed, depth, limit, &mut traversal);
    while let Some((package_path, d)) = traversal.queue.pop_front() {
        if traversal.out.len() >= limit {
            break;
        }
        for edge in project
            .package_edges
            .iter()
            .filter(|edge| edge.to == package_path)
        {
            if traversal.seen.insert(edge.from.clone()) {
                if traversal.out_seen.insert(edge.from_manifest.clone()) {
                    traversal.out.push(edge.from_manifest.clone());
                }
                if d + 1 < depth {
                    traversal.queue.push_back((edge.from.clone(), d + 1));
                }
                if traversal.out.len() >= limit {
                    break;
                }
            }
        }
    }
    traversal.out
}

struct PackageConsumerTraversal {
    seen: BTreeSet<String>,
    queue: VecDeque<(String, usize)>,
    out: Vec<String>,
    out_seen: BTreeSet<String>,
}

fn workspace_manifest_consumers(
    project: &Project,
    changed: &[String],
    depth: usize,
    limit: usize,
) -> Vec<String> {
    let mut traversal = PackageConsumerTraversal {
        seen: BTreeSet::new(),
        queue: VecDeque::new(),
        out: Vec::new(),
        out_seen: BTreeSet::new(),
    };
    seed_workspace_manifest_consumers(project, changed, depth, limit, &mut traversal);
    while let Some((package_path, d)) = traversal.queue.pop_front() {
        if traversal.out.len() >= limit {
            break;
        }
        for edge in project
            .package_edges
            .iter()
            .filter(|edge| edge.to == package_path)
        {
            if traversal.seen.insert(edge.from.clone()) {
                if traversal.out_seen.insert(edge.from_manifest.clone()) {
                    traversal.out.push(edge.from_manifest.clone());
                }
                if d + 1 < depth {
                    traversal.queue.push_back((edge.from.clone(), d + 1));
                }
                if traversal.out.len() >= limit {
                    break;
                }
            }
        }
    }
    traversal.out
}

fn seed_workspace_manifest_consumers(
    project: &Project,
    changed: &[String],
    depth: usize,
    limit: usize,
    traversal: &mut PackageConsumerTraversal,
) {
    if depth == 0 || limit == 0 {
        return;
    }
    for rel in changed {
        if !requires_package_consumer_expansion(project, rel) {
            continue;
        }
        for edge in project
            .package_edges
            .iter()
            .filter(|edge| edge.workspace_manifest.as_deref() == Some(rel.as_str()))
        {
            if traversal.seen.insert(edge.from.clone()) {
                if traversal.out_seen.insert(edge.from_manifest.clone()) {
                    traversal.out.push(edge.from_manifest.clone());
                }
                if 1 < depth {
                    traversal.queue.push_back((edge.from.clone(), 1));
                }
                if traversal.out.len() >= limit {
                    return;
                }
            }
        }
    }
}

fn requires_package_consumer_expansion(project: &Project, rel: &str) -> bool {
    let Some(file) = project.files.get(rel) else {
        return false;
    };
    file.has_role("public_boundary")
        || file.has_role("schema_contract")
        || matches!(
            Path::new(rel).file_name().and_then(|name| name.to_str()),
            Some("package.json" | "Cargo.toml" | "go.mod" | "pyproject.toml")
        )
}

pub(crate) fn package_for_rel<'a>(
    project: &'a Project,
    rel: &str,
) -> Option<&'a crate::model::PackageInfo> {
    let mut best = None;
    let mut best_len = 0usize;
    for package in &project.packages {
        let prefix = package.path.trim_end_matches('/');
        let matches = prefix == "."
            || rel == package.manifest
            || rel == prefix
            || rel.starts_with(&format!("{prefix}/"));
        if matches {
            let len = if prefix == "." { 0 } else { prefix.len() };
            if len >= best_len {
                best = Some(package);
                best_len = len;
            }
        }
    }
    best
}
