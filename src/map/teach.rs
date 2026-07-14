// Responsibility: map-teach
use crate::model::{FileInfo, Project, TeachProofCommand, TeachReport, TeachRolePattern};
use std::collections::BTreeMap;
use std::path::Path;

pub fn teach_report(project: &Project) -> TeachReport {
    let role_patterns = teach_role_patterns(project);
    let proof_changed = teach_proof_commands(project);
    let codemap_yml = teach_codemap_yml(&role_patterns, &proof_changed);
    TeachReport {
        kind: "teach_report",
        schema_version: "1",
        config: project.config_path.clone(),
        role_patterns,
        proof_changed,
        codemap_yml,
        expand: vec![
            "codemap anchors validate".to_string(),
            "codemap status".to_string(),
            "codemap proof changed".to_string(),
        ],
    }
}

fn teach_role_patterns(project: &Project) -> Vec<TeachRolePattern> {
    let mut grouped: BTreeMap<(String, String, String), Vec<String>> = BTreeMap::new();
    for file in project.files.values() {
        for role in &file.roles {
            let Some((pattern, evidence)) = teach_pattern_for_role(file, role) else {
                continue;
            };
            if project
                .anchors
                .roles
                .iter()
                .any(|(existing, existing_role)| existing == &pattern && existing_role == role)
            {
                continue;
            }
            grouped
                .entry((pattern, role.clone(), evidence.to_string()))
                .or_default()
                .push(file.rel.clone());
        }
    }
    grouped
        .into_iter()
        .map(|((pattern, role, evidence), mut examples)| {
            examples.sort();
            let matched = examples.len();
            examples.truncate(5);
            TeachRolePattern {
                pattern,
                role,
                evidence,
                matched,
                examples,
            }
        })
        .take(12)
        .collect()
}

fn teach_pattern_for_role(file: &FileInfo, role: &str) -> Option<(String, &'static str)> {
    let path = Path::new(&file.rel);
    let name = path.file_name()?.to_str()?;
    let dir = path
        .parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| ".".to_string());
    let ext = if file.ext.is_empty() { "*" } else { &file.ext };
    match role {
        "receipt" if file.rel.contains("receipts/") => {
            Some((format!("{dir}/*.{ext}"), "receipt_path"))
        }
        "witness" if file.rel.contains("witnesses/") => {
            Some((format!("{dir}/*.{ext}"), "witness_path"))
        }
        "proof_runner" if name.starts_with("run_") => {
            Some((format!("{dir}/run_*.{ext}"), "proof_runner_path"))
        }
        "proof_runner" if name.starts_with("run-") => {
            Some((format!("{dir}/run-*.{ext}"), "proof_runner_path"))
        }
        "owner_doc" if name.starts_with("qwen-") => {
            Some((format!("{dir}/qwen-*.{ext}"), "owner_doc_path"))
        }
        "migration" if file.rel.contains("migrations/") => {
            Some((format!("{dir}/*.{ext}"), "migration_path"))
        }
        _ => None,
    }
}

fn teach_proof_commands(project: &Project) -> Vec<TeachProofCommand> {
    project
        .scripts
        .iter()
        .filter(|script| teach_command_is_validation(&script.command))
        .map(|script| TeachProofCommand {
            command: script.command.clone(),
            evidence: script.reason.clone(),
            source: script.path.clone(),
            line_start: script.line_start,
        })
        .take(8)
        .collect()
}

fn teach_command_is_validation(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    if [
        "deploy", "release", "publish", "destroy", "delete", "drop", "migrate",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        return false;
    }
    [
        "test", "check", "lint", "validate", "verify", "doctor", "proof",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn teach_codemap_yml(roles: &[TeachRolePattern], proof: &[TeachProofCommand]) -> Vec<String> {
    let mut lines = vec!["version: 1".to_string()];
    if !roles.is_empty() {
        lines.push("roles:".to_string());
        for role in roles.iter().take(8) {
            lines.push(format!(
                "  \"{}\": {}",
                yaml_quote(&role.pattern),
                role.role
            ));
        }
    }
    if !proof.is_empty() {
        lines.push("proof:".to_string());
        lines.push("  changed:".to_string());
        for command in proof.iter().take(5) {
            lines.push(format!("    - {}", yaml_string(&command.command)));
        }
    }
    lines
}

fn yaml_quote(value: &str) -> String {
    value.replace('"', "\\\"")
}

fn yaml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}
