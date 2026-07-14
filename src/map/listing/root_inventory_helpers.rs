// Responsibility: map-listing-root-inventory-helpers
use crate::map::{
    command_target, manifest_dir_for_rel, manifest_file_name, script_target_for_path,
    structural_edge_with_locations,
};
use crate::model::{EvidenceLocation, EvidenceStrength, StructuralEdge};
use std::path::Path;

pub(crate) fn inventory_package_kind(root: &Path, rel: &str) -> Option<String> {
    match manifest_file_name(rel) {
        "package.json" => Some("package:javascript".to_string()),
        "Cargo.toml" => std::fs::read_to_string(root.join(rel))
            .ok()
            .filter(|text| text.contains("[package]"))
            .map(|_| "package:rust".to_string()),
        "go.mod" => Some("package:go".to_string()),
        "pyproject.toml" => Some("package:python".to_string()),
        "Package.swift" => Some("package:swift".to_string()),
        _ => None,
    }
}

pub(crate) fn inventory_recursive_structural_kind(kind: &str, rel: &str) -> bool {
    matches!(
        kind,
        "manifest"
            | "lockfile"
            | "env_config"
            | "runtime_config"
            | "build_ci"
            | "schema_contract"
            | "migration"
    ) || rel.starts_with("contracts/")
}

pub(crate) fn inventory_root_script_edges(
    root: &Path,
    files: &[String],
) -> (Vec<String>, Vec<StructuralEdge>) {
    let mut labels = Vec::new();
    let mut edges = Vec::new();
    if files.iter().any(|rel| rel == "package.json") {
        inventory_package_json_script_edges(root, "package.json", &mut labels, &mut edges);
    }
    if files.iter().any(|rel| rel == "Cargo.toml")
        && let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml"))
        && (text.contains("[package]") || text.contains("[workspace]"))
    {
        inventory_command_edges(
            "Cargo.toml",
            "test",
            "cargo test",
            1,
            &mut labels,
            &mut edges,
        );
    }
    for rel in ["Makefile", "justfile"] {
        if files.iter().any(|file| file == rel) {
            inventory_target_file_edges(root, rel, &mut labels, &mut edges);
        }
    }
    inventory_ci_run_edges(root, files, &mut edges);
    (labels, edges)
}

fn inventory_package_json_script_edges(
    root: &Path,
    rel: &str,
    labels: &mut Vec<String>,
    edges: &mut Vec<StructuralEdge>,
) {
    let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
        return;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(scripts) = value.get("scripts").and_then(|scripts| scripts.as_object()) else {
        return;
    };
    for (name, value) in scripts {
        let Some(command) = value
            .as_str()
            .map(str::trim)
            .filter(|command| !command.is_empty())
        else {
            continue;
        };
        if !inventory_script_name_relevant(name) && !inventory_command_relevant(command) {
            continue;
        }
        let line = inventory_json_key_line(&text, name).unwrap_or(1);
        inventory_command_edges(rel, name, command, line, labels, edges);
    }
}

fn inventory_target_file_edges(
    root: &Path,
    rel: &str,
    labels: &mut Vec<String>,
    edges: &mut Vec<StructuralEdge>,
) {
    let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
        return;
    };
    for (index, line) in text.lines().enumerate() {
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((name, _)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty()
            || name.starts_with('.')
            || name.starts_with('@')
            || name.contains(' ')
            || name.contains('$')
            || name.contains('=')
            || (!inventory_script_name_relevant(name) && !inventory_command_relevant(name))
        {
            continue;
        }
        inventory_command_edges(
            rel,
            name,
            &format!("{} {}", inventory_runner_for_target_file(rel), name),
            index + 1,
            labels,
            edges,
        );
    }
}

fn inventory_ci_run_edges(root: &Path, files: &[String], edges: &mut Vec<StructuralEdge>) {
    for rel in files.iter().filter(|rel| inventory_ci_path(rel)) {
        let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start().trim_start_matches("-").trim_start();
            let Some(command) = trimmed.strip_prefix("run:").map(str::trim) else {
                continue;
            };
            if command.is_empty() {
                continue;
            }
            if matches!(command, "|" | ">") {
                edges.push(structural_edge_with_locations(
                    rel.clone(),
                    format!("ci_run_block:{rel}:{}", index + 1),
                    "declares_run_block",
                    "ci_run_step",
                    EvidenceStrength::High,
                    vec![EvidenceLocation::line(rel, index + 1, "ci_run_step")],
                ));
            } else {
                edges.push(structural_edge_with_locations(
                    rel.clone(),
                    command_target(command),
                    "runs_command",
                    "ci_run_step",
                    EvidenceStrength::High,
                    vec![EvidenceLocation::line(rel, index + 1, "ci_run_step")],
                ));
            }
        }
    }
}

fn inventory_command_edges(
    rel: &str,
    name: &str,
    command: &str,
    line: usize,
    labels: &mut Vec<String>,
    edges: &mut Vec<StructuralEdge>,
) {
    labels.push(format!("{name}: {command}"));
    let script = script_target_for_path(rel, name);
    edges.push(structural_edge_with_locations(
        rel.to_string(),
        script.clone(),
        "declares_script",
        "root_inventory_script",
        EvidenceStrength::Hard,
        vec![EvidenceLocation::line(rel, line, "script_declaration")],
    ));
    edges.push(structural_edge_with_locations(
        script,
        command_target(command),
        "runs_command",
        "root_inventory_script",
        EvidenceStrength::Hard,
        vec![EvidenceLocation::line(rel, line, "script_declaration")],
    ));
}

pub(crate) fn inventory_workspace_edges(root: &Path, files: &[String]) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let package_manifests = files
        .iter()
        .filter(|rel| manifest_file_name(rel) == "package.json" && rel.as_str() != "package.json")
        .collect::<Vec<_>>();
    let patterns = inventory_workspace_patterns(root);
    for (manifest, pattern, line) in patterns {
        for package_manifest in &package_manifests {
            let package_dir = manifest_dir_for_rel(package_manifest);
            if inventory_workspace_pattern_matches(&package_dir, &pattern) {
                edges.push(structural_edge_with_locations(
                    manifest.clone(),
                    (*package_manifest).clone(),
                    "workspace_member",
                    "root_inventory_workspace_pattern",
                    EvidenceStrength::Hard,
                    vec![EvidenceLocation::line(&manifest, line, "workspace_pattern")],
                ));
            }
        }
    }
    edges
}

fn inventory_workspace_patterns(root: &Path) -> Vec<(String, String, usize)> {
    let mut patterns = Vec::new();
    if let Ok(text) = std::fs::read_to_string(root.join("package.json"))
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(workspaces) = value.get("workspaces")
    {
        if let Some(array) = workspaces.as_array() {
            for pattern in array.iter().filter_map(|item| item.as_str()) {
                patterns.push((
                    "package.json".to_string(),
                    pattern.to_string(),
                    inventory_json_string_line(&text, pattern).unwrap_or(1),
                ));
            }
        } else if let Some(array) = workspaces.get("packages").and_then(|item| item.as_array()) {
            for pattern in array.iter().filter_map(|item| item.as_str()) {
                patterns.push((
                    "package.json".to_string(),
                    pattern.to_string(),
                    inventory_json_string_line(&text, pattern).unwrap_or(1),
                ));
            }
        }
    }
    if let Ok(text) = std::fs::read_to_string(root.join("pnpm-workspace.yaml")) {
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(pattern) = trimmed.strip_prefix("-").map(str::trim) {
                patterns.push((
                    "pnpm-workspace.yaml".to_string(),
                    pattern.trim_matches(['"', '\'']).to_string(),
                    index + 1,
                ));
            }
        }
    }
    patterns
}

fn inventory_workspace_pattern_matches(path: &str, pattern: &str) -> bool {
    let path_parts = path.split('/').collect::<Vec<_>>();
    let pattern_parts = pattern.trim_end_matches('/').split('/').collect::<Vec<_>>();
    if path_parts.len() != pattern_parts.len() {
        return false;
    }
    path_parts
        .iter()
        .zip(pattern_parts.iter())
        .all(|(path, pattern)| *pattern == "*" || *path == *pattern)
}

fn inventory_script_name_relevant(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    ["test", "type", "lint", "check", "build", "e2e"]
        .iter()
        .any(|term| name.contains(term))
}

fn inventory_command_relevant(command: &str) -> bool {
    let command = command.to_ascii_lowercase();
    [
        "test",
        "typecheck",
        "tsc",
        "lint",
        "check",
        "build",
        "clippy",
        "fmt",
    ]
    .iter()
    .any(|term| command.contains(term))
}

fn inventory_runner_for_target_file(rel: &str) -> &'static str {
    if rel.eq_ignore_ascii_case("justfile") {
        "just"
    } else {
        "make"
    }
}

fn inventory_json_key_line(text: &str, key: &str) -> Option<usize> {
    inventory_json_string_line(text, &format!("\"{key}\""))
        .or_else(|| inventory_json_string_line(text, key))
}

fn inventory_json_string_line(text: &str, needle: &str) -> Option<usize> {
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .map(|(index, _)| index + 1)
}

pub(crate) fn inventory_lockfile_name(name: &str) -> bool {
    matches!(
        name,
        "package-lock.json"
            | "npm-shrinkwrap.json"
            | "pnpm-lock.yaml"
            | "pnpm-lock.yml"
            | "yarn.lock"
            | "bun.lock"
            | "bun.lockb"
            | "cargo.lock"
            | "poetry.lock"
            | "pdm.lock"
            | "uv.lock"
            | "gemfile.lock"
            | "composer.lock"
    ) || name.ends_with(".lock")
}

pub(crate) fn inventory_ci_path(rel: &str) -> bool {
    let rel = rel.to_ascii_lowercase();
    rel.starts_with(".github/workflows/")
        || rel.starts_with(".circleci/")
        || rel.starts_with(".buildkite/")
}

pub(crate) fn inventory_runtime_config_path(rel: &str, name: &str) -> bool {
    matches!(
        name,
        "dockerfile"
            | "docker-compose.yml"
            | "docker-compose.yaml"
            | "compose.yml"
            | "compose.yaml"
            | "kustomization.yaml"
            | "kustomization.yml"
    ) || rel.starts_with("deploy/")
        || rel.starts_with("deployment/")
        || rel.starts_with("infra/")
        || rel.contains("/k8s/")
}

pub(crate) fn inventory_schema_path(rel: &str, ext: &str) -> bool {
    matches!(ext, "prisma" | "graphql" | "gql" | "proto" | "avsc") || rel.starts_with("contracts/")
}

pub(crate) fn inventory_migration_path(rel: &str, ext: &str) -> bool {
    ext == "sql" && (rel.starts_with("migrations/") || rel.contains("/migrations/"))
}

pub(crate) fn inventory_support_unit(rel: &str) -> String {
    let mut parts = Vec::new();
    for part in rel.split('/') {
        parts.push(part);
        if matches!(
            part,
            "fixtures" | "examples" | "samples" | ".agents" | ".codex" | ".claude"
        ) {
            return format!("{}/", parts.join("/"));
        }
    }
    rel.to_string()
}
