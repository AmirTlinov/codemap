use std::collections::BTreeSet;

use crate::model::{Domain, GraphEdge, GraphLens, Project, VerificationPlan};

use super::{
    boundary_findings, domain_files, impact_report, impacted_domains, path_mentions_support,
    primary_domain, select_read_first, test_files_for, unique, verification_plan,
};

pub fn graph_lens(
    project: &Project,
    path: Option<&str>,
    lens: &str,
    limit: usize,
    changed: Option<&[String]>,
) -> GraphLens {
    let requested_domain = primary_domain(project, "", path);
    let lens_key = lens.to_ascii_lowercase();
    let (nodes, edges) = match lens_key.as_str() {
        "boundary" | "boundaries" => boundary_graph(project, limit),
        "verification" | "verify" => verification_graph(project, &requested_domain, changed, limit),
        "impact" if changed.is_some() => impact_graph(project, changed.unwrap_or(&[]), limit),
        _ => causal_graph(project, &requested_domain, path, limit),
    };
    let domain = graph_output_domain(
        project,
        &requested_domain,
        &nodes,
        matches!(
            lens_key.as_str(),
            "boundary" | "boundaries" | "impact" | "verification" | "verify"
        ),
    );
    GraphLens {
        kind: "graph_lens",
        schema_version: "1",
        domain: (&domain).into(),
        lens: lens.to_string(),
        nodes,
        edges,
    }
}

fn graph_output_domain(
    project: &Project,
    fallback: &Domain,
    nodes: &[String],
    derive_from_nodes: bool,
) -> Domain {
    if !derive_from_nodes {
        return fallback.clone();
    }
    let file_nodes = nodes
        .iter()
        .filter(|node| project.files.contains_key(*node))
        .cloned()
        .collect::<Vec<_>>();
    let domains = impacted_domains(project, &file_nodes);
    match domains.as_slice() {
        [only] => (*only).clone(),
        [] => fallback.clone(),
        _ => project
            .domains
            .iter()
            .find(|domain| domain.path == ".")
            .cloned()
            .unwrap_or_else(|| Domain {
                id: "multi".to_string(),
                path: ".".to_string(),
                config_path: None,
            }),
    }
}

fn causal_graph(
    project: &Project,
    domain: &Domain,
    path: Option<&str>,
    limit: usize,
) -> (Vec<String>, Vec<GraphEdge>) {
    let include_support =
        path.map(path_mentions_support).unwrap_or(false) || path_mentions_support(&domain.path);
    let mut scored = domain_files(project, domain)
        .into_iter()
        .filter(|file| {
            !file.has_role("test")
                && !file.has_role("generated")
                && (include_support || (!file.has_role("fixture") && !file.has_role("example")))
        })
        .map(|file| {
            let mut score = 0.0;
            for (role, boost) in [
                ("source_of_truth", 5.0),
                ("runtime_state", 4.5),
                ("public_boundary", 4.0),
                ("schema_contract", 3.0),
                ("adapter", 2.5),
                ("parser", 2.0),
                ("persistence", 2.0),
            ] {
                if file.has_role(role) {
                    score += boost;
                }
            }
            score += project
                .reverse_imports
                .get(&file.rel)
                .map(|x| x.len() as f64 * 0.4)
                .unwrap_or(0.0)
                .min(4.0);
            (score, file.rel.clone())
        })
        .filter(|(score, _)| *score > 0.0)
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let nodes = scored
        .into_iter()
        .map(|(_, rel)| rel)
        .take(limit)
        .collect::<Vec<_>>();
    let edges = structural_edges_for_nodes(project, &nodes);
    (nodes, edges)
}

fn impact_graph(
    project: &Project,
    changed: &[String],
    limit: usize,
) -> (Vec<String>, Vec<GraphEdge>) {
    let report = impact_report(project, changed.to_vec(), 2, limit);
    let nodes = unique([report.changed, report.impacted].concat())
        .into_iter()
        .take(limit)
        .collect::<Vec<_>>();
    let edges = structural_edges_for_nodes(project, &nodes);
    (nodes, edges)
}

fn verification_graph(
    project: &Project,
    domain: &Domain,
    changed: Option<&[String]>,
    limit: usize,
) -> (Vec<String>, Vec<GraphEdge>) {
    let (seed_files, related_tests, plan) = match changed {
        Some(changed) => {
            if changed.is_empty() {
                return (Vec::new(), Vec::new());
            }
            let report = impact_report(project, changed.to_vec(), 2, limit);
            let seeds = unique([report.changed.clone(), report.impacted.clone()].concat());
            let tests = report.related_tests.clone();
            let plan = VerificationPlan {
                minimal: report.minimal_verification,
                recommended: report.recommended_verification,
                full_only_if_triggered: report.full_verification,
            };
            (seeds, tests, plan)
        }
        None => {
            let seeds = select_read_first(
                project,
                domain,
                "",
                "general",
                limit.min(5),
                &BTreeSet::new(),
            )
            .into_iter()
            .map(|candidate| candidate.path)
            .collect::<Vec<_>>();
            let tests = test_files_for(project, &seeds, Some(domain), 5);
            let plan = verification_plan(project, &seeds, &[]);
            (seeds, tests, plan)
        }
    };
    let command_nodes = plan
        .minimal
        .iter()
        .chain(plan.recommended.iter())
        .take(3)
        .map(|command| format!("$ {command}"))
        .collect::<Vec<_>>();

    let mut nodes = Vec::new();
    let reserve_for_commands = command_nodes.len().min(limit.saturating_sub(1));
    let content_limit = limit.saturating_sub(reserve_for_commands);
    let seed_limit = content_limit.div_ceil(2).max(1);
    push_unique_nodes(
        &mut nodes,
        seed_files.iter().take(seed_limit).cloned(),
        content_limit,
    );
    let test_limit = content_limit.max(nodes.len());
    push_unique_nodes(&mut nodes, related_tests.iter().cloned(), test_limit);
    push_unique_nodes(&mut nodes, command_nodes.iter().cloned(), limit);

    let node_set = nodes.iter().cloned().collect::<BTreeSet<_>>();
    let mut edges = structural_edges_for_nodes(project, &nodes);
    let mut seen = graph_edge_set(&edges);
    for source in &seed_files {
        if !node_set.contains(source)
            || project
                .files
                .get(source)
                .map(|file| file.has_role("test"))
                .unwrap_or(false)
        {
            continue;
        }
        for test in test_files_for(project, std::slice::from_ref(source), None, 5) {
            if node_set.contains(&test) {
                push_graph_edge(&mut edges, &mut seen, source.clone(), test, "tested_by");
            }
        }
    }
    if let Some(command) = command_nodes
        .iter()
        .find(|command| node_set.contains(*command))
    {
        let mut connected = false;
        for test in &related_tests {
            if node_set.contains(test) {
                push_graph_edge(
                    &mut edges,
                    &mut seen,
                    test.clone(),
                    command.clone(),
                    "verified_by",
                );
                connected = true;
            }
        }
        if !connected {
            for source in &seed_files {
                if node_set.contains(source) {
                    push_graph_edge(
                        &mut edges,
                        &mut seen,
                        source.clone(),
                        command.clone(),
                        "verified_by",
                    );
                }
            }
        }
    }
    (nodes, edges)
}

fn boundary_graph(project: &Project, limit: usize) -> (Vec<String>, Vec<GraphEdge>) {
    let findings = boundary_findings(project, None);
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
