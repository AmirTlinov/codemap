// Responsibility: map-boundary-package-graph
use crate::map::glob_match;
use crate::model::Project;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

pub(crate) struct PackageGraphPath {
    pub(crate) from: String,
    pub(crate) from_manifest: String,
    pub(crate) to: String,
    pub(crate) to_manifest: Option<String>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) manifests: Vec<String>,
}

pub(crate) fn package_transitive_paths(
    project: &Project,
    max_depth: usize,
) -> Vec<PackageGraphPath> {
    let mut outgoing: BTreeMap<&str, Vec<&crate::model::PackageDependency>> = BTreeMap::new();
    for edge in &project.package_edges {
        outgoing.entry(&edge.from).or_default().push(edge);
    }
    let mut paths = Vec::new();
    for first in &project.package_edges {
        let mut first_manifests = vec![first.from_manifest.clone()];
        append_manifest(&mut first_manifests, first.to_manifest.as_deref());
        append_manifest(&mut first_manifests, first.workspace_manifest.as_deref());
        let mut queue = VecDeque::from([(
            first.to.clone(),
            vec![first.dependency.clone()],
            first_manifests,
            BTreeSet::from([first.from.clone(), first.to.clone()]),
            1usize,
        )]);
        while let Some((current, dependencies, manifests, seen, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            let Some(next_edges) = outgoing.get(current.as_str()) else {
                continue;
            };
            for edge in next_edges {
                if seen.contains(&edge.to) {
                    continue;
                }
                let mut next_dependencies = dependencies.clone();
                next_dependencies.push(edge.dependency.clone());
                let mut next_manifests = manifests.clone();
                append_manifest(&mut next_manifests, Some(&edge.from_manifest));
                append_manifest(&mut next_manifests, edge.to_manifest.as_deref());
                append_manifest(&mut next_manifests, edge.workspace_manifest.as_deref());
                let next_depth = depth + 1;
                paths.push(PackageGraphPath {
                    from: first.from.clone(),
                    from_manifest: first.from_manifest.clone(),
                    to: edge.to.clone(),
                    to_manifest: edge.to_manifest.clone(),
                    dependencies: next_dependencies.clone(),
                    manifests: next_manifests.clone(),
                });
                let mut next_seen = seen.clone();
                next_seen.insert(edge.to.clone());
                queue.push_back((
                    edge.to.clone(),
                    next_dependencies,
                    next_manifests,
                    next_seen,
                    next_depth,
                ));
            }
        }
    }
    paths
}

pub(crate) fn package_edge_touched(
    edge: &crate::model::PackageDependency,
    changed: &BTreeSet<String>,
) -> bool {
    changed.contains(&edge.from_manifest)
        || edge
            .to_manifest
            .as_ref()
            .map(|manifest| changed.contains(manifest))
            .unwrap_or(false)
        || edge
            .workspace_manifest
            .as_ref()
            .map(|manifest| changed.contains(manifest))
            .unwrap_or(false)
}

fn append_manifest(manifests: &mut Vec<String>, manifest: Option<&str>) {
    if let Some(manifest) = manifest
        && !manifests.iter().any(|existing| existing == manifest)
    {
        manifests.push(manifest.to_string());
    }
}

pub(crate) fn package_edge_matches_rule(pattern: &str, package_path: &str) -> bool {
    let package_path = package_path.trim_end_matches('/');
    let probes = if package_path == "." {
        vec![
            "package.json".to_string(),
            "Cargo.toml".to_string(),
            "go.mod".to_string(),
            "pyproject.toml".to_string(),
            "src/__package_dependency__".to_string(),
            "__package_dependency__".to_string(),
        ]
    } else {
        vec![
            format!("{package_path}/package.json"),
            format!("{package_path}/Cargo.toml"),
            format!("{package_path}/go.mod"),
            format!("{package_path}/pyproject.toml"),
            format!("{package_path}/src/__package_dependency__"),
            format!("{package_path}/__package_dependency__"),
        ]
    };
    probes.iter().any(|probe| glob_match(pattern, probe))
}
