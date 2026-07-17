// Responsibility: soft-contract-document-candidates
use crate::map::structural_edge_with_locations;
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge};
use crate::repo;
use std::collections::BTreeSet;

pub(super) fn contract_document_candidates(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let anchor_tokens = contract_subject_tokens(rel);
    if anchor_tokens.is_empty() {
        return Vec::new();
    }
    project
        .files
        .values()
        .filter(|file| {
            file.rel != rel
                && file.ext == "md"
                && file.rel.starts_with("contracts/")
                && !anchor_tokens.is_disjoint(&contract_subject_tokens(&file.rel))
        })
        .map(|file| {
            structural_edge_with_locations(
                rel.to_string(),
                file.rel.clone(),
                "documented_by_candidate",
                "shared_contract_path_token",
                EvidenceStrength::Low,
                vec![EvidenceLocation::path(
                    &file.rel,
                    "contract_document_candidate",
                )],
            )
        })
        .collect()
}

fn contract_subject_tokens(rel: &str) -> BTreeSet<String> {
    const GENERIC: [&str; 12] = [
        "app",
        "apps",
        "contract",
        "contracts",
        "doc",
        "docs",
        "index",
        "lib",
        "src",
        "schema",
        "schemas",
        "types",
    ];
    repo::path_tokens(rel)
        .into_iter()
        .filter(|token| token.len() >= 4 && !GENERIC.contains(&token.as_str()))
        .collect()
}
