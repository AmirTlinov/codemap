fn classify_roles(root: &Path, info: &mut FileInfo) {
    let rel = info.rel.to_ascii_lowercase();
    let name = Path::new(&info.rel)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if is_generated(&rel) {
        info.roles.insert("generated".to_string());
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
    if is_schema_contract_surface(&rel, &name, &info.ext) {
        info.roles.insert("schema_contract".to_string());
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
    add_role_if(
        &mut info.roles,
        &rel,
        &["render", "view", "component", "page", "screen", "ui"],
        "renderer_ui",
    );
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
    if is_build_ci_surface(&rel, &name, &info.tokens) {
        info.roles.insert("build_ci".to_string());
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

fn is_schema_contract_surface(rel: &str, name: &str, ext: &str) -> bool {
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

fn is_build_ci_surface(rel: &str, name: &str, tokens: &BTreeSet<String>) -> bool {
    rel.starts_with(".github/workflows/")
        || rel.starts_with(".circleci/")
        || rel.starts_with(".buildkite/")
        || rel.starts_with(".teamcity/")
        || matches!(
            name,
            ".gitlab-ci.yml"
                | ".gitlab-ci.yaml"
                | "azure-pipelines.yml"
                | "azure-pipelines.yaml"
                | "bitbucket-pipelines.yml"
                | "bitbucket-pipelines.yaml"
                | ".drone.yml"
                | ".drone.yaml"
                | ".woodpecker.yml"
                | ".woodpecker.yaml"
                | "jenkinsfile"
                | "dockerfile"
                | "docker-compose.yml"
                | "docker-compose.yaml"
                | "compose.yml"
                | "compose.yaml"
                | "makefile"
                | "justfile"
                | "taskfile"
                | "taskfile.yml"
                | "taskfile.yaml"
                | "earthfile"
        )
        || tokens.contains("build")
        || tokens.contains("ci")
        || tokens.contains("workflow")
}

fn is_generated(rel: &str) -> bool {
    rel.contains(".generated.")
        || rel.contains(".gen.")
        || rel.contains("/generated/")
        || rel.ends_with(".pb.go")
        || rel.ends_with(".g.dart")
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
