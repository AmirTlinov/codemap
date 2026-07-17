// Responsibility: map-cone-runtime-boundary-edges
use crate::map::{
    first_line_containing, runtime_fact_index_for_paths, runtime_routes_for_file, sort_edges,
    structural_edge_with_locations,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge, Unknown};
use crate::repo;
use std::path::Path;

pub(crate) fn owner_runtime_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = browser_extension_manifest_edges(project, rel);
    let Some(file) = project.files.get(rel) else {
        return edges;
    };
    if runtime_routes_for_file(project, file).is_empty() {
        return edges;
    }
    let facts = runtime_fact_index_for_paths(project, &[rel.to_string()]);
    edges.extend(
        facts
            .routes_for_file(rel)
            .into_iter()
            .flat_map(|route| facts.paths_for_route(&route)),
    );
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    edges
}

pub(crate) fn owner_runtime_incoming_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = project
        .files
        .keys()
        .filter(|candidate| candidate.ends_with("/manifest.json") || *candidate == "manifest.json")
        .flat_map(|manifest| browser_extension_manifest_edges(project, manifest))
        .filter(|edge| edge.to == rel)
        .collect::<Vec<_>>();
    sort_edges(&mut edges);
    edges
}

fn browser_extension_manifest_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    if Path::new(rel).file_name().and_then(|name| name.to_str()) != Some("manifest.json") {
        return Vec::new();
    }
    let Some(text) = project.read_indexed_text(rel) else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    if manifest
        .get("manifest_version")
        .and_then(|value| value.as_u64())
        .is_none()
    {
        return Vec::new();
    }
    let Some(worker) = manifest
        .get("background")
        .and_then(|value| value.get("service_worker"))
        .and_then(|value| value.as_str())
    else {
        return Vec::new();
    };
    let parent = Path::new(rel).parent().unwrap_or_else(|| Path::new("."));
    let target = repo::normalize_rel_path(&parent.join(worker).to_string_lossy());
    project
        .files
        .contains_key(&target)
        .then(|| {
            structural_edge_with_locations(
                rel.to_string(),
                target,
                "declares_worker",
                "browser_extension_manifest",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(
                    rel,
                    first_line_containing(project, rel, &["\"service_worker\""]).unwrap_or(1),
                    "browser_extension_service_worker",
                )],
            )
        })
        .into_iter()
        .collect()
}

pub(crate) fn owner_runtime_unknowns(project: &Project, rel: &str) -> Vec<Unknown> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    if runtime_routes_for_file(project, file).is_empty() {
        return Vec::new();
    }
    let facts = runtime_fact_index_for_paths(project, &[rel.to_string()]);
    facts
        .routes_for_file(rel)
        .into_iter()
        .flat_map(|route| facts.unknowns_for_route(&route))
        .collect()
}
