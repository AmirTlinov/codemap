// Responsibility: map-listing-ls-surface-meta
use crate::map::shell_quote;
use crate::model::EvidenceStrength;

pub(crate) fn directory_next_commands(rel: &str) -> Vec<String> {
    let graph = if rel == "." {
        "codemap graph --lens causal".to_string()
    } else {
        format!("codemap graph --path {} --lens causal", shell_quote(rel))
    };
    vec![
        graph,
        format!("codemap cone {} --depth 1", shell_quote(rel)),
    ]
}

pub(crate) fn directory_surface_id(scope: &str, kind: &str, examples: &[String]) -> String {
    let anchor = examples
        .first()
        .map(String::as_str)
        .unwrap_or(scope)
        .trim_end_matches('/');
    format!("surface:{kind}:{anchor}")
}

pub(crate) fn directory_surface_path(examples: &[String]) -> Option<String> {
    (examples.len() == 1).then(|| examples[0].clone())
}

// Script surface examples are `name: command` labels, not paths. The surface
// path must stay a real path: the single manifest/rail file that defines the
// scripts, or None when they span several files.
pub(crate) fn script_surface_path(project: &crate::model::Project) -> Option<String> {
    let mut defining_paths = project
        .scripts
        .iter()
        .map(|script| script.path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if defining_paths.len() != 1 {
        return None;
    }
    defining_paths.pop_first().flatten()
}

pub(crate) fn directory_surface_role(kind: &str) -> Option<String> {
    if kind == "domain" {
        Some("domain".to_string())
    } else if kind == "script" {
        Some("script".to_string())
    } else if kind == "dir" {
        Some("container".to_string())
    } else if matches!(
        kind,
        "runtime_container"
            | "contract_container"
            | "data_container"
            | "deployment_container"
            | "verification_container"
    ) {
        Some(kind.trim_end_matches("_container").to_string())
    } else if kind.starts_with("package:") || kind.starts_with("support_package:") {
        Some("package".to_string())
    } else if matches!(
        kind,
        "test"
            | "e2e_test"
            | "test_support"
            | "source"
            | "snapshot"
            | "golden"
            | "asset"
            | "config"
            | "docs"
            | "env_config"
            | "runtime_config"
            | "manifest"
            | "lockfile"
            | "schema_contract"
            | "schema"
            | "public_boundary"
            | "public_api"
            | "internal_api"
            | "build_ci"
            | "deploy"
            | "entrypoint"
            | "runtime_surface"
            | "application"
            | "service"
            | "domain"
            | "controller"
            | "module"
            | "repository"
            | "adapter"
            | "parser"
            | "renderer_ui"
            | "persistence"
            | "package_graph"
            | "role_classifier"
            | "script_catalog"
            | "cli_surface"
            | "map_surface"
            | "extractor"
            | "config_loader"
            | "evidence_surface"
            | "repo_discovery"
            | "cache"
            | "semantic_anchor"
            | "agent_bootstrap"
            | "fixture"
            | "example"
            | "generated"
            | "archive"
            | "witness"
            | "receipt"
            | "proof_runner"
            | "owner_doc"
            | "migration"
            | "build_output"
            | "agent_support"
            | "file"
            | "style"
    ) {
        Some(kind.to_string())
    } else {
        None
    }
}

pub(crate) fn directory_surface_evidence(kind: &str) -> String {
    if kind == "domain" {
        "domain_boundary".to_string()
    } else if kind == "script" {
        "package_script".to_string()
    } else if kind == "dir" {
        "directory_inventory".to_string()
    } else if kind.ends_with("_container") {
        "current_level_atlas".to_string()
    } else if kind.starts_with("package:") || kind.starts_with("support_package:") {
        "package_manifest".to_string()
    } else if kind == "manifest" {
        "manifest_file".to_string()
    } else if kind == "env_config" {
        "env_file".to_string()
    } else if kind == "runtime_config" {
        "runtime_config_file".to_string()
    } else if kind == "lockfile" {
        "lockfile".to_string()
    } else if kind.starts_with("recursive:") {
        "recursive_inventory".to_string()
    } else {
        "file_role_or_extension".to_string()
    }
}

pub(crate) fn directory_surface_strength(kind: &str) -> EvidenceStrength {
    if kind == "script" || kind.starts_with("package:") || kind.starts_with("support_package:") {
        EvidenceStrength::Hard
    } else if kind == "domain"
        || kind.ends_with("_container")
        || kind == "schema_contract"
        || kind == "public_boundary"
        || kind == "manifest"
        || kind == "env_config"
        || kind == "runtime_config"
        || kind == "lockfile"
        || kind == "build_ci"
    {
        EvidenceStrength::High
    } else {
        EvidenceStrength::Medium
    }
}
