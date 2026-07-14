// Responsibility: map-cone-xray-context-surfaces
use crate::map::{flow_report, side_effect_surfaces_for_file};
use crate::model::{EvidenceStrength, FileInfo, FlowStep, Project, Surface};
use std::path::Path;

pub(crate) fn xray_side_effects(
    project: &Project,
    seed_files: &[String],
    limit: usize,
    include_hidden: bool,
) -> Vec<Surface> {
    let mut surfaces = Vec::new();
    let directory_scope = seed_files.len() > 1;
    for rel in seed_files {
        if let Some(file) = project.files.get(rel) {
            if directory_scope
                && !include_hidden
                && (file.has_role("test") || file.has_role("test_support"))
            {
                continue;
            }
            surfaces.extend(side_effect_surfaces_for_file(project, file));
        }
    }
    surfaces.into_iter().take(limit).collect()
}

pub(crate) fn xray_flow_steps(
    project: &Project,
    seed_files: &[String],
    limit: usize,
) -> Vec<FlowStep> {
    let Some(rel) = seed_files.first() else {
        return Vec::new();
    };
    if !project.files.contains_key(rel) {
        return Vec::new();
    }
    flow_report(project, rel, false, limit)
        .steps
        .into_iter()
        .take(limit)
        .collect()
}

pub(crate) fn xray_nearby_surfaces(
    project: &Project,
    seed_files: &[String],
    limit: usize,
) -> Vec<Surface> {
    let Some(rel) = seed_files.first() else {
        return Vec::new();
    };
    let Some(parent) = Path::new(rel).parent() else {
        return Vec::new();
    };
    let parent = parent.to_string_lossy();
    let prefix = if parent.is_empty() || parent == "." {
        String::new()
    } else {
        format!("{}/", parent.trim_end_matches('/'))
    };
    let mut surfaces = Vec::new();
    for file in project.files.values() {
        if file.rel == *rel || !file.rel.starts_with(&prefix) {
            continue;
        }
        let rest = file.rel.trim_start_matches(&prefix);
        if rest.contains('/') {
            continue;
        }
        let Some(kind) = xray_nearby_kind(file) else {
            continue;
        };
        surfaces.push(Surface {
            id: format!("surface:xray_nearby:{kind}:{}", file.rel),
            kind: kind.to_string(),
            path: Some(file.rel.clone()),
            role: Some("existing_nearby_surface".to_string()),
            evidence: "same_directory_surface".to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(1),
            examples: vec![file.rel.clone()],
            hidden_count: 0,
        });
    }
    surfaces.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.path.cmp(&b.path)));
    surfaces.into_iter().take(limit).collect()
}

fn xray_nearby_kind(file: &FileInfo) -> Option<&'static str> {
    let name = Path::new(&file.rel)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if file.has_role("test") {
        Some("test")
    } else if file.has_role("schema_contract") {
        Some("schema")
    } else if file.has_role("repository") {
        Some("repository")
    } else if file.has_role("parser") {
        Some("parser")
    } else if file.has_role("proof_runner") {
        Some("proof_runner")
    } else if file.has_role("doctor") || name.contains("check") || name.contains("validate") {
        Some("validator")
    } else if name.contains("digest") || name.contains("hash") || name.contains("checksum") {
        Some("digest_helper")
    } else if file.has_role("receipt") || file.has_role("witness") {
        Some("witness")
    } else if file.has_role("manifest")
        || file.has_role("env_config")
        || file.has_role("runtime_config")
    {
        Some("config")
    } else if file.has_role("public_boundary") {
        Some("public_boundary")
    } else {
        None
    }
}
