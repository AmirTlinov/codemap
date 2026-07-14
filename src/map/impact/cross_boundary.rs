// Responsibility: map-impact-cross-boundary
use crate::map::{
    contract_evidence, domain_by_rel, edge_with_path_location, import_edge,
    package_consumer_manifests, package_export_edges, package_for_rel,
    structural_edge_with_locations, unique,
};
use crate::model::{EvidenceStrength, Project, StructuralEdge};

pub(crate) fn cross_boundary_consumer_edges(
    project: &Project,
    rel: &str,
    direct_consumers: &[StructuralEdge],
    depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let changed_domain = domain_by_rel(project, rel).map(|domain| domain.path.clone());
    let changed_package = package_for_rel(project, rel).map(|package| package.path.clone());
    for edge in direct_consumers {
        let consumer_domain = domain_by_rel(project, &edge.from).map(|domain| domain.path.clone());
        let consumer_package =
            package_for_rel(project, &edge.from).map(|package| package.path.clone());
        if changed_domain != consumer_domain || changed_package != consumer_package {
            edges.push(structural_edge_with_locations(
                edge.from.clone(),
                rel.to_string(),
                "cross_boundary_consumer",
                "reverse_import_cross_boundary",
                EvidenceStrength::High,
                edge.locations.clone(),
            ));
        }
    }
    let package_seeds = package_consumer_seeds_for_impact(project, rel, direct_consumers);
    for manifest in package_consumer_manifests(project, &package_seeds, depth.max(1), usize::MAX) {
        edges.push(edge_with_path_location(
            manifest.clone(),
            rel.to_string(),
            "package_consumer",
            "package_manifest_reverse_dependency",
            EvidenceStrength::High,
            manifest,
            "package_manifest",
        ));
    }
    edges
}

fn package_consumer_seeds_for_impact(
    project: &Project,
    rel: &str,
    direct_consumers: &[StructuralEdge],
) -> Vec<String> {
    let mut seeds = vec![rel.to_string()];
    for consumer in direct_consumers {
        if let Some(file) = project.files.get(&consumer.from)
            && contract_evidence(file).is_some()
        {
            seeds.push(consumer.from.clone());
        }
    }
    unique(seeds)
}

pub(crate) fn contract_link_edges(
    project: &Project,
    rel: &str,
    direct_consumers: &[StructuralEdge],
) -> Vec<StructuralEdge> {
    let mut edges = package_export_edges(project, rel);
    if let Some(file) = project.files.get(rel) {
        if let Some(evidence) = contract_evidence(file) {
            edges.push(edge_with_path_location(
                rel.to_string(),
                rel.to_string(),
                "contract_changed",
                evidence,
                EvidenceStrength::High,
                rel.to_string(),
                "contract_file",
            ));
        }
        for target in &file.resolved_imports {
            if let Some(target_file) = project.files.get(target)
                && let Some(evidence) = contract_evidence(target_file)
            {
                edges.push(import_edge(
                    project,
                    rel.to_string(),
                    target.clone(),
                    "contract_dependency",
                    evidence,
                    EvidenceStrength::High,
                ));
            }
        }
    }
    for consumer in direct_consumers {
        if let Some(consumer_file) = project.files.get(&consumer.from)
            && let Some(evidence) = contract_evidence(consumer_file)
        {
            edges.push(structural_edge_with_locations(
                consumer.from.clone(),
                rel.to_string(),
                "contract_consumer",
                evidence,
                EvidenceStrength::High,
                consumer.locations.clone(),
            ));
        }
    }
    edges
}
