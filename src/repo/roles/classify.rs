// Responsibility: repo-roles-classify
use crate::model::FileInfo;
use crate::repo::{
    classify_source_roles, has_generated_header, is_asset_ext, is_build_ci_surface,
    is_deploy_surface, is_docs_surface, is_doctor_surface, is_e2e_test_path, is_entrypoint_surface,
    is_env_surface_name, is_generated, is_golden_surface, is_internal_api_surface,
    is_lockfile_name, is_migration_surface, is_owner_doc_surface, is_package_manifest_name,
    is_proof_runner_surface, is_receipt_surface, is_runtime_config_surface, is_runtime_surface,
    is_schema_contract_surface, is_script_ext, is_snapshot_surface, is_source_ext, is_test_path,
    is_test_support_path, is_witness_surface, source_has_test_declaration,
};
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn classify_roles(root: &Path, info: &mut FileInfo) {
    let rel = info.rel.to_ascii_lowercase();
    let name = Path::new(&info.rel)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_generated(&rel) || has_generated_header(root, info) {
        info.roles.insert("generated".to_string());
    }
    if is_asset_ext(&info.ext) {
        info.roles.insert("asset".to_string());
    }
    if is_snapshot_surface(&rel, &name, &info.ext) {
        info.roles.insert("snapshot".to_string());
    }
    if is_golden_surface(&rel) {
        info.roles.insert("golden".to_string());
    }
    if rel.starts_with("fixtures/") || rel.contains("/fixtures/") {
        info.roles.insert("fixture".to_string());
    }
    if rel.starts_with("examples/")
        || rel.contains("/examples/")
        || rel.starts_with("samples/")
        || rel.contains("/samples/")
    {
        info.roles.insert("example".to_string());
    }
    if is_test_path(&rel, &info.ext) && is_source_ext(&info.ext) {
        let support_like =
            is_test_support_path(&rel) || name == "__init__.py" || name == "conftest.py";
        if support_like && !source_has_test_declaration(root, info) {
            info.roles.insert("test_support".to_string());
        } else {
            info.roles.insert("test".to_string());
            if is_e2e_test_path(&rel) {
                info.roles.insert("e2e_test".to_string());
            }
        }
    }
    if matches!(
        name.as_str(),
        "index.ts"
            | "index.tsx"
            | "index.js"
            | "index.jsx"
            | "mod.rs"
            | "lib.rs"
            | "main.rs"
            | "main.go"
            | "__init__.py"
            | "api.ts"
            | "routes.ts"
            | "package.json"
            | "cargo.toml"
            | "go.mod"
            | "pyproject.toml"
            | "package.swift"
    ) {
        info.roles.insert("public_boundary".to_string());
    }
    if is_package_manifest_name(&name) {
        info.roles.insert("manifest".to_string());
    }
    if is_env_surface_name(&name) {
        info.roles.insert("env_config".to_string());
        info.roles.insert("runtime_config".to_string());
    }
    if is_lockfile_name(&name) {
        info.roles.insert("lockfile".to_string());
    }
    if is_docs_surface(&rel, &name, &info.ext) {
        info.roles.insert("docs".to_string());
    }
    if is_script_ext(&info.ext) {
        info.roles.insert("script".to_string());
    }
    if is_receipt_surface(&rel, &name, &info.ext) {
        info.roles.insert("receipt".to_string());
    }
    if is_witness_surface(&rel, &name, &info.ext) {
        info.roles.insert("witness".to_string());
    }
    if is_owner_doc_surface(&rel, &name, &info.ext) {
        info.roles.insert("owner_doc".to_string());
    }
    if is_proof_runner_surface(&rel, &name, &info.ext, &info.tokens) {
        info.roles.insert("proof_runner".to_string());
    }
    if is_migration_surface(&rel, &name, &info.ext) {
        info.roles.insert("migration".to_string());
    }
    if is_deploy_surface(&rel, &name, &info.ext) {
        info.roles.insert("deploy".to_string());
    }
    if is_entrypoint_surface(&rel, &name, &info.ext) {
        info.roles.insert("entrypoint".to_string());
    }
    if is_source_ext(&info.ext) {
        classify_source_roles(root, info, &rel, &name);
        add_role_if(
            &mut info.roles,
            &rel,
            &[
                "state",
                "store",
                "model",
                "entity",
                "timeline",
                "reducer",
                "machine",
                "registry",
                "repository",
                "project",
                "aggregate",
            ],
            "state_model",
        );
        add_role_if(
            &mut info.roles,
            &rel,
            &["session", "cursor", "clock", "controller", "manager"],
            "runtime_state",
        );
    }
    if !info.roles.contains("manifest") && is_schema_contract_surface(&rel, &name, &info.ext) {
        info.roles.insert("schema_contract".to_string());
        info.roles.insert("schema".to_string());
    }
    if !is_docs_ext(&info.ext) {
        add_role_if(
            &mut info.roles,
            &rel,
            &["adapter", "gateway", "client", "provider", "port", "driver"],
            "adapter",
        );
        add_role_if(
            &mut info.roles,
            &rel,
            &["parser", "parse", "loader", "reader", "decoder"],
            "parser",
        );
        add_role_if(
            &mut info.roles,
            &rel,
            &[
                "save",
                "load",
                "reopen",
                "persist",
                "persistence",
                "storage",
            ],
            "persistence",
        );
        add_role_if(
            &mut info.roles,
            &rel,
            &["root", "inventory", "files", "discover", "discovery"],
            "repo_discovery",
        );
        add_role_if(&mut info.roles, &rel, &["cache", "fingerprint"], "cache");
        add_role_if(&mut info.roles, &rel, &["cli", "command"], "cli_surface");
    }
    add_renderer_ui_role_if(&mut info.roles, &rel, &info.ext, &info.tokens);
    if matches!(name.as_str(), "repo.rs" | "repo.ts" | "repo.js") {
        info.roles.insert("repo_discovery".to_string());
    }
    if is_build_ci_surface(&rel, &name, &info.ext, &info.tokens) {
        info.roles.insert("build_ci".to_string());
    }
    if is_runtime_config_surface(&rel, &name) {
        info.roles.insert("runtime_config".to_string());
    }
    if is_runtime_surface(info) {
        info.roles.insert("runtime_surface".to_string());
    }
    if info.roles.contains("public_boundary") {
        info.roles.insert("public_api".to_string());
    }
    if is_internal_api_surface(&rel, &name, &info.ext) {
        info.roles.insert("internal_api".to_string());
    }
    if is_doctor_surface(&rel, &name, &info.ext) {
        info.roles.insert("doctor".to_string());
    }
    if name == "agents.md" {
        info.roles.insert("agent_bootstrap".to_string());
    }
    if matches!(
        name.as_str(),
        ".codemap.yml" | ".codemap.yaml" | ".codemap.json"
    ) {
        info.roles.insert("semantic_anchor".to_string());
    }
    if info.roles.contains("test") {
        for role in [
            "state_model",
            "runtime_state",
            "public_boundary",
            "adapter",
            "schema_contract",
            "parser",
            "renderer_ui",
            "persistence",
            "repo_discovery",
            "cache",
            "cli_surface",
            "build_ci",
            "proof_runner",
            "script",
            "entrypoint",
            "runtime_surface",
            "public_api",
            "internal_api",
            "doctor",
        ] {
            info.roles.remove(role);
        }
    }
}

pub(crate) fn base_roles_for_cache(root: &Path, info: &FileInfo) -> BTreeSet<String> {
    let mut base = info.clone();
    base.roles.clear();
    classify_roles(root, &mut base);
    base.roles
}

fn add_role_if(roles: &mut BTreeSet<String>, rel: &str, needles: &[&str], role: &str) {
    if needles
        .iter()
        .any(|needle| path_has_role_token(rel, needle))
    {
        roles.insert(role.to_string());
    }
}

// A needle only counts when it matches a whole path token (directory segment
// or `.`/`_`/`-`-separated file-name part), never a substring inside a word:
// `cone_reports.rs` must not match `port`.
fn path_has_role_token(rel: &str, needle: &str) -> bool {
    rel.split('/')
        .flat_map(|segment| segment.split(['.', '_', '-']))
        .filter(|token| !token.is_empty())
        .any(|token| {
            token == needle
                || (token.len() == needle.len() + 1
                    && token.ends_with('s')
                    && token.starts_with(needle))
        })
}

fn is_docs_ext(ext: &str) -> bool {
    matches!(ext, "md" | "mdx" | "rst" | "txt")
}

fn add_renderer_ui_role_if(
    roles: &mut BTreeSet<String>,
    rel: &str,
    ext: &str,
    _tokens: &BTreeSet<String>,
) {
    if matches!(ext, "tsx" | "jsx" | "vue" | "svelte")
        || (matches!(ext, "ts" | "js") && path_has_ui_surface_convention(rel))
    {
        roles.insert("renderer_ui".to_string());
    }
}

fn path_has_ui_surface_convention(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    let path = Path::new(&lower);
    let has_ui_segment = path.components().any(|component| {
        let part = component.as_os_str().to_string_lossy();
        matches!(
            part.as_ref(),
            "pages"
                | "page"
                | "components"
                | "component"
                | "screens"
                | "screen"
                | "views"
                | "view"
                | "ui"
        )
    });
    if has_ui_segment {
        return true;
    }
    let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    stem.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .any(|part| matches!(part, "page" | "component" | "screen" | "view"))
}
