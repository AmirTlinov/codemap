use std::collections::BTreeSet;

use crate::model::{Domain, GraphEdge, GraphLens, Project};

use super::{boundary_findings, impact_report, impacted_domains, unique};

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
    let graph_changed = changed.or(explicit_seed.as_ref().map(std::slice::from_ref));
    let (nodes, edges) = match lens_key.as_str() {
        "boundary" | "boundaries" => boundary_graph(project, graph_changed, limit),
        "impact" => impact_graph(project, graph_changed.unwrap_or(&[]), limit),
        "proof" => proof_graph(project, graph_changed, limit),
        _ => causal_graph(project, path, limit),
    };
    let domain = graph_output_domain(project, path, &nodes);
    GraphLens {
        kind: "graph_lens",
        schema_version: "2",
        domain: (&domain).into(),
        lens: lens.to_string(),
        nodes,
        edges,
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

fn causal_graph(
    project: &Project,
    path: Option<&str>,
    limit: usize,
) -> (Vec<String>, Vec<GraphEdge>) {
    let mut nodes = Vec::new();
    if let Some(seed) = path.and_then(|path| file_seed_for_path(project, path)) {
        push_unique_nodes(&mut nodes, [seed.clone()], limit);
        if let Some(file) = project.files.get(&seed) {
            push_unique_nodes(&mut nodes, file.resolved_imports.iter().cloned(), limit);
        }
        if let Some(importers) = project.reverse_imports.get(&seed) {
            push_unique_nodes(&mut nodes, importers.iter().cloned(), limit);
        }
    } else if let Some(path) = path {
        let prefix = directory_prefix(path);
        push_unique_nodes(
            &mut nodes,
            project
                .files
                .keys()
                .filter(|rel| prefix_matches(rel, prefix.as_deref()))
                .take(limit)
                .cloned(),
            limit,
        );
    } else {
        push_unique_nodes(
            &mut nodes,
            project
                .packages
                .iter()
                .map(|package| package.manifest.clone())
                .chain(project.domains.iter().map(|domain| domain.path.clone())),
            limit,
        );
    }
    let edges = structural_edges_for_nodes(project, &nodes);
    (nodes, edges)
}

fn impact_graph(
    project: &Project,
    changed: &[String],
    limit: usize,
) -> (Vec<String>, Vec<GraphEdge>) {
    if changed.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let report = impact_report(project, changed.to_vec(), 2, limit);
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
                    .chain(cluster.contract_risks.iter())
                    .flat_map(|edge| [edge.from.clone(), edge.to.clone()])
            }))
            .collect(),
    )
    .into_iter()
    .take(limit)
    .collect::<Vec<_>>();
    let edges = structural_edges_for_nodes(project, &nodes);
    (nodes, edges)
}

fn proof_graph(
    project: &Project,
    changed: Option<&[String]>,
    limit: usize,
) -> (Vec<String>, Vec<GraphEdge>) {
    let Some(changed) = changed else {
        return (Vec::new(), Vec::new());
    };
    if changed.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let report = impact_report(project, changed.to_vec(), 1, limit);
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
    )
    .into_iter()
    .take(limit)
    .collect::<Vec<_>>();
    let mut edges = structural_edges_for_nodes(project, &nodes);
    let mut seen = graph_edge_set(&edges);
    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    for cluster in report.clusters {
        for edge in cluster.proof {
            if node_set.contains(&edge.from) && node_set.contains(&edge.to) {
                push_graph_edge(&mut edges, &mut seen, edge.from, edge.to, "tests");
            }
        }
    }
    (nodes, edges)
}

fn boundary_graph(
    project: &Project,
    changed: Option<&[String]>,
    limit: usize,
) -> (Vec<String>, Vec<GraphEdge>) {
    let changed_set = changed.map(|files| files.iter().cloned().collect::<BTreeSet<_>>());
    let findings = boundary_findings(project, changed_set.as_ref());
    let nodes = unique(
        findings
            .iter()
            .flat_map(|finding| [finding.from.clone(), finding.to.clone()])
            .filter(|value| !value.is_empty())
            .collect(),
    )
    .into_iter()
    .take(limit)
    .collect::<Vec<_>>();
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
        push_graph_edge(
            &mut edges,
            &mut seen,
            finding.from,
            finding.to,
            finding.status,
        );
    }
    (nodes, edges)
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
                        node.clone(),
                        target.clone(),
                        "imports",
                    );
                }
            }
        }
    }
    for edge in &project.package_edges {
        let from = edge.from_manifest.clone();
        let to = edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone());
        if node_set.contains(&from) && node_set.contains(&to) {
            push_graph_edge(&mut edges, &mut seen, from, to, "package_depends");
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
    from: String,
    to: String,
    edge_type: impl Into<String>,
) {
    if from == to {
        return;
    }
    let edge_type = edge_type.into();
    if seen.insert((from.clone(), to.clone(), edge_type.clone())) {
        edges.push(GraphEdge {
            from,
            to,
            edge_type,
        });
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

fn directory_prefix(path: &str) -> Option<String> {
    let rel = crate::repo::normalize_rel_path(path);
    (rel != ".").then(|| format!("{}/", rel.trim_end_matches('/')))
}

fn prefix_matches(rel: &str, prefix: Option<&str>) -> bool {
    prefix.map(|prefix| rel.starts_with(prefix)).unwrap_or(true)
}
