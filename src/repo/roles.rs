fn classify_roles(info: &mut FileInfo) {
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
    if is_test_path(&rel) {
        info.roles.insert("test".to_string());
        if is_e2e_test_path(&rel) {
            info.roles.insert("e2e_test".to_string());
        }
        if is_test_support_path(&rel) || name == "__init__.py" || name == "conftest.py" {
            info.roles.insert("test_support".to_string());
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
    add_role_if(
        &mut info.roles,
        &rel,
        &[
            "schema",
            "contract",
            "dto",
            "types",
            "interface",
            "migration",
        ],
        "schema_contract",
    );
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
    rel.contains("/support/")
        || rel.contains("/helpers/")
        || rel.contains("/fixtures/")
        || rel.contains("/mocks/")
        || rel.contains("/setup")
        || rel.contains(".setup.")
}

