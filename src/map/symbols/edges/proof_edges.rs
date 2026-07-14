// Responsibility: map-symbols-proof-edges
use super::{ImportedSymbolReferenceKind, symbol_local_incoming_edges};
use crate::evidence::import_statement_locations;
use crate::map::{
    file_imported_symbol_reference_kind, first_identifier_reference_location, matching_symbols,
    same_scope_file_references_symbol, semantic_name_terms, semantic_path_terms, sort_edges,
    split_symbol_anchor, strict_test_edges_for_file, structural_edge_with_locations,
    surface_phrase_terms, symbol_anchor_path, test_surface_terms,
};
use crate::model::{EvidenceStrength, Project, StructuralEdge};
use std::collections::BTreeSet;

pub(crate) fn symbol_proof_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(file_rel) else {
        return Vec::new();
    };
    if matching_symbols(anchor, symbol_name).is_empty() {
        return Vec::new();
    }
    let anchor_path = symbol_anchor_path(file_rel, symbol_name);
    let mut edges = Vec::new();
    for file in project.files.values() {
        if !file.has_role("test") || file.has_role("test_support") {
            continue;
        }
        let evidence = if let Some(kind) =
            file_imported_symbol_reference_kind(project, file, file_rel, symbol_name)
        {
            Some(match kind {
                ImportedSymbolReferenceKind::Direct => "test_imported_symbol_reference",
                ImportedSymbolReferenceKind::Reexported => "test_reexported_symbol_reference",
            })
        } else if same_scope_file_references_symbol(anchor, file, symbol_name) {
            Some("test_symbol_reference")
        } else {
            None
        };
        if let Some(evidence) = evidence {
            edges.push(structural_edge_with_locations(
                file.rel.clone(),
                anchor_path.clone(),
                "tests",
                evidence,
                EvidenceStrength::High,
                first_identifier_reference_location(
                    project,
                    &file.rel,
                    symbol_name,
                    "test_symbol_reference",
                ),
            ));
        }
    }
    sort_edges(&mut edges);
    edges
}

pub(crate) fn symbol_proof_edges_with_owning_file(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    limit: usize,
) -> Vec<StructuralEdge> {
    let exact = symbol_proof_edges(project, file_rel, symbol_name);
    if !exact.is_empty() {
        return exact;
    }
    let via_local_consumers =
        symbol_proof_edges_via_local_consumers(project, file_rel, symbol_name, limit);
    if !via_local_consumers.is_empty() {
        return via_local_consumers;
    }
    symbol_owning_file_proof_edges(project, file_rel, symbol_name, limit)
}

fn symbol_proof_edges_via_local_consumers(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    limit: usize,
) -> Vec<StructuralEdge> {
    let Some(info) = project.files.get(file_rel) else {
        return Vec::new();
    };
    let target_anchor = symbol_anchor_path(file_rel, symbol_name);
    let mut edges = Vec::new();
    for consumer in symbol_local_incoming_edges(project, info, symbol_name)
        .into_iter()
        .take(limit)
    {
        let Some((consumer_file, consumer_symbol)) = split_symbol_anchor(&consumer.from) else {
            continue;
        };
        if consumer_file != file_rel {
            continue;
        }
        for proof in symbol_proof_edges(project, file_rel, &consumer_symbol)
            .into_iter()
            .take(limit)
        {
            edges.push(structural_edge_with_locations(
                proof.from,
                target_anchor.clone(),
                "tests",
                format!("{}_via_local_symbol_consumer", proof.evidence),
                proof.strength.min(EvidenceStrength::Medium),
                proof.locations,
            ));
        }
    }
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    edges
}

fn symbol_owning_file_proof_edges(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    limit: usize,
) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(file_rel) else {
        return Vec::new();
    };
    if !matching_symbols(anchor, symbol_name)
        .into_iter()
        .any(|symbol| symbol.exported)
    {
        return Vec::new();
    }
    let anchor_path = symbol_anchor_path(file_rel, symbol_name);
    strict_test_edges_for_file(project, file_rel, limit)
        .into_iter()
        .filter(|(test, evidence, _)| {
            symbol_owning_file_proof_can_use(project, file_rel, symbol_name, test, evidence)
        })
        .map(|(test, evidence, strength)| {
            let locations = import_statement_locations(project, &test, file_rel);
            structural_edge_with_locations(
                test,
                anchor_path.clone(),
                "tests",
                format!("{evidence}_owning_file"),
                strength.min(EvidenceStrength::Medium),
                locations,
            )
        })
        .collect()
}

fn symbol_owning_file_proof_can_use(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    test_rel: &str,
    evidence: &str,
) -> bool {
    if !matches!(
        evidence,
        "e2e_path_surface" | "e2e_surface_phrase" | "test_surface_phrase"
    ) {
        return false;
    }
    let symbol_terms = semantic_name_terms(symbol_name);
    if symbol_terms.is_empty() {
        return false;
    }
    let Some(anchor) = project.files.get(file_rel) else {
        return false;
    };
    let path_terms = semantic_path_terms(file_rel);
    let mut sibling_terms = BTreeSet::new();
    for symbol in &anchor.symbols {
        if symbol.name != symbol_name && symbol.exported {
            sibling_terms.extend(semantic_name_terms(&symbol.name));
        }
    }
    for export in &anchor.exports {
        if export != symbol_name {
            sibling_terms.extend(semantic_name_terms(export));
        }
    }
    let distinctive_symbol_terms = symbol_terms
        .difference(&path_terms)
        .filter(|term| !sibling_terms.contains(*term))
        .cloned()
        .collect::<BTreeSet<_>>();
    if distinctive_symbol_terms.is_empty() {
        return false;
    }
    let Some(test) = project.files.get(test_rel) else {
        return false;
    };
    let mut test_terms = test_surface_terms(test);
    for phrase in &test.surface_phrases {
        test_terms.extend(surface_phrase_terms(phrase));
    }
    !distinctive_symbol_terms.is_disjoint(&test_terms)
}
