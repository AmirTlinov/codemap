// Responsibility: map-proof-owner-surfaces-schema
use crate::map::{
    ci_run_reference_proof_surfaces, package_for_rel, package_json_scripts, package_script_command,
    schema_ci_run_match_reason,
};
use crate::model::{EvidenceLocation, EvidenceStrength, FileInfo, Project, ProofSurface};
use std::path::Path;

pub(crate) fn schema_script_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    let Some(package) = package_for_rel(project, &file.rel) else {
        return Vec::new();
    };
    if package.ecosystem != "javascript" {
        return Vec::new();
    }
    package_json_scripts(project, &package.manifest)
        .into_iter()
        .filter(|(name, command, _)| schema_script_is_proof_relevant(name, command, &file.rel))
        .map(|(name, command, line)| ProofSurface {
            command: package_script_command(project, package, &name),
            path: Some(package.manifest.clone()),
            target_anchor: Some(file.rel.clone()),
            evidence: "schema_package_script".to_string(),
            strength: EvidenceStrength::Hard,
            reason: format!("package script references schema tooling: `{name}` -> {command}"),
            locations: vec![EvidenceLocation::line(
                &package.manifest,
                line,
                "package_script",
            )],
        })
        .collect()
}

pub(crate) fn schema_ci_reference_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    let owner_package = package_for_rel(project, &file.rel);
    ci_run_reference_proof_surfaces(project, file, "schema_ci_reference", |command| {
        schema_ci_run_match_reason(owner_package, &file.rel, command)
    })
}

fn schema_script_is_proof_relevant(name: &str, command: &str, rel: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let command = command.to_ascii_lowercase();
    let hay = format!("{name} {command}");
    let schema_name = Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(rel)
        .to_ascii_lowercase();
    [
        "prisma",
        "migrate",
        "migration",
        "db:",
        "db-",
        "generate",
        "seed",
    ]
    .iter()
    .any(|needle| hay.contains(needle))
        || (!schema_name.is_empty() && command.contains(&schema_name))
}

pub(crate) fn schema_owner_path(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with(".prisma")
        || lower.ends_with(".graphql")
        || lower.ends_with(".gql")
        || lower.ends_with(".proto")
        || lower.ends_with(".avsc")
        || lower.ends_with(".sql")
        || lower.starts_with("migrations/")
        || lower.contains("/migrations/")
}
