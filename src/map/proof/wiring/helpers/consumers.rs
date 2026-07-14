// Responsibility: map-proof-wiring-helpers-consumers
use crate::model::{FileInfo, Project, ProofSurface};

pub(crate) fn proof_wiring_needs_artifact_chain(project: &Project, proof: &ProofSurface) -> bool {
    let Some(path) = proof.path.as_deref() else {
        return proof.evidence.contains("receipt") || proof.evidence.contains("witness");
    };
    project.files.get(path).is_some_and(|file| {
        file.has_role("receipt")
            || file.has_role("witness")
            || file.has_role("owner_doc")
            || proof.evidence.contains("receipt")
            || proof.evidence.contains("witness")
    })
}

pub(crate) fn command_mentions_artifact(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("receipt")
        || lower.contains("witness")
        || lower.contains("artifact")
        || lower.contains(".json")
        || lower.contains(".jsonl")
}

pub(crate) fn artifact_consumers(project: &Project, artifact: &str) -> Vec<(String, usize)> {
    let mut consumers = Vec::new();
    let basename = artifact.rsplit('/').next().unwrap_or(artifact);
    for file in project.files.values() {
        if file.rel == artifact || !proof_wiring_file_can_consume_artifact(file) {
            continue;
        }
        if !proof_wiring_file_can_consume_evidence(file) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if line.contains(artifact) || (!basename.is_empty() && line.contains(basename)) {
                consumers.push((file.rel.clone(), index + 1));
                break;
            }
        }
    }
    consumers
}

pub(crate) fn proof_wiring_consumer_texts(project: &Project, owner: &str) -> Vec<(String, String)> {
    project
        .files
        .values()
        .filter(|file| file.rel != owner)
        .filter(|file| proof_wiring_file_can_consume_artifact(file))
        .filter(|file| proof_wiring_file_can_consume_evidence(file))
        .filter_map(|file| {
            std::fs::read_to_string(project.root.join(&file.rel))
                .ok()
                .map(|text| (file.rel.clone(), text))
        })
        .collect()
}

pub(crate) fn field_consumers_from_texts(
    texts: &[(String, String)],
    field: &str,
) -> Vec<(String, usize)> {
    let needle = format!("\"{field}\"");
    let mut consumers = Vec::new();
    for (rel, text) in texts {
        for (index, line) in text.lines().enumerate() {
            if line.contains(&needle) || line.contains(field) {
                consumers.push((rel.clone(), index + 1));
                break;
            }
        }
    }
    consumers
}

fn proof_wiring_file_can_consume_artifact(file: &FileInfo) -> bool {
    if file.language != "markdown" {
        return true;
    }
    let rel = file.rel.to_ascii_lowercase();
    rel.contains("review")
        || rel.contains("report")
        || rel.contains("receipt")
        || rel.contains("witness")
}

fn proof_wiring_file_can_consume_evidence(file: &FileInfo) -> bool {
    if file.has_role("proof_runner")
        || file.has_role("doctor")
        || file.has_role("test")
        || file.has_role("schema")
        || file.has_role("schema_contract")
        || file.has_role("receipt")
        || file.has_role("witness")
    {
        return true;
    }
    let rel = file.rel.to_ascii_lowercase();
    let name = rel.rsplit('/').next().unwrap_or(&rel);
    rel.starts_with("reviews/")
        || rel.contains("/reviews/")
        || rel.starts_with("reports/")
        || rel.contains("/reports/")
        || rel.starts_with("validators/")
        || rel.contains("/validators/")
        || rel.starts_with("checks/")
        || rel.contains("/checks/")
        || name.contains("review")
        || name.contains("report")
        || name.contains("predicate")
        || name.contains("validator")
        || name.contains("validate")
        || name.contains("doctor")
        || name.contains("check")
        || name.contains("assert")
}
