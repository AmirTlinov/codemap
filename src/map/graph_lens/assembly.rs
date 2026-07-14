// Responsibility: map-graph-lens-assembly
use crate::evidence::{import_statement_locations, package_dependency_locations};
use crate::map::shell_quote;
use crate::model::{
    EvidenceLocation, EvidenceStrength, GraphEdge, HiddenGroup, Project, StructuralEdge,
};
use std::collections::BTreeSet;

pub(crate) fn limit_graph_nodes(
    mut nodes: Vec<String>,
    limit: usize,
    lens: &str,
    path: Option<&str>,
    changed: bool,
) -> (Vec<String>, Vec<HiddenGroup>) {
    if nodes.len() <= limit {
        return (nodes, Vec::new());
    }
    let total = nodes.len();
    nodes.truncate(limit);
    (
        nodes,
        vec![HiddenGroup {
            reason: "graph nodes hidden by limit".to_string(),
            count: total - limit,
            expand: graph_expand_command(lens, path, changed, total),
        }],
    )
}

fn graph_expand_command(lens: &str, path: Option<&str>, changed: bool, limit: usize) -> String {
    let mut command = format!("codemap graph --lens {}", shell_quote(lens));
    if let Some(path) = path {
        command.push_str(" --path ");
        command.push_str(&shell_quote(path));
    }
    if changed {
        command.push_str(" --changed");
    }
    command.push_str(&format!(" --limit {limit}"));
    command
}

pub(crate) fn structural_edges_for_nodes(project: &Project, nodes: &[String]) -> Vec<GraphEdge> {
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    for node in nodes {
        if let Some(file) = project.files.get(node) {
            for target in &file.resolved_imports {
                if node_set.contains(target) {
                    push_graph_edge(
                        &mut edges,
                        &mut seen,
                        GraphEdge {
                            from: node.clone(),
                            to: target.clone(),
                            edge_type: "imports".to_string(),
                            evidence: "resolved_import".to_string(),
                            strength: EvidenceStrength::High,
                            locations: import_statement_locations(project, node, target),
                        },
                    );
                }
            }
        }
    }
    for edge in &project.package_edges {
        let from = edge.from_manifest.clone();
        let to = edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone());
        if node_set.contains(&from) && node_set.contains(&to) {
            push_graph_edge(
                &mut edges,
                &mut seen,
                GraphEdge {
                    from,
                    to,
                    edge_type: "package_depends".to_string(),
                    evidence: edge.source.clone(),
                    strength: EvidenceStrength::Hard,
                    locations: package_dependency_locations(project, edge),
                },
            );
        }
    }
    edges
}

pub(crate) fn push_unique_nodes<I>(nodes: &mut Vec<String>, values: I, limit: usize)
where
    I: IntoIterator<Item = String>,
{
    let mut seen = nodes.iter().cloned().collect::<BTreeSet<_>>();
    for value in values {
        if nodes.len() >= limit {
            break;
        }
        if !value.is_empty() && seen.insert(value.clone()) {
            nodes.push(value);
        }
    }
}

pub(crate) fn graph_edge_set(edges: &[GraphEdge]) -> BTreeSet<(String, String, String)> {
    edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone(), edge.edge_type.clone()))
        .collect()
}

pub(crate) fn push_graph_edge(
    edges: &mut Vec<GraphEdge>,
    seen: &mut BTreeSet<(String, String, String)>,
    edge: GraphEdge,
) {
    if edge.from == edge.to {
        return;
    }
    if seen.insert((edge.from.clone(), edge.to.clone(), edge.edge_type.clone())) {
        edges.push(edge);
    }
}

pub(crate) fn push_graph_edge_from_structural(
    edges: &mut Vec<GraphEdge>,
    seen: &mut BTreeSet<(String, String, String)>,
    edge: StructuralEdge,
) {
    push_graph_edge(
        edges,
        seen,
        GraphEdge {
            from: edge.from,
            to: edge.to,
            edge_type: edge.edge_type,
            evidence: edge.evidence,
            strength: edge.strength,
            locations: edge.locations,
        },
    );
}

pub(crate) fn graph_surface_location(node: &str) -> Vec<EvidenceLocation> {
    let kind = if node == "." {
        "current_level_scope"
    } else if node.ends_with('/') {
        "current_level_directory"
    } else {
        "current_level_file"
    };
    vec![EvidenceLocation::path(node, kind)]
}
