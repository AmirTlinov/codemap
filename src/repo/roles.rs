fn classify_roles(root: &Path, info: &mut FileInfo) {
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
    if is_test_path(&rel) && is_source_ext(&info.ext) {
        let support_like = is_test_support_path(&rel) || name == "__init__.py" || name == "conftest.py";
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
    add_renderer_ui_role_if(&mut info.roles, &rel, &info.ext, &info.tokens);
    add_role_if(
        &mut info.roles,
        &rel,
        &["save", "load", "reopen", "persist", "storage"],
        "persistence",
    );
    add_role_if(
        &mut info.roles,
        &rel,
        &["root", "inventory", "files", "discover"],
        "repo_discovery",
    );
    if matches!(name.as_str(), "repo.rs" | "repo.ts" | "repo.js") {
        info.roles.insert("repo_discovery".to_string());
    }
    add_role_if(&mut info.roles, &rel, &["cache", "fingerprint"], "cache");
    add_role_if(&mut info.roles, &rel, &["cli", "command"], "cli_surface");
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
    if matches!(name.as_str(), ".ctx.yml" | ".ctx.yaml" | ".ctx.json") {
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

fn add_role_if(roles: &mut BTreeSet<String>, haystack: &str, needles: &[&str], role: &str) {
    if needles.iter().any(|needle| haystack.contains(needle)) {
        roles.insert(role.to_string());
    }
}

fn add_renderer_ui_role_if(
    roles: &mut BTreeSet<String>,
    rel: &str,
    ext: &str,
    tokens: &BTreeSet<String>,
) {
    if matches!(ext, "tsx" | "jsx" | "vue" | "svelte")
        || ["render", "view", "component", "page", "screen"]
        .iter()
        .any(|needle| rel.contains(needle))
        || tokens.contains("ui")
    {
        roles.insert("renderer_ui".to_string());
    }
}

pub(crate) fn is_schema_contract_surface(rel: &str, name: &str, ext: &str) -> bool {
    if !is_contract_surface_ext(ext) {
        return false;
    }
    let path = Path::new(rel);
    let stem = contract_surface_stem(path, name);
    if matches!(
        stem.as_str(),
        "schema"
            | "schemas"
            | "dto"
            | "dtos"
            | "types"
            | "interface"
            | "interfaces"
            | "migration"
            | "migrations"
    ) {
        return true;
    }
    if [
        ".schema.",
        ".dto.",
        ".types.",
        ".interface.",
        ".migration.",
        ".contract.",
    ]
    .iter()
    .any(|marker| name.contains(marker))
    {
        return true;
    }
    path.parent()
        .into_iter()
        .flat_map(|parent| parent.components())
        .filter_map(|component| component.as_os_str().to_str())
        .map(str::to_ascii_lowercase)
        .any(|part| {
            matches!(
                part.as_str(),
                "schema"
                    | "schemas"
                    | "dto"
                    | "dtos"
                    | "types"
                    | "interfaces"
                    | "contract"
                    | "contracts"
                    | "migration"
                    | "migrations"
                )
        })
}

fn is_contract_surface_ext(ext: &str) -> bool {
    is_source_ext(ext)
        || matches!(
            ext,
            "json" | "yaml" | "yml" | "sql" | "prisma" | "graphql" | "gql" | "proto" | "avsc"
        )
}

fn contract_surface_stem(path: &Path, name: &str) -> String {
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.ends_with(".d.ts") {
        stem.trim_end_matches(".d").to_string()
    } else {
        stem
    }
}

fn is_generated(rel: &str) -> bool {
    rel.contains(".generated.")
        || rel.contains(".gen.")
        || rel.contains("/generated/")
        || rel.ends_with(".pb.go")
        || rel.ends_with(".g.dart")
}

fn has_generated_header(root: &Path, info: &FileInfo) -> bool {
    if !is_source_ext(&info.ext) && !TEXT_EXTS.iter().any(|ext| ext == &info.ext) {
        return false;
    }
    let Ok(text) = fs::read_to_string(root.join(&info.rel)) else {
        return false;
    };
    let header = text
        .lines()
        .take(8)
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("\n");
    header.contains("@generated")
        || header.contains("code generated")
        || header.contains("auto-generated")
        || header.contains("autogenerated")
        || (header.contains("generated") && header.contains("do not edit"))
}

fn is_test_path(rel: &str) -> bool {
    rel.contains("/tests/")
        || rel.contains("/test/")
        || rel.starts_with("tests/")
        || rel.starts_with("test/")
        || rel.contains("/__tests__/")
        || rel.contains(".test.")
        || rel.contains(".spec.")
        || rel.ends_with("_test.rs")
        || rel.ends_with("_test.go")
        || rel
            .rsplit('/')
            .next()
            .map(|name| name.starts_with("test_"))
            .unwrap_or(false)
}

fn is_e2e_test_path(rel: &str) -> bool {
    let rel = rel.to_ascii_lowercase();
    rel.contains("/e2e/")
        || rel.contains("/e2e-")
        || rel.contains(".e2e.")
        || rel.contains("/playwright/")
        || rel.contains("/cypress/")
}

fn is_test_support_path(rel: &str) -> bool {
    let rel = rel.to_ascii_lowercase();
    let name = Path::new(&rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    rel.contains("/support/")
        || rel.contains("/helpers/")
        || rel.contains("/fixtures/")
        || rel.contains("/mocks/")
        || rel.contains("/setup")
        || rel.contains(".setup.")
        || name.starts_with("support_")
        || name.starts_with("support-")
        || name.starts_with("helper_")
        || name.starts_with("helper-")
}

fn source_has_test_declaration(root: &Path, info: &FileInfo) -> bool {
    let Ok(text) = fs::read_to_string(root.join(&info.rel)) else {
        return false;
    };
    match info.ext.as_str() {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "vue" | "svelte" => {
            js_test_declaration_re().is_match(&code_without_comments_or_strings(&text, &info.ext))
        }
        "rs" => text.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("#[test")
                || (trimmed.starts_with("#[") && trimmed.contains("::test"))
        }),
        "py" => py_test_declaration_re().is_match(&text),
        "go" => go_test_declaration_re().is_match(&text),
        "swift" => swift_test_declaration_re().is_match(&text),
        _ => false,
    }
}

fn js_test_declaration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)(^|[^A-Za-z0-9_$])(test|it|describe)(\s*\.\s*describe)?\s*\("#)
            .expect("valid js test declaration regex")
    })
}

fn py_test_declaration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*(async\s+def|def)\s+test_[A-Za-z0-9_]*\s*\("#)
            .expect("valid python test declaration regex")
    })
}

fn go_test_declaration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*func\s+Test[A-Za-z0-9_]*\s*\("#)
            .expect("valid go test declaration regex")
    })
}

fn swift_test_declaration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?m)^\s*func\s+test[A-Za-z0-9_]*\s*\("#)
            .expect("valid swift test declaration regex")
    })
}
