// Responsibility: map-listing-root-inventory-classify
use crate::map::{
    directory_surface_evidence, directory_surface_id, directory_surface_path,
    directory_surface_role, directory_surface_strength, inventory_ci_path, inventory_lockfile_name,
    inventory_migration_path, inventory_runtime_config_path, inventory_schema_path,
    manifest_file_name, surface_priority,
};
use crate::model::{DirectorySurface, StructuralEdge};
use crate::repo;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn inventory_edge_priority(edge: &StructuralEdge) -> usize {
    match edge.edge_type.as_str() {
        "runs_command" => 0,
        "declares_script" => 1,
        "workspace_member" => 2,
        "declares_run_block" => 8,
        _ => 9,
    }
}

pub(crate) fn inventory_top_level_dirs(files: &[String]) -> Vec<String> {
    let mut dirs = BTreeSet::new();
    for rel in files {
        if let Some((dir, _)) = rel.split_once('/') {
            dirs.insert(format!("{dir}/"));
        }
    }
    dirs.into_iter().collect()
}

pub(crate) fn inventory_push(
    grouped: &mut BTreeMap<String, BTreeSet<String>>,
    kind: &str,
    value: &str,
) {
    grouped
        .entry(kind.to_string())
        .or_default()
        .insert(value.to_string());
}

pub(crate) fn inventory_surfaces(
    scope: &str,
    grouped: BTreeMap<String, BTreeSet<String>>,
    include_all_examples: bool,
) -> Vec<DirectorySurface> {
    let mut surfaces = grouped
        .into_iter()
        .map(|(kind, files)| {
            let count = files.len();
            let examples = if include_all_examples {
                files.into_iter().collect::<Vec<_>>()
            } else {
                files.into_iter().take(5).collect::<Vec<_>>()
            };
            let shown = examples.len();
            DirectorySurface {
                id: directory_surface_id(scope, &kind, &examples),
                // Script examples are `name: command` labels, not paths; the
                // owner report fills the defining rail path afterwards.
                path: if kind == "script" {
                    None
                } else {
                    directory_surface_path(&examples)
                },
                role: directory_surface_role(&kind),
                evidence: directory_surface_evidence(&kind),
                strength: directory_surface_strength(&kind),
                kind,
                count,
                examples,
                hidden_count: count.saturating_sub(shown),
            }
        })
        .collect::<Vec<_>>();
    surfaces.sort_by(|a, b| {
        surface_priority(&a.kind)
            .cmp(&surface_priority(&b.kind))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    surfaces
}

pub(crate) fn inventory_dir_role(dir: &str) -> Option<String> {
    let name = dir.trim_end_matches('/').to_ascii_lowercase();
    match name.as_str() {
        ".github" | ".circleci" | ".buildkite" => Some("build_ci".to_string()),
        "docs" | "doc" | "documentation" => Some("docs".to_string()),
        "contracts" | "schemas" | "schema" | "migrations" => Some("schema_contract".to_string()),
        "deploy" | "deployment" | "infra" | "k8s" => Some("deploy".to_string()),
        "fixtures" | "examples" | "samples" => Some("fixture".to_string()),
        _ => None,
    }
}

pub(crate) fn inventory_file_kind(rel: &str) -> String {
    let path = Path::new(rel);
    let name = manifest_file_name(rel).to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if repo::is_env_surface_name(&name) {
        return "env_config".to_string();
    }
    if inventory_lockfile_name(&name) {
        return "lockfile".to_string();
    }
    if matches!(
        name.as_str(),
        "package.json"
            | "cargo.toml"
            | "go.mod"
            | "go.work"
            | "pyproject.toml"
            | "requirements.txt"
            | "package.swift"
            | "pnpm-workspace.yaml"
            | "pnpm-workspace.yml"
    ) {
        return "manifest".to_string();
    }
    if inventory_ci_path(rel) {
        return "build_ci".to_string();
    }
    if inventory_runtime_config_path(rel, &name) {
        return "runtime_config".to_string();
    }
    if inventory_schema_path(rel, &ext) {
        return "schema_contract".to_string();
    }
    if inventory_migration_path(rel, &ext) {
        return "migration".to_string();
    }
    if ext == "md" {
        return "docs".to_string();
    }
    if repo::is_script_ext(&ext) || matches!(name.as_str(), "makefile" | "justfile") {
        return "script".to_string();
    }
    if repo::is_source_ext(&ext) {
        return "source".to_string();
    }
    if repo::is_asset_ext(&ext) {
        return "asset".to_string();
    }
    if matches!(ext.as_str(), "json" | "toml" | "yaml" | "yml") {
        return "config".to_string();
    }
    "file".to_string()
}
