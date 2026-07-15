// Responsibility: map-cone-runtime-boundary-edges
use crate::map::{runtime_fact_index_for_paths, runtime_routes_for_file, sort_edges};
use crate::model::{Project, StructuralEdge, Unknown};

pub(crate) fn owner_runtime_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    if runtime_routes_for_file(project, file).is_empty() {
        return Vec::new();
    }
    let facts = runtime_fact_index_for_paths(project, &[rel.to_string()]);
    let mut edges = facts
        .routes_for_file(rel)
        .into_iter()
        .flat_map(|route| facts.paths_for_route(&route))
        .collect::<Vec<_>>();
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    edges
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
