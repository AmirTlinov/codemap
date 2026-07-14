// Responsibility: map-proof-owner-surfaces-env
use crate::map::{
    ci_line_reference_proof_surfaces, line_may_contain_static_env_reference, prisma_env_names,
    static_env_names, unique_proof_surfaces,
};
use crate::model::{EvidenceLocation, EvidenceStrength, FileInfo, Project, ProofSurface};
use std::collections::BTreeSet;

pub(crate) fn env_consumer_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    let keys = env_declared_keys(project, &file.rel);
    if keys.is_empty() {
        return Vec::new();
    }
    let key_set = keys
        .iter()
        .map(|(key, _)| key.as_str())
        .collect::<BTreeSet<_>>();
    let mut out = Vec::new();
    for candidate in project.files.values() {
        if candidate.rel == file.rel
            || candidate.has_role("generated")
            || candidate.has_role("fixture")
            || candidate.has_role("archive")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(project.root.join(&candidate.rel)) else {
            continue;
        };
        for (line_number, line) in text.lines().enumerate() {
            if !line_may_contain_static_env_reference(line) {
                continue;
            }
            let mut names = static_env_names(line);
            names.extend(prisma_env_names(line));
            names.sort();
            names.dedup();
            for name in names {
                if !key_set.contains(name.as_str()) {
                    continue;
                }
                out.push(ProofSurface {
                    command: None,
                    path: Some(candidate.rel.clone()),
                    target_anchor: Some(file.rel.clone()),
                    evidence: "env_consumer_reference".to_string(),
                    strength: EvidenceStrength::High,
                    reason: format!("source reads env key `{name}` declared in {}", file.rel),
                    locations: vec![EvidenceLocation::line(
                        &candidate.rel,
                        line_number + 1,
                        "env_reference",
                    )],
                });
            }
        }
    }
    unique_proof_surfaces(out)
}

pub(crate) fn env_ci_reference_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    let keys = env_declared_keys(project, &file.rel)
        .into_iter()
        .map(|(key, _)| key)
        .collect::<BTreeSet<_>>();
    if keys.is_empty() {
        return Vec::new();
    }
    ci_line_reference_proof_surfaces(project, file, "env_ci_reference", |line| {
        keys.iter()
            .find(|key| line.contains(key.as_str()))
            .map(|key| format!("CI line references env key `{key}`"))
    })
}

pub(crate) fn env_declared_keys(project: &Project, rel: &str) -> Vec<(String, usize)> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, _)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim().trim_start_matches("export ").trim();
        if !name.is_empty()
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            out.push((name.to_string(), index + 1));
        }
    }
    out
}
