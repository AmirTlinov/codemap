use std::collections::BTreeSet;

use crate::evidence::{import_statement_locations, package_dependency_locations};
use crate::model::{
    Domain, EvidenceLocation, EvidenceStrength, GraphEdge, GraphLens, HiddenGroup, Project,
    StructuralEdge,
};

use super::{
    boundary_findings, direct_files_under_directory, directory_edges_at_depth, directory_has_files,
    immediate_child_dirs, impact_report, impacted_domains, is_generic_noise,
    is_support_artifact_path, path_under_scope, proof_report, shell_quote, unique,
};

pub fn graph_lens(
    project: &Project,
    path: Option<&str>,
    lens: &str,
    limit: usize,
    changed: Option<&[String]>,
) -> GraphLens {
    let limit = limit.max(1);
    let lens_key = lens.to_ascii_lowercase();
    let explicit_seed = path.and_then(|path| file_seed_for_path(project, path));
    let changed_selected = changed.is_some();
    let graph_changed = changed.or(explicit_seed.as_ref().map(std::slice::from_ref));
    let (nodes, edges, hidden) = match lens_key.as_str() {
        "boundary" | "boundaries" => {
            boundary_graph(project, graph_changed, limit, lens, path, changed_selected)
        }
        "impact" => impact_graph(
            project,
            graph_changed.unwrap_or(&[]),
            limit,
            lens,
            path,
            changed_selected,
        ),
        "proof" => proof_graph(project, path, changed, limit, lens, changed_selected),
        _ => causal_graph(project, path, limit, lens),
    };
    let domain = graph_output_domain(project, path, &nodes);
    GraphLens {
        kind: "graph_lens",
        schema_version: "4",
        domain: (&domain).into(),
        lens: lens.to_string(),
        nodes,
        edges,
        hidden,
    }
}

fn graph_output_domain(project: &Project, path: Option<&str>, nodes: &[String]) -> Domain {
    if let Some(path) = path
        && let Some(domain) = domain_for_path(project, path)
    {
        return domain.clone();
    }
    let domains = impacted_domains(
        project,
        &nodes
            .iter()
            .filter(|node| project.files.contains_key(*node))
            .cloned()
            .collect::<Vec<_>>(),
    );
    match domains.as_slice() {
        [only] => (*only).clone(),
        _ => project
            .domains
            .iter()
            .find(|domain| domain.path == ".")
            .cloned()
            .unwrap_or_else(|| Domain {
                id: "repo".to_string(),
                path: ".".to_string(),
                config_path: None,
            }),
    }
}

fn file_seed_for_path(project: &Project, path: &str) -> Option<String> {
    let rel = if std::path::Path::new(path).is_absolute() {
        let absolute = std::path::Path::new(path).canonicalize().ok()?;
        absolute
            .strip_prefix(&project.root)
            .ok()
            .map(|path| crate::repo::normalize_rel_path(&path.to_string_lossy()))?
    } else {
        crate::repo::normalize_rel_path(path)
    };
    project.files.contains_key(&rel).then_some(rel)
}

include!("graph_lens/causal.rs");

fn impact_graph(
    project: &Project,
    changed: &[String],
    limit: usize,
    lens: &str,
    path: Option<&str>,
    changed_selected: bool,
) -> (Vec<String>, Vec<GraphEdge>, Vec<HiddenGroup>) {
    if changed.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let report = impact_report(project, changed.to_vec(), 2, usize::MAX);
    let nodes = unique(
        report
            .changed
            .iter()
            .map(|file| file.path.clone())
            .chain(report.clusters.iter().flat_map(|cluster| {
                cluster
                    .direct_consumers
                    .iter()
                    .chain(cluster.cross_boundary_consumers.iter())
                    .chain(cluster.contract_links.iter())
                    .flat_map(|edge| [edge.from.clone(), edge.to.clone()])
            }))
            .collect(),
    );
    let (nodes, hidden) = limit_graph_nodes(nodes, limit, lens, path, changed_selected);
    let edges = structural_edges_for_nodes(project, &nodes);
    (nodes, edges, hidden)
}

fn proof_graph(
    project: &Project,
    path: Option<&str>,
    changed: Option<&[String]>,
    limit: usize,
    lens: &str,
    changed_selected: bool,
) -> (Vec<String>, Vec<GraphEdge>, Vec<HiddenGroup>) {
    if changed.is_none() {
        if let Some(path) = path {
            return proof_graph_for_path(project, path, limit, lens);
        }
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let changed = changed.unwrap_or(&[]);
    if changed.is_empty() {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let report = impact_report(project, changed.to_vec(), 1, usize::MAX);
    let nodes = unique(
        report
            .clusters
            .iter()
            .flat_map(|cluster| {
                cluster
                    .proof
                    .iter()
                    .flat_map(|edge| [edge.from.clone(), edge.to.clone()])
            })
            .collect(),
    );
    let (nodes, hidden) = limit_graph_nodes(nodes, limit, lens, path, changed_selected);
    let mut edges = structural_edges_for_nodes(project, &nodes);
    let mut seen = graph_edge_set(&edges);
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    for cluster in report.clusters {
        for edge in cluster.proof {
            if node_set.contains(&edge.from) && node_set.contains(&edge.to) {
                push_graph_edge_from_structural(&mut edges, &mut seen, edge);
            }
        }
    }
    (nodes, edges, hidden)
}

fn proof_graph_for_path(
    project: &Project,
    path: &str,
    limit: usize,
    lens: &str,
) -> (Vec<String>, Vec<GraphEdge>, Vec<HiddenGroup>) {
    let target = crate::repo::normalize_rel_path(path);
    let report = proof_report(
        project,
        Some(target.clone()),
        Vec::new(),
        target.clone(),
        1,
        usize::MAX,
    );
    let nodes = unique(
        std::iter::once(target.clone())
            .chain(report.proofs.iter().filter_map(|proof| proof.path.clone()))
            .collect(),
    );
    let (nodes, hidden) = limit_graph_nodes(nodes, limit, lens, Some(path), false);
    let mut edges = structural_edges_for_nodes(project, &nodes);
    let mut seen = graph_edge_set(&edges);
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    for proof in report.proofs {
        if let Some(path) = proof.path
            && node_set.contains(&path)
            && node_set.contains(&target)
        {
            push_graph_edge(
                &mut edges,
                &mut seen,
                GraphEdge {
                    from: path,
                    to: target.clone(),
                    edge_type: "tests".to_string(),
                    evidence: proof.evidence,
                    strength: proof.strength,
                    locations: proof.locations,
                },
            );
        }
    }
    (nodes, edges, hidden)
}

fn boundary_graph(
    project: &Project,
    changed: Option<&[String]>,
    limit: usize,
    lens: &str,
    path: Option<&str>,
    changed_selected: bool,
) -> (Vec<String>, Vec<GraphEdge>, Vec<HiddenGroup>) {
    let changed_set = changed.map(|files| files.iter().cloned().collect::<BTreeSet<_>>());
    let findings = boundary_findings(project, changed_set.as_ref());
    let nodes = unique(
        findings
            .iter()
            .flat_map(|finding| [finding.from.clone(), finding.to.clone()])
            .filter(|value| !value.is_empty())
            .collect(),
    );
    let (nodes, hidden) = limit_graph_nodes(nodes, limit, lens, path, changed_selected);
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    for finding in findings {
        if finding.from.is_empty()
            || finding.to.is_empty()
            || !node_set.contains(&finding.from)
            || !node_set.contains(&finding.to)
        {
            continue;
        }
        let locations = boundary_finding_locations(project, &finding.from, &finding.to);
        push_graph_edge(
            &mut edges,
            &mut seen,
            GraphEdge {
                from: finding.from,
                to: finding.to,
                edge_type: finding.status,
                evidence: finding.provenance,
                strength: evidence_strength_from_str(&finding.strength),
                locations,
            },
        );
    }
    (nodes, edges, hidden)
}

fn limit_graph_nodes(
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

fn structural_edges_for_nodes(project: &Project, nodes: &[String]) -> Vec<GraphEdge> {
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

fn push_unique_nodes<I>(nodes: &mut Vec<String>, values: I, limit: usize)
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

fn graph_edge_set(edges: &[GraphEdge]) -> BTreeSet<(String, String, String)> {
    edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone(), edge.edge_type.clone()))
        .collect()
}

fn push_graph_edge(
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

fn push_graph_edge_from_structural(
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

fn graph_surface_location(node: &str) -> Vec<EvidenceLocation> {
    let kind = if node == "." {
        "current_level_scope"
    } else if node.ends_with('/') {
        "current_level_directory"
    } else {
        "current_level_file"
    };
    vec![EvidenceLocation::path(node, kind)]
}

fn boundary_finding_locations(project: &Project, from: &str, to: &str) -> Vec<EvidenceLocation> {
    if project
        .files
        .get(from)
        .is_some_and(|file| file.resolved_imports.contains(to))
    {
        return import_statement_locations(project, from, to);
    }
    if !from.is_empty() {
        return vec![EvidenceLocation::path(from, "boundary_finding")];
    }
    vec![EvidenceLocation::aggregate("boundary_finding")]
}

fn evidence_strength_from_str(value: &str) -> EvidenceStrength {
    match value {
        "hard" => EvidenceStrength::Hard,
        "high" => EvidenceStrength::High,
        "low" => EvidenceStrength::Low,
        _ => EvidenceStrength::Medium,
    }
}

fn domain_for_path<'a>(project: &'a Project, path: &str) -> Option<&'a Domain> {
    let rel = crate::repo::normalize_rel_path(path);
    project
        .domains
        .iter()
        .filter(|domain| {
            domain.path == "."
                || rel == domain.path
                || rel.starts_with(&format!("{}/", domain.path))
        })
        .max_by_key(|domain| {
            if domain.path == "." {
                0
            } else {
                domain.path.len()
            }
        })
}
