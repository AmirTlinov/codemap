// Responsibility: map-listing-root-inventory-graph
use crate::map::{root_inventory_ls_report, shell_quote};
use crate::model::{
    DirectorySurface, Domain, EvidenceLocation, GraphEdge, GraphLens, HiddenGroup, LsReport,
    StructuralEdge,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn root_inventory_graph_lens(
    root: &Path,
    files: &[String],
    limit: usize,
    lens: &str,
) -> GraphLens {
    let report = root_inventory_ls_report(root, files, false, usize::MAX / 2);
    let (nodes, edges, hidden) = root_inventory_graph_projection(report, limit, lens);
    GraphLens {
        kind: "graph_lens",
        schema_version: "6",
        domain: (&Domain {
            id: "repo".to_string(),
            path: ".".to_string(),
            config_path: None,
        })
            .into(),
        lens: lens.to_string(),
        nodes,
        edges,
        hidden,
    }
}

pub(crate) fn root_inventory_graph_projection(
    report: LsReport,
    limit: usize,
    lens: &str,
) -> (Vec<String>, Vec<GraphEdge>, Vec<HiddenGroup>) {
    let mut graph_edges = Vec::new();
    let mut seen_edges = BTreeSet::new();

    for surface in &report.directory {
        for node in inventory_graph_surface_examples(surface) {
            inventory_graph_push_edge(
                &mut graph_edges,
                &mut seen_edges,
                GraphEdge {
                    from: ".".to_string(),
                    to: node.clone(),
                    edge_type: "contains".to_string(),
                    evidence: "current_level_inventory_surface".to_string(),
                    strength: surface.strength,
                    locations: vec![inventory_graph_surface_location(&node, &surface.evidence)],
                },
            );
        }
    }

    let mut nodes = inventory_graph_node_order(&report, &graph_edges);

    for edge in report.edges {
        inventory_graph_push_edge(
            &mut graph_edges,
            &mut seen_edges,
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

    let total_nodes = nodes.len();
    let limit = limit.max(1);
    if nodes.len() > limit {
        nodes.truncate(limit);
    }
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    graph_edges.retain(|edge| node_set.contains(&edge.from) && node_set.contains(&edge.to));

    let mut hidden = Vec::new();
    if total_nodes > nodes.len() {
        hidden.push(HiddenGroup {
            reason: "graph nodes hidden by limit".to_string(),
            count: total_nodes - nodes.len(),
            expand: format!(
                "codemap graph --lens {} --limit {total_nodes}",
                shell_quote(lens)
            ),
        });
    }
    hidden.extend(report.hidden.into_iter().filter(|group| {
        group.reason == "full-index source edges hidden by bounded root inventory"
            || group.reason == "inventory edges hidden by limit"
            || group.reason == "support artifacts hidden"
    }));

    (nodes, graph_edges, hidden)
}

fn inventory_graph_node_order(report: &LsReport, graph_edges: &[GraphEdge]) -> Vec<String> {
    let mut nodes = Vec::new();
    inventory_graph_push_node(&mut nodes, ".".to_string());
    for surface in report
        .directory
        .iter()
        .filter(|surface| inventory_graph_is_atlas_surface(&surface.kind))
    {
        if let Some(node) = inventory_graph_surface_examples(surface).into_iter().next() {
            inventory_graph_push_node(&mut nodes, node);
        }
    }
    for node in inventory_graph_edge_node_order(
        &report
            .edges
            .iter()
            .filter(|edge| inventory_graph_is_atlas_edge(&edge.edge_type))
            .cloned()
            .collect::<Vec<_>>(),
    ) {
        inventory_graph_push_node(&mut nodes, node);
    }
    for surface in report
        .directory
        .iter()
        .filter(|surface| !inventory_graph_is_atlas_surface(&surface.kind))
    {
        if let Some(node) = inventory_graph_surface_examples(surface).into_iter().next() {
            inventory_graph_push_node(&mut nodes, node);
        }
    }
    let (_, secondary_surfaces) = inventory_graph_surface_node_rounds(&report.directory);
    for node in secondary_surfaces {
        inventory_graph_push_node(&mut nodes, node);
    }
    for node in inventory_graph_edge_node_order(&report.edges) {
        inventory_graph_push_node(&mut nodes, node);
    }
    for edge in graph_edges {
        inventory_graph_push_node(&mut nodes, edge.from.clone());
        inventory_graph_push_node(&mut nodes, edge.to.clone());
    }
    nodes
}

fn inventory_graph_is_atlas_surface(kind: &str) -> bool {
    kind == "domain"
        || kind.starts_with("package:")
        || matches!(
            kind,
            "runtime_container"
                | "contract_container"
                | "data_container"
                | "deployment_container"
                | "verification_container"
        )
}

fn inventory_graph_is_atlas_edge(edge_type: &str) -> bool {
    edge_type == "package_internal"
        || edge_type == "domain_contains_package"
        || edge_type.starts_with("package_contains_")
        || edge_type.starts_with("domain_contains_")
}

fn inventory_graph_surface_node_rounds(
    surfaces: &[DirectorySurface],
) -> (Vec<String>, Vec<String>) {
    let mut by_surface = surfaces
        .iter()
        .map(inventory_graph_surface_examples)
        .filter(|examples| !examples.is_empty())
        .collect::<Vec<_>>();
    let mut primary = Vec::new();
    let mut secondary = Vec::new();
    let mut round = 0usize;
    loop {
        let mut pushed = false;
        for examples in &mut by_surface {
            if let Some(node) = examples.get(round) {
                if round == 0 {
                    primary.push(node.clone());
                } else {
                    secondary.push(node.clone());
                }
                pushed = true;
            }
        }
        if !pushed {
            break;
        }
        round += 1;
    }
    (primary, secondary)
}

fn inventory_graph_surface_examples(surface: &DirectorySurface) -> Vec<String> {
    let mut examples = surface.examples.clone();
    if surface.kind == "script" {
        examples.retain(|example| inventory_graph_example_is_path_like(example));
    }
    if surface.kind != "dir" {
        examples.sort_by(|a, b| {
            inventory_graph_surface_example_priority(a)
                .cmp(&inventory_graph_surface_example_priority(b))
                .then_with(|| a.cmp(b))
        });
    }
    examples
}

fn inventory_graph_surface_example_priority(example: &str) -> usize {
    if example.ends_with('/') {
        2
    } else if inventory_graph_example_is_path_like(example) {
        0
    } else {
        1
    }
}

fn inventory_graph_example_is_path_like(example: &str) -> bool {
    example.contains('/') || !example.contains(':')
}

fn inventory_graph_edge_node_order(edges: &[StructuralEdge]) -> Vec<String> {
    let mut sources = Vec::new();
    let mut by_source: BTreeMap<String, Vec<&StructuralEdge>> = BTreeMap::new();
    for edge in edges {
        if !by_source.contains_key(&edge.from) {
            sources.push(edge.from.clone());
        }
        by_source.entry(edge.from.clone()).or_default().push(edge);
    }

    let mut nodes = Vec::new();
    let mut round = 0usize;
    loop {
        let mut pushed = false;
        for source in &sources {
            let Some(source_edges) = by_source.get(source) else {
                continue;
            };
            if let Some(edge) = source_edges.get(round) {
                nodes.push(edge.from.clone());
                nodes.push(edge.to.clone());
                pushed = true;
            }
        }
        if !pushed {
            break;
        }
        round += 1;
    }
    nodes
}

fn inventory_graph_push_node(nodes: &mut Vec<String>, node: String) {
    if !node.is_empty() && !nodes.contains(&node) {
        nodes.push(node);
    }
}

fn inventory_graph_push_edge(
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

fn inventory_graph_surface_location(node: &str, kind: &str) -> EvidenceLocation {
    if node.ends_with('/') {
        EvidenceLocation::path(node, "directory_inventory")
    } else if node.contains(':') && !node.contains('/') {
        EvidenceLocation::aggregate(kind)
    } else {
        EvidenceLocation::path(node, kind)
    }
}
