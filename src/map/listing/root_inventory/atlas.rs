// Responsibility: map-listing-root-inventory-atlas
use crate::map::{inventory_file_kind, inventory_push, is_support_artifact_path};
use crate::model::{EvidenceLocation, EvidenceStrength, PackageInfo, StructuralEdge};
use crate::repo::{detect_package_edges_from_paths, is_source_ext};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Default)]
pub(crate) struct RootAtlasProjection {
    pub(crate) grouped: BTreeMap<String, BTreeSet<String>>,
    pub(crate) edges: Vec<StructuralEdge>,
}

pub(crate) fn root_atlas_projection(
    root: &Path,
    files: &[String],
    packages: &[PackageInfo],
) -> RootAtlasProjection {
    let packages = current_level_packages(packages);
    let mut atlas = RootAtlasProjection::default();
    let package_coordinates = packages
        .iter()
        .map(|package| (package.path.clone(), package_coordinate(package)))
        .collect::<BTreeMap<_, _>>();
    let mut domains = BTreeSet::new();
    for package in &packages {
        let coordinate = package_coordinate(package);
        let kind = if is_support_artifact_path(&package.path)
            || is_support_artifact_path(&package.manifest)
        {
            format!("support_package:{}", package.ecosystem)
        } else {
            format!("package:{}", package.ecosystem)
        };
        inventory_push(&mut atlas.grouped, &kind, &coordinate);
        if let Some(domain) = package_domain(&package.path) {
            domains.insert(domain);
        }
    }
    for rel in files {
        if let Some(domain) = semantic_top_level_domain(rel) {
            domains.insert(domain);
        }
        collect_file_containers(rel, &packages, &mut atlas.grouped);
    }
    for domain in &domains {
        inventory_push(&mut atlas.grouped, "domain", domain);
    }
    atlas.edges.extend(package_dependency_edges(
        root,
        files,
        &packages,
        &package_coordinates,
    ));
    atlas
        .edges
        .extend(atlas_containment_edges(&atlas.grouped, &packages, &domains));
    atlas.edges.sort_by(|a, b| {
        atlas_edge_priority(a)
            .cmp(&atlas_edge_priority(b))
            .then_with(|| a.from.cmp(&b.from))
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.edge_type.cmp(&b.edge_type))
    });
    atlas
        .edges
        .dedup_by(|a, b| a.from == b.from && a.to == b.to && a.edge_type == b.edge_type);
    atlas
}

fn current_level_packages(packages: &[PackageInfo]) -> Vec<PackageInfo> {
    packages
        .iter()
        .filter(|package| {
            !packages.iter().any(|owner| {
                owner.path != "."
                    && owner.path != package.path
                    && path_under(&package.path, &owner.path)
            })
        })
        .cloned()
        .collect()
}

fn collect_file_containers(
    rel: &str,
    packages: &[PackageInfo],
    grouped: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let kind = inventory_file_kind(rel);
    if let Some(container) = deployment_container(rel) {
        inventory_push(grouped, "deployment_container", &container);
    }
    if let Some(container) = contract_container(rel, &kind) {
        inventory_push(grouped, "contract_container", &container);
    }
    if let Some(container) = data_container(rel, &kind) {
        inventory_push(grouped, "data_container", &container);
    }
    if let Some(container) = verification_container(rel) {
        inventory_push(grouped, "verification_container", &container);
    }
    if let Some(container) = runtime_container(rel, &kind, packages) {
        inventory_push(grouped, "runtime_container", &container);
    }
}

fn runtime_container(rel: &str, kind: &str, packages: &[PackageInfo]) -> Option<String> {
    let ext = Path::new(rel)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if !is_source_ext(ext) && kind != "runtime_config" {
        return None;
    }
    if deployment_container(rel).is_some() || verification_container(rel).is_some() {
        return None;
    }
    if let Some(package) = nearest_nested_package(packages, rel) {
        return Some(directory_coordinate(&package.path));
    }
    let first = rel.split('/').next()?;
    matches!(
        first.to_ascii_lowercase().as_str(),
        "src"
            | "app"
            | "apps"
            | "cmd"
            | "server"
            | "servers"
            | "service"
            | "services"
            | "worker"
            | "workers"
            | "web"
            | "frontend"
            | "backend"
    )
    .then(|| directory_coordinate(first))
}

fn contract_container(rel: &str, kind: &str) -> Option<String> {
    container_through_marker(
        rel,
        &[
            "contract",
            "contracts",
            "schema",
            "schemas",
            "openapi",
            "proto",
            "protobuf",
            "prisma",
        ],
    )
    .or_else(|| (kind == "schema_contract").then(|| parent_coordinate(rel)))
}

fn data_container(rel: &str, kind: &str) -> Option<String> {
    container_through_marker(
        rel,
        &[
            "data",
            "database",
            "databases",
            "db",
            "migration",
            "migrations",
            "prisma",
            "storage",
        ],
    )
    .or_else(|| (kind == "migration").then(|| parent_coordinate(rel)))
}

fn deployment_container(rel: &str) -> Option<String> {
    container_through_marker(
        rel,
        &[
            ".argocd",
            ".buildkite",
            ".circleci",
            ".github",
            "deploy",
            "deployment",
            "deployments",
            "helm",
            "infra",
            "infrastructure",
            "k8s",
            "kubernetes",
            "ops",
            "terraform",
        ],
    )
}

fn verification_container(rel: &str) -> Option<String> {
    container_through_marker(
        rel,
        &["e2e", "integration", "spec", "specs", "test", "tests"],
    )
    .or_else(|| {
        let name = Path::new(rel).file_name()?.to_str()?.to_ascii_lowercase();
        (name.contains(".test.") || name.contains(".spec.")).then(|| parent_coordinate(rel))
    })
}

fn container_through_marker(rel: &str, markers: &[&str]) -> Option<String> {
    let parts = rel.split('/').collect::<Vec<_>>();
    let index = parts.iter().enumerate().position(|(index, part)| {
        let normalized = part.to_ascii_lowercase();
        index + 1 < parts.len() && markers.iter().any(|marker| normalized == *marker)
    })?;
    Some(directory_coordinate(&parts[..=index].join("/")))
}

fn package_dependency_edges(
    root: &Path,
    files: &[String],
    packages: &[PackageInfo],
    coordinates: &BTreeMap<String, String>,
) -> Vec<StructuralEdge> {
    detect_package_edges_from_paths(root, files, packages)
        .into_iter()
        .filter_map(|edge| {
            let from = coordinates.get(&edge.from)?.clone();
            let to = coordinates.get(&edge.to)?.clone();
            (from != to).then(|| StructuralEdge {
                from,
                to,
                edge_type: "package_internal".to_string(),
                evidence: format!("package_manifest:{}", edge.dependency),
                strength: EvidenceStrength::Hard,
                locations: vec![EvidenceLocation::path(
                    &edge.from_manifest,
                    format!("package_dependency:{}", edge.dependency_kind),
                )],
            })
        })
        .collect()
}

fn atlas_containment_edges(
    grouped: &BTreeMap<String, BTreeSet<String>>,
    packages: &[PackageInfo],
    domains: &BTreeSet<String>,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for package in packages {
        let Some(domain) = package_domain(&package.path) else {
            continue;
        };
        let coordinate = package_coordinate(package);
        if coordinate != domain {
            edges.push(atlas_edge(
                domain,
                coordinate,
                "domain_contains_package",
                &package.manifest,
            ));
        }
    }
    for (kind, containers) in grouped.iter().filter(|(kind, _)| {
        matches!(
            kind.as_str(),
            "runtime_container"
                | "contract_container"
                | "data_container"
                | "deployment_container"
                | "verification_container"
        )
    }) {
        for container in containers {
            if let Some(package) = nearest_nested_package(packages, container) {
                let package = directory_coordinate(&package.path);
                if package != *container {
                    edges.push(atlas_edge(
                        package,
                        container.clone(),
                        &format!("package_contains_{}", kind.trim_end_matches("_container")),
                        container,
                    ));
                    continue;
                }
            }
            if let Some(domain) = domains
                .iter()
                .filter(|domain| path_under(container, domain))
                .max_by_key(|domain| domain.len())
                && domain != container
            {
                edges.push(atlas_edge(
                    domain.clone(),
                    container.clone(),
                    &format!("domain_contains_{}", kind.trim_end_matches("_container")),
                    container,
                ));
            }
        }
    }
    edges
}

fn atlas_edge(from: String, to: String, edge_type: &str, path: &str) -> StructuralEdge {
    StructuralEdge {
        from,
        to,
        edge_type: edge_type.to_string(),
        evidence: "atlas_path_ownership".to_string(),
        strength: EvidenceStrength::High,
        locations: vec![EvidenceLocation::path(path, "atlas_container")],
    }
}

fn atlas_edge_priority(edge: &StructuralEdge) -> usize {
    match edge.edge_type.as_str() {
        "package_internal" => 0,
        "domain_contains_package" => 1,
        value if value.starts_with("package_contains_") => 2,
        value if value.starts_with("domain_contains_") => 3,
        _ => 9,
    }
}

fn nearest_nested_package<'a>(packages: &'a [PackageInfo], rel: &str) -> Option<&'a PackageInfo> {
    packages
        .iter()
        .filter(|package| package.path != "." && path_under(rel, &package.path))
        .max_by_key(|package| package.path.len())
}

fn package_coordinate(package: &PackageInfo) -> String {
    if package.path == "." {
        package.manifest.clone()
    } else {
        directory_coordinate(&package.path)
    }
}

fn package_domain(path: &str) -> Option<String> {
    (path != ".")
        .then(|| path.split('/').next())
        .flatten()
        .map(directory_coordinate)
}

fn semantic_top_level_domain(rel: &str) -> Option<String> {
    let first = rel.split('/').next()?;
    matches!(
        first.to_ascii_lowercase().as_str(),
        "apps"
            | "components"
            | "crates"
            | "domains"
            | "libraries"
            | "libs"
            | "modules"
            | "packages"
            | "plugins"
            | "services"
            | "workers"
    )
    .then(|| directory_coordinate(first))
}

fn parent_coordinate(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .and_then(|path| path.to_str())
        .filter(|path| !path.is_empty())
        .map(directory_coordinate)
        .unwrap_or_else(|| rel.to_string())
}

fn directory_coordinate(path: &str) -> String {
    let path = path.trim_end_matches('/');
    if path == "." || path.is_empty() {
        ".".to_string()
    } else {
        format!("{path}/")
    }
}

fn path_under(path: &str, scope: &str) -> bool {
    let path = path.trim_end_matches('/');
    let scope = scope.trim_end_matches('/');
    path == scope || path.starts_with(&format!("{scope}/"))
}
