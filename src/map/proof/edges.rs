// Responsibility: map-proof-edges
use crate::evidence::import_statement_locations;
use crate::map::{
    anchor_core_terms, anchor_symbol_reference_names, anchor_terms, boundary_findings,
    contract_document_candidate_edges, contract_neighborhood_edges, direct_consumer_edges,
    direct_dependency_edges, directory_has_files, edge_with_path_location, package_for_rel,
    proof_evidence_precedence, semantic_name_terms, strict_test_edges_for_file,
    structural_edge_with_locations, structural_test_surface_match,
};
use crate::model::{EvidenceStrength, FileInfo, Project, StructuralEdge};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn cone_proof_edges(project: &Project, seeds: &[String]) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for seed in seeds {
        for (test, evidence, strength) in strict_test_edges_for_file(project, seed, usize::MAX) {
            let locations = import_statement_locations(project, &test, seed);
            edges.push(structural_edge_with_locations(
                test,
                seed.clone(),
                "tests",
                evidence,
                strength,
                locations,
            ));
        }
    }
    edges
}

pub(crate) fn cone_proof_edges_with_direct_consumers(
    project: &Project,
    seeds: &[String],
) -> Vec<StructuralEdge> {
    let mut edges = cone_proof_edges(project, seeds);
    for seed in seeds {
        if !edges.iter().any(|edge| edge.to == *seed) {
            edges.extend(proof_edges_via_direct_dependencies(
                project,
                seed,
                usize::MAX,
            ));
        }
        for consumer in direct_consumer_edges(project, seed) {
            for (test, evidence, strength) in
                strict_test_edges_for_file(project, &consumer.from, usize::MAX)
            {
                let Some(test_file) = project.files.get(&test) else {
                    continue;
                };
                if !test_mentions_anchor(project, seed, test_file) {
                    continue;
                }
                let locations = import_statement_locations(project, &test, &consumer.from);
                edges.push(structural_edge_with_locations(
                    test,
                    seed.clone(),
                    "tests",
                    format!("{evidence}_via_direct_consumer"),
                    mediated_proof_strength(strength),
                    locations,
                ));
            }
        }
    }
    dedupe_proof_edges_by_endpoint(edges)
}

pub(crate) fn mediated_proof_strength(strength: EvidenceStrength) -> EvidenceStrength {
    match strength {
        EvidenceStrength::Hard | EvidenceStrength::High => EvidenceStrength::Medium,
        other => other,
    }
}

pub(crate) fn proof_edges_via_direct_dependencies(
    project: &Project,
    seed: &str,
    limit: usize,
) -> Vec<StructuralEdge> {
    if limit == 0 {
        return Vec::new();
    }
    let Some(anchor) = project.files.get(seed) else {
        return Vec::new();
    };
    if !anchor_can_use_dependency_proof(anchor) {
        return Vec::new();
    }
    let mut edges = Vec::new();
    for dependency in direct_dependency_edges(project, seed)
        .into_iter()
        .take(limit)
    {
        let Some(dep_file) = project.files.get(&dependency.to) else {
            continue;
        };
        if !dependency_can_prove_anchor(project, anchor, dep_file) {
            continue;
        }
        for (test, evidence, strength) in strict_test_edges_for_file(project, &dependency.to, limit)
            .into_iter()
            .filter(|(_, evidence, _)| dependency_proof_can_transfer(evidence))
        {
            let locations = import_statement_locations(project, &test, &dependency.to);
            edges.push(structural_edge_with_locations(
                test,
                seed.to_string(),
                "tests",
                format!("{evidence}_via_direct_dependency"),
                strength,
                locations,
            ));
        }
    }
    edges
}

fn dependency_proof_can_transfer(evidence: &str) -> bool {
    evidence == "e2e_surface_phrase"
}

fn anchor_can_use_dependency_proof(anchor: &FileInfo) -> bool {
    anchor.has_role("renderer_ui")
        || matches!(anchor.ext.as_str(), "tsx" | "jsx" | "vue" | "svelte")
}

fn dependency_can_prove_anchor(
    project: &Project,
    anchor: &FileInfo,
    dependency: &FileInfo,
) -> bool {
    if dependency.has_role("test") || dependency.has_role("test_support") {
        return false;
    }
    if package_for_rel(project, &anchor.rel).map(|package| package.path.clone())
        != package_for_rel(project, &dependency.rel).map(|package| package.path.clone())
    {
        return false;
    }
    if Path::new(&anchor.rel).parent() != Path::new(&dependency.rel).parent() {
        return false;
    }
    if !anchor_renders_dependency(anchor, dependency) {
        return false;
    }
    dependency.has_role("renderer_ui")
        || !dependency.surface_phrases.is_empty()
        || dependency
            .symbols
            .iter()
            .any(|symbol| symbol.kind == "component")
}

fn anchor_renders_dependency(anchor: &FileInfo, dependency: &FileInfo) -> bool {
    if anchor.jsx_tags.is_empty() {
        return false;
    }
    let Some(bindings) = anchor.resolved_import_bindings.get(&dependency.rel) else {
        return false;
    };
    let exported_components = dependency
        .symbols
        .iter()
        .filter(|symbol| symbol.exported && symbol.kind == "component")
        .map(|symbol| &symbol.name)
        .collect::<BTreeSet<_>>();
    bindings.iter().any(|(local, imported)| {
        anchor.jsx_tags.contains(local)
            && !anchor_declares_symbol(anchor, local)
            && exported_components.contains(imported)
    })
}

fn anchor_declares_symbol(anchor: &FileInfo, name: &str) -> bool {
    anchor.symbols.iter().any(|symbol| symbol.name == name) || anchor.local_bindings.contains(name)
}

fn test_mentions_anchor(project: &Project, rel: &str, test: &FileInfo) -> bool {
    let Some(anchor) = project.files.get(rel) else {
        return false;
    };
    if anchor_symbol_reference_names(anchor)
        .iter()
        .any(|name| test.references.contains(name))
    {
        return true;
    }
    let anchor_terms = anchor_terms(project, rel);
    let anchor_core_terms = anchor_core_terms(project, rel);
    if structural_test_surface_match(project, rel, &anchor_terms, &anchor_core_terms, test)
        .is_some()
    {
        return true;
    }
    if anchor_core_terms.is_empty() {
        return false;
    }
    let mut reference_terms = BTreeSet::new();
    for reference in &test.references {
        reference_terms.extend(semantic_name_terms(reference));
    }
    anchor_core_terms.intersection(&reference_terms).count() >= 1
}

pub(crate) fn dedupe_proof_edges_by_endpoint(edges: Vec<StructuralEdge>) -> Vec<StructuralEdge> {
    let mut seen = BTreeMap::new();
    let mut out: Vec<StructuralEdge> = Vec::new();
    for edge in edges {
        let key = (edge.from.clone(), edge.to.clone(), edge.edge_type.clone());
        if let Some(index) = seen.get(&key).copied() {
            if proof_edge_precedence(&edge) > proof_edge_precedence(&out[index]) {
                out[index] = edge;
            }
        } else {
            seen.insert(key, out.len());
            out.push(edge);
        }
    }
    out
}

fn proof_edge_precedence(edge: &StructuralEdge) -> (EvidenceStrength, usize) {
    (edge.strength, proof_evidence_precedence(&edge.evidence))
}

pub(crate) fn cone_contract_edges(
    project: &Project,
    outgoing: &[StructuralEdge],
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for edge in outgoing {
        let Some(target) = project.files.get(&edge.to) else {
            continue;
        };
        edges.extend(contract_document_candidate_edges(project, &edge.to));
        if let Some(evidence) = contract_evidence(target) {
            edges.push(structural_edge_with_locations(
                edge.from.clone(),
                edge.to.clone(),
                "contract",
                evidence,
                EvidenceStrength::High,
                edge.locations.clone(),
            ));
        }
    }
    for target in edges
        .iter()
        .map(|edge| edge.to.clone())
        .collect::<BTreeSet<_>>()
    {
        edges.extend(contract_neighborhood_edges(project, &target));
    }
    edges
}

pub(crate) fn contract_evidence(file: &FileInfo) -> Option<String> {
    for role in [
        "schema_contract",
        "public_boundary",
        "semantic_anchor",
        "build_ci",
    ] {
        if file.has_role(role) {
            return Some(format!("role:{role}"));
        }
    }
    (file.language == "config").then(|| "language:config".to_string())
}

pub(crate) fn cone_boundary_edges(
    project: &Project,
    rel: &str,
    depths: &BTreeMap<String, usize>,
) -> Vec<StructuralEdge> {
    let node_set = depths.keys().cloned().collect::<BTreeSet<_>>();
    let directory_prefix = directory_has_files(project, rel).then(|| {
        if rel == "." {
            String::new()
        } else {
            format!("{}/", rel.trim_end_matches('/'))
        }
    });
    boundary_findings(project, None)
        .into_iter()
        .filter(|finding| {
            node_set.contains(&finding.from)
                || node_set.contains(&finding.to)
                || directory_prefix
                    .as_ref()
                    .map(|prefix| {
                        if prefix.is_empty() {
                            true
                        } else {
                            finding.from.starts_with(prefix) || finding.to.starts_with(prefix)
                        }
                    })
                    .unwrap_or(false)
        })
        .map(|finding| {
            edge_with_path_location(
                finding.from.clone(),
                finding.to,
                "boundary",
                finding.provenance,
                if finding.strength == "hard" {
                    EvidenceStrength::Hard
                } else {
                    EvidenceStrength::Medium
                },
                finding.from,
                "boundary_rule_match",
            )
        })
        .collect()
}
