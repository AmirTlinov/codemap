// Responsibility: map-proof-report-unknowns
use crate::map::{
    changed_env_unknowns, changed_manifest_unknowns, changed_schema_unknowns,
    is_support_artifact_path, owner_env_unknowns, proof_missing_should_surface,
    proof_surface_satisfies_specific_proof, schema_owner_path, shell_quote,
    unknown_missing_deterministic_proof,
};
use crate::model::{FileInfo, Project, ProofSurface, Unknown};

pub(crate) fn proof_target_owner_unknowns(
    project: &Project,
    target: &str,
    file: &FileInfo,
) -> Vec<Unknown> {
    let mut unknowns = Vec::new();
    if file.has_role("manifest") {
        unknowns.extend(changed_manifest_unknowns(project, target));
    }
    if file.has_role("schema_contract") || schema_owner_path(target) {
        unknowns.extend(changed_schema_unknowns(project, target));
    }
    if file.has_role("env_config") {
        unknowns.extend(owner_env_unknowns(project, target));
        unknowns.extend(changed_env_unknowns(project, target));
    }
    unknowns
}

pub(crate) fn changed_missing_deterministic_proof_unknowns(
    project: &Project,
    changed: &[String],
    fallback: &[String],
    proofs: &[ProofSurface],
) -> Vec<Unknown> {
    if !fallback.is_empty() || proofs.iter().any(proof_surface_satisfies_specific_proof) {
        return Vec::new();
    }
    let has_soft_or_empty = proofs.is_empty()
        || proofs
            .iter()
            .any(crate::proof_classification::proof_surface_is_soft_evidence);
    if !has_soft_or_empty {
        return Vec::new();
    }
    changed
        .iter()
        .filter(|rel| changed_path_needs_missing_deterministic_proof_unknown(project, rel))
        .map(|rel| {
            unknown_missing_deterministic_proof(
                rel,
                format!("codemap proof-map --files {}", shell_quote(rel)),
            )
        })
        .collect()
}

fn changed_path_needs_missing_deterministic_proof_unknown(project: &Project, rel: &str) -> bool {
    if is_support_artifact_path(rel) {
        return true;
    }
    project.files.get(rel).is_some_and(|file| {
        proof_missing_should_surface(project, rel)
            || [
                "receipt",
                "witness",
                "fixture",
                "generated",
                "archive",
                "build_output",
            ]
            .iter()
            .any(|role| file.has_role(role))
    })
}
