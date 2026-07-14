// Responsibility: anchor-role-hints
use crate::render::{bullet, disclaimer};

pub(crate) fn render_roles(anchor: &crate::model::FileSummary) {
    let roles = canonical_roles(anchor);
    if roles.is_empty() {
        return;
    }
    println!("\n## Surface Hints\n");
    disclaimer(
        "Derived from deterministic path/name/extension/manifest patterns. Not intent, correctness, or ownership truth.",
    );
    println!("{}", bullet(&roles, true, None));
}

pub(crate) fn canonical_roles(anchor: &crate::model::FileSummary) -> Vec<String> {
    let mut roles = std::collections::BTreeSet::new();
    let local = anchor.roles.iter().map(String::as_str).collect::<Vec<_>>();
    let path = anchor.path.to_ascii_lowercase();
    let support_artifact = support_artifact_hint_path(&path)
        || local.iter().any(|role| {
            matches!(
                *role,
                "receipt" | "witness" | "fixture" | "generated" | "archive" | "build_output"
            )
        });
    if local
        .iter()
        .any(|role| matches!(*role, "test" | "e2e_test" | "test_support"))
    {
        roles.insert("test".to_string());
    }
    if local.contains(&"public_boundary") || anchor.kind == "public_boundary" {
        roles.insert("public_boundary".to_string());
    }
    if local.contains(&"manifest") || matches!(path.as_str(), "package.json" | "cargo.toml") {
        roles.insert("manifest".to_string());
    }
    if local.contains(&"env_config") || anchor.kind == "env_config" || path.contains(".env") {
        roles.insert("env".to_string());
    }
    if local.contains(&"runtime_config")
        || anchor.kind == "runtime_config"
        || (!support_artifact && (anchor.kind == "config" || anchor.language == "config"))
    {
        roles.insert("config".to_string());
    }
    if local.contains(&"lockfile") || anchor.kind == "lockfile" {
        roles.insert("lockfile".to_string());
    }
    if local.contains(&"docs") || anchor.kind == "docs" || path.ends_with(".md") {
        roles.insert("docs".to_string());
    }
    if local.contains(&"schema_contract") || anchor.kind == "schema_contract" {
        roles.insert("schema".to_string());
    }
    if local.contains(&"build_ci") || anchor.kind == "build_ci" {
        roles.insert("ci".to_string());
    }
    if anchor.kind == "script" || anchor.path.starts_with("test: ") {
        roles.insert("script".to_string());
    }
    if local.contains(&"proof_runner") {
        roles.insert("proof_runner".to_string());
    }
    if local.contains(&"owner_doc") {
        roles.insert("owner_doc".to_string());
    }
    if local.contains(&"doctor") {
        roles.insert("doctor".to_string());
    }
    if local.contains(&"receipt") {
        roles.insert("receipt".to_string());
    }
    if local.contains(&"witness") {
        roles.insert("witness".to_string());
    }
    for role in [
        "application",
        "service",
        "domain",
        "controller",
        "module",
        "repository",
        "package_graph",
        "role_classifier",
        "script_catalog",
        "cli_surface",
        "map_surface",
        "render_surface",
        "helper_surface",
        "proof_surface",
        "contract_surface",
        "analysis_surface",
        "teach_surface",
        "extractor",
        "config_loader",
        "evidence_surface",
    ] {
        if local.contains(&role) || anchor.kind == role {
            roles.insert(role.to_string());
        }
    }
    if local.contains(&"fixture") || path.contains("/fixtures/") || path.starts_with("fixtures/") {
        roles.insert("fixture".to_string());
    }
    if local.contains(&"generated") {
        roles.insert("generated".to_string());
    }
    if path.contains("/archive/") || path.starts_with("archive/") || path.contains("/archives/") {
        roles.insert("archive".to_string());
    }
    if support_artifact_hint_path(&path) {
        roles.insert("witness".to_string());
    }
    if path.contains("/dist/")
        || path.starts_with("dist/")
        || path.contains("/build/")
        || path.starts_with("build/")
    {
        roles.insert("build_output".to_string());
    }
    if path.ends_with(".md") && (path.contains("/contracts/") || path.contains("contract")) {
        roles.insert("contract_doc".to_string());
    }
    if roles.is_empty() && looks_like_source_anchor(anchor) {
        roles.insert("source".to_string());
    }
    if roles.is_empty() {
        roles.insert("unknown".to_string());
    }
    roles.into_iter().collect()
}

fn support_artifact_hint_path(path: &str) -> bool {
    path.contains("/witness")
        || path.contains("/receipts/")
        || path.starts_with("receipts/")
        || path.contains("/witnesses/")
        || path.starts_with("witnesses/")
        || path.contains("/artifacts/")
        || path.starts_with("artifacts/")
        || path.contains("-proof/")
        || path.contains("/proof/")
}

fn looks_like_source_anchor(anchor: &crate::model::FileSummary) -> bool {
    anchor.kind == "source"
        || !anchor.symbols.is_empty()
        || !anchor.imports.is_empty()
        || !anchor.exports.is_empty()
        || matches!(
            anchor.language.as_str(),
            "rust"
                | "typescript"
                | "tsx"
                | "javascript"
                | "jsx"
                | "python"
                | "go"
                | "swift"
                | "kotlin"
                | "java"
                | "c"
                | "cpp"
        )
}
