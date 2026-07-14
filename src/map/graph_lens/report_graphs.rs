// Responsibility: map-graph-lens-report-graphs
use super::{
    graph_edge_set, limit_graph_nodes, push_graph_edge, push_graph_edge_from_structural,
    structural_edges_for_nodes,
};
use crate::evidence::import_statement_locations;
use crate::map::{boundary_findings, impact_report, proof_report, unique};
use crate::model::{EvidenceLocation, EvidenceStrength, GraphEdge, HiddenGroup, Project};
use std::collections::BTreeSet;

pub(crate) fn impact_graph(
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
    let report = impact_report(
        project,
        changed.to_vec(),
        "--changed".to_string(),
        2,
        usize::MAX,
    );
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

pub(crate) fn proof_graph(
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
    let report = impact_report(
        project,
        changed.to_vec(),
        "--changed".to_string(),
        1,
        usize::MAX,
    );
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

pub(crate) fn boundary_graph(
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
