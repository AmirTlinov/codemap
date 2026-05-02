fn same_scope_file_references_symbol(
    anchor: &FileInfo,
    file: &FileInfo,
    symbol_name: &str,
) -> bool {
    !file.resolved_imports.contains(&anchor.rel)
        && same_symbol_reference_scope(anchor, file)
        && file.references.contains(symbol_name)
}

fn structural_roles_for_ls(info: &FileInfo) -> Vec<String> {
    info.roles.iter().cloned().collect()
}

fn package_name_for_file(project: &Project, rel: &str) -> Option<String> {
    project
        .packages
        .iter()
        .filter(|package| {
            rel == package.path
                || rel == package.manifest
                || package.path == "."
                || rel.starts_with(&format!("{}/", package.path.trim_end_matches('/')))
        })
        .max_by_key(|package| {
            if package.path == "." {
                0
            } else {
                package.path.len()
            }
        })
        .map(|package| package.name.clone())
}

fn file_kind_for_ls(info: &FileInfo) -> String {
    for role in [
        "snapshot",
        "golden",
        "asset",
        "e2e_test",
        "test_support",
        "test",
        "manifest",
        "schema_contract",
        "public_boundary",
        "env_config",
        "runtime_config",
        "lockfile",
        "docs",
        "runtime_state",
        "adapter",
        "parser",
        "renderer_ui",
        "persistence",
        "repo_discovery",
        "cache",
        "build_ci",
        "semantic_anchor",
        "agent_bootstrap",
        "fixture",
        "example",
        "generated",
    ] {
        if info.has_role(role) {
            return role.to_string();
        }
    }
    if repo::is_source_ext(&info.ext) {
        "source".to_string()
    } else if info.language == "style" {
        "style".to_string()
    } else if info.language == "asset" {
        "asset".to_string()
    } else if info.language == "snapshot" {
        "snapshot".to_string()
    } else if info.language == "config" {
        "config".to_string()
    } else if info.language == "env" {
        "env_config".to_string()
    } else if info.language == "schema" {
        "schema_contract".to_string()
    } else if info.language == "lockfile" {
        "lockfile".to_string()
    } else if info.language == "markdown" {
        "docs".to_string()
    } else {
        "file".to_string()
    }
}

fn is_generic_noise(info: &FileInfo) -> bool {
    repo::is_source_ext(&info.ext)
        && info.roles.is_empty()
        && info.imports.is_empty()
        && info.exports.is_empty()
        && info.symbols.is_empty()
}
