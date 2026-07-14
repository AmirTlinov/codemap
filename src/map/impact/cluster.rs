// Responsibility: map-impact-cluster
use crate::map::{
    cone_proof_edges_with_direct_consumers, contract_link_edges, cross_boundary_consumer_edges,
    direct_consumer_edges, proof_evidence_precedence, shell_quote, sort_edges, unique,
};
use crate::model::{EvidenceStrength, HiddenGroup, ImpactCluster, Project, Risk, StructuralEdge};
use std::collections::BTreeMap;

pub(crate) fn impact_expand_commands(changed: &[String], selector: &str) -> Vec<String> {
    if changed.is_empty() {
        return Vec::new();
    }
    let selector = selector.trim();
    if selector.is_empty() {
        return Vec::new();
    }
    let proof_selector = if selector == "--changed" {
        "changed".to_string()
    } else {
        selector.to_string()
    };
    vec![
        format!("codemap impact {selector} --depth 2"),
        format!("codemap proof {proof_selector}"),
    ]
}

pub(crate) fn impact_cluster(
    project: &Project,
    rel: &str,
    depth: usize,
    limit: usize,
) -> (ImpactCluster, Vec<HiddenGroup>) {
    let mut direct_consumers = direct_consumer_edges(project, rel);
    let mut cross_boundary_consumers =
        cross_boundary_consumer_edges(project, rel, &direct_consumers, depth);
    let mut contract_links = contract_link_edges(project, rel, &direct_consumers);
    let proof_seeds = proof_seeds_for_impact(rel, &direct_consumers);
    let mut proof = dedupe_impact_proof_edges(
        cone_proof_edges_with_direct_consumers(project, &proof_seeds),
        rel,
    );
    sort_edges(&mut direct_consumers);
    sort_edges(&mut cross_boundary_consumers);
    sort_edges(&mut contract_links);
    sort_edges(&mut proof);
    let (risk, reasons) = structural_impact_level(
        project,
        rel,
        &direct_consumers,
        &cross_boundary_consumers,
        &contract_links,
    );
    let mut hidden = Vec::new();
    limit_impact_edges(
        &mut direct_consumers,
        limit,
        &mut hidden,
        rel,
        depth,
        "direct consumer edges hidden by limit",
    );
    limit_impact_edges(
        &mut cross_boundary_consumers,
        limit,
        &mut hidden,
        rel,
        depth,
        "cross-boundary consumer edges hidden by limit",
    );
    limit_impact_edges(
        &mut contract_links,
        limit,
        &mut hidden,
        rel,
        depth,
        "contract link edges hidden by limit",
    );
    limit_impact_edges(
        &mut proof,
        limit,
        &mut hidden,
        rel,
        depth,
        "verification edges hidden by limit",
    );
    (
        ImpactCluster {
            id: format!("changed:{rel}"),
            risk: risk.as_str().to_string(),
            changed: vec![rel.to_string()],
            direct_consumers,
            cross_boundary_consumers,
            contract_links,
            proof,
            reasons,
        },
        hidden,
    )
}

fn limit_impact_edges(
    edges: &mut Vec<StructuralEdge>,
    limit: usize,
    hidden: &mut Vec<HiddenGroup>,
    rel: &str,
    depth: usize,
    reason: &str,
) {
    if edges.len() <= limit {
        return;
    }
    hidden.push(HiddenGroup {
        reason: format!("{reason} for changed:{rel}"),
        count: edges.len() - limit,
        expand: format!(
            "codemap impact --files {} --depth {depth} --limit {}",
            shell_quote(rel),
            edges.len()
        ),
    });
    edges.truncate(limit);
}

fn dedupe_impact_proof_edges(edges: Vec<StructuralEdge>, changed_rel: &str) -> Vec<StructuralEdge> {
    let mut seen = BTreeMap::new();
    let mut out: Vec<StructuralEdge> = Vec::new();
    for edge in edges {
        let key = (edge.from.clone(), edge.edge_type.clone());
        if let Some(index) = seen.get(&key).copied() {
            if impact_proof_precedence(&edge, changed_rel)
                > impact_proof_precedence(&out[index], changed_rel)
            {
                out[index] = edge;
            }
        } else {
            seen.insert(key, out.len());
            out.push(edge);
        }
    }
    sort_edges(&mut out);
    out
}

fn impact_proof_precedence(
    edge: &StructuralEdge,
    changed_rel: &str,
) -> (bool, EvidenceStrength, usize) {
    (
        edge.to == changed_rel,
        edge.strength,
        proof_evidence_precedence(&edge.evidence),
    )
}

fn proof_seeds_for_impact(rel: &str, direct_consumers: &[StructuralEdge]) -> Vec<String> {
    let mut seeds = vec![rel.to_string()];
    seeds.extend(direct_consumers.iter().map(|edge| edge.from.clone()));
    unique(seeds)
}

pub(crate) fn structural_impact_level(
    project: &Project,
    rel: &str,
    direct_consumers: &[StructuralEdge],
    cross_boundary_consumers: &[StructuralEdge],
    contract_links: &[StructuralEdge],
) -> (Risk, Vec<String>) {
    let Some(file) = project.files.get(rel) else {
        return (
            Risk::Medium,
            vec!["changed file is not indexed".to_string()],
        );
    };
    let mut risk = Risk::Low;
    let mut reasons = Vec::new();
    let mut bump = |level, reason: &str| {
        risk = risk.max(level);
        reasons.push(reason.to_string());
    };
    if file.has_role("generated") {
        bump(Risk::Critical, "generated file changed");
    }
    if file.has_role("public_boundary") {
        bump(Risk::Critical, "public boundary changed");
    }
    if file.has_role("schema_contract") {
        bump(Risk::High, "schema or DTO contract changed");
    }
    if file.has_role("semantic_anchor") {
        bump(Risk::High, "semantic anchor changed");
    }
    if file.has_role("runtime_state") {
        bump(Risk::MediumHigh, "runtime state surface changed");
    }
    if file.has_role("persistence") {
        bump(Risk::High, "persistence surface changed");
    }
    if !contract_links.is_empty() {
        bump(Risk::High, "contract surface participates");
    }
    if !cross_boundary_consumers.is_empty() {
        bump(Risk::High, "consumer crosses package or domain boundary");
    }
    if direct_consumers.len() >= 3 {
        bump(Risk::High, "multiple direct consumers");
    } else if !direct_consumers.is_empty() {
        bump(Risk::Medium, "direct consumer exists");
    }
    (risk, unique(reasons))
}
