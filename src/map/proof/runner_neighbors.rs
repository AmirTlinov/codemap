// Responsibility: map-proof-runner-neighbors
use crate::map::{
    anchor_core_terms, command_target, proof_context_token, role_aware_script_locations,
    script_search_text, script_text_has_any, script_text_has_token, source_stem,
    structural_edge_with_locations,
};
use crate::model::{EvidenceLocation, EvidenceStrength, FileInfo, Project, StructuralEdge};
use std::collections::BTreeSet;

pub(crate) fn proof_runner_neighbor_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(anchor) = project.files.get(rel) else {
        return Vec::new();
    };
    let tokens = proof_neighbor_tokens(project, anchor);
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut edges = Vec::new();
    edges.extend(proof_runner_file_neighbor_edges(project, rel, &tokens));
    edges.extend(proof_runner_script_neighbor_edges(project, rel, &tokens));
    unique_structural_edges(edges)
        .into_iter()
        .take(12)
        .collect()
}

fn proof_runner_file_neighbor_edges(
    project: &Project,
    rel: &str,
    tokens: &BTreeSet<String>,
) -> Vec<StructuralEdge> {
    project
        .files
        .values()
        .filter(|file| file.rel != rel && proof_neighbor_file_candidate(file))
        .filter_map(|file| {
            let hits = proof_neighbor_token_hits(tokens, &file.tokens);
            let required_hits = if file.has_role("receipt") || file.has_role("witness") {
                1
            } else {
                2
            };
            if hits < required_hits && !proof_neighbor_mentions_path(project, &file.rel, rel) {
                return None;
            }
            Some(structural_edge_with_locations(
                rel.to_string(),
                file.rel.clone(),
                "soft_evidence_surface",
                "proof_neighbor_token_match",
                EvidenceStrength::Medium,
                proof_neighbor_file_location(project, &file.rel, tokens),
            ))
        })
        .collect()
}

fn proof_runner_script_neighbor_edges(
    project: &Project,
    rel: &str,
    tokens: &BTreeSet<String>,
) -> Vec<StructuralEdge> {
    project
        .scripts
        .iter()
        .filter_map(|script| {
            let text = script_search_text(script);
            let hits = tokens
                .iter()
                .filter(|token| script_text_has_token(&text, token))
                .count();
            let rail_match = script_text_has_any(
                &text,
                &["doctor", "next", "proof", "receipt", "validate", "verify"],
            );
            if hits < 2 && !(hits >= 1 && rail_match) {
                return None;
            }
            Some(structural_edge_with_locations(
                rel.to_string(),
                command_target(&script.command),
                "soft_evidence_surface",
                "script_surface_match",
                EvidenceStrength::Medium,
                role_aware_script_locations(script, "script_surface_match"),
            ))
        })
        .collect()
}

fn proof_neighbor_file_candidate(file: &FileInfo) -> bool {
    file.has_role("receipt")
        || file.has_role("witness")
        || file.has_role("owner_doc")
        || file.has_role("doctor")
        || file.has_role("build_ci")
        || file.rel.contains("/reviews/")
        || file.rel.starts_with("reviews/")
        || file.rel.contains("/receipts/")
        || file.rel.starts_with("receipts/")
        || file.rel.contains("/witnesses/")
        || file.rel.starts_with("witnesses/")
        || file.rel.contains("/artifacts/")
        || file.rel.starts_with("artifacts/")
}

fn proof_neighbor_tokens(project: &Project, file: &FileInfo) -> BTreeSet<String> {
    let mut tokens = anchor_core_terms(project, &file.rel);
    tokens.extend(
        file.tokens
            .iter()
            .filter(|token| proof_context_token(token))
            .cloned(),
    );
    tokens
        .into_iter()
        .filter(|token| {
            token.len() >= 4
                && !matches!(
                    token.as_str(),
                    "proof" | "runner" | "receipt" | "witness" | "doctor" | "validate"
                )
        })
        .collect()
}

fn proof_neighbor_token_hits(tokens: &BTreeSet<String>, other: &BTreeSet<String>) -> usize {
    tokens.iter().filter(|token| other.contains(*token)).count()
}

fn proof_neighbor_mentions_path(project: &Project, candidate: &str, rel: &str) -> bool {
    let Some(text) = project.read_indexed_text(candidate) else {
        return false;
    };
    let lower = text.to_ascii_lowercase();
    lower.contains(&rel.to_ascii_lowercase()) || lower.contains(&source_stem(rel))
}

fn proof_neighbor_file_location(
    project: &Project,
    rel: &str,
    tokens: &BTreeSet<String>,
) -> Vec<EvidenceLocation> {
    let token_refs = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    if let Some(line) = first_line_containing_ci(project, rel, &token_refs) {
        vec![EvidenceLocation::line(
            rel,
            line,
            "proof_neighbor_token_match",
        )]
    } else {
        vec![EvidenceLocation::path(rel, "proof_neighbor_token_match")]
    }
}

fn first_line_containing_ci(project: &Project, rel: &str, needles: &[&str]) -> Option<usize> {
    let text = project.read_indexed_text(rel)?;
    text.lines().enumerate().find_map(|(index, line)| {
        let lower = line.to_ascii_lowercase();
        needles
            .iter()
            .any(|needle| lower.contains(&needle.to_ascii_lowercase()))
            .then_some(index + 1)
    })
}

fn unique_structural_edges(values: Vec<StructuralEdge>) -> Vec<StructuralEdge> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for edge in values {
        let key = (
            edge.from.clone(),
            edge.to.clone(),
            edge.edge_type.clone(),
            edge.evidence.clone(),
        );
        if seen.insert(key) {
            out.push(edge);
        }
    }
    out
}
