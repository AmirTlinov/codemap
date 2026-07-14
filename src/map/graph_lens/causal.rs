// Responsibility: map-graph-lens-causal
use super::{
    file_seed_for_path, graph_surface_location, limit_graph_nodes, push_graph_edge,
    push_graph_edge_from_structural, push_unique_nodes, structural_edges_for_nodes,
};
use crate::map::{
    direct_files_under_directory, directory_edges_at_depth, directory_has_files,
    immediate_child_dirs, is_generic_noise, is_support_artifact_path, path_under_scope,
};
use crate::model::{EvidenceStrength, GraphEdge, HiddenGroup, Project};
use std::collections::BTreeSet;

pub(crate) fn causal_graph(
    project: &Project,
    path: Option<&str>,
    limit: usize,
    lens: &str,
) -> (Vec<String>, Vec<GraphEdge>, Vec<HiddenGroup>) {
    if let Some(seed) = path.and_then(|path| file_seed_for_path(project, path)) {
        let mut nodes = Vec::new();
        push_unique_nodes(&mut nodes, [seed.clone()], usize::MAX);
        if let Some(file) = project.files.get(&seed) {
            push_unique_nodes(
                &mut nodes,
                file.resolved_imports.iter().cloned(),
                usize::MAX,
            );
        }
        if let Some(importers) = project.reverse_imports.get(&seed) {
            push_unique_nodes(&mut nodes, importers.iter().cloned(), usize::MAX);
        }
        let (nodes, hidden) = limit_graph_nodes(nodes, limit, lens, path, false);
        let edges = structural_edges_for_nodes(project, &nodes);
        return (nodes, edges, hidden);
    }

    let rel = path
        .map(crate::repo::normalize_rel_path)
        .unwrap_or_else(|| ".".to_string());
    if directory_has_files(project, &rel) {
        return directory_causal_graph(project, &rel, limit, lens, path);
    }

    let nodes = Vec::new();
    let (nodes, hidden) = limit_graph_nodes(nodes, limit, lens, path, false);
    let edges = structural_edges_for_nodes(project, &nodes);
    (nodes, edges, hidden)
}

fn directory_causal_graph(
    project: &Project,
    rel: &str,
    limit: usize,
    lens: &str,
    path: Option<&str>,
) -> (Vec<String>, Vec<GraphEdge>, Vec<HiddenGroup>) {
    let mut nodes = Vec::new();
    let mut graph_edges = Vec::new();
    let mut seen_edges = BTreeSet::new();
    let scope_node = directory_scope_node(rel);
    push_unique_nodes(&mut nodes, [scope_node.clone()], usize::MAX);

    for edge in directory_edges_at_depth(project, rel, false, 1) {
        push_unique_nodes(&mut nodes, [edge.from.clone(), edge.to.clone()], usize::MAX);
        push_graph_edge_from_structural(&mut graph_edges, &mut seen_edges, edge);
    }

    let surface_nodes = directory_surface_nodes(project, rel);
    for node in &surface_nodes {
        push_graph_edge(
            &mut graph_edges,
            &mut seen_edges,
            GraphEdge {
                from: scope_node.clone(),
                to: node.clone(),
                edge_type: "contains".to_string(),
                evidence: "current_level_surface".to_string(),
                strength: EvidenceStrength::Medium,
                locations: graph_surface_location(node),
            },
        );
    }
    push_unique_nodes(&mut nodes, surface_nodes, usize::MAX);
    let (nodes, hidden) = limit_graph_nodes(nodes, limit, lens, path, false);
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    graph_edges.retain(|edge| node_set.contains(&edge.from) && node_set.contains(&edge.to));

    (nodes, graph_edges, hidden)
}

fn directory_scope_node(rel: &str) -> String {
    let rel = crate::repo::normalize_rel_path(rel);
    if rel == "." {
        rel
    } else {
        format!("{}/", rel.trim_end_matches('/'))
    }
}

fn directory_surface_nodes(project: &Project, rel: &str) -> Vec<String> {
    let mut nodes = Vec::new();
    let scope_is_support = is_support_artifact_path(rel);

    push_unique_nodes(
        &mut nodes,
        project
            .packages
            .iter()
            .filter(|package| {
                (path_under_scope(&package.path, rel) || path_under_scope(&package.manifest, rel))
                    && (scope_is_support
                        || (!is_support_artifact_path(&package.path)
                            && !is_support_artifact_path(&package.manifest)))
            })
            .map(|package| package.manifest.clone()),
        usize::MAX,
    );

    push_unique_nodes(
        &mut nodes,
        project
            .domains
            .iter()
            .filter(|domain| {
                domain.path != "."
                    && path_under_scope(&domain.path, rel)
                    && (scope_is_support || !is_support_artifact_path(&domain.path))
            })
            .map(|domain| directory_scope_node(&domain.path)),
        usize::MAX,
    );

    push_unique_nodes(
        &mut nodes,
        immediate_child_dirs(project, rel)
            .into_iter()
            .filter(|dir| scope_is_support || !is_support_artifact_path(dir)),
        usize::MAX,
    );

    push_unique_nodes(
        &mut nodes,
        direct_files_under_directory(project, rel)
            .into_iter()
            .filter(|file| {
                (scope_is_support || !is_support_artifact_path(&file.rel))
                    && !is_generic_noise(file)
            })
            .map(|file| file.rel.clone()),
        usize::MAX,
    );

    nodes
}
