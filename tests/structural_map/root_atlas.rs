// Responsibility: root-current-level-atlas-contract
#[test]
fn root_atlas_groups_owner_containers_and_keeps_package_crossings() {
    let repo = TempDir::new().expect("atlas repo");
    let cache = TempDir::new().expect("atlas cache");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"atlas-root","private":true,"workspaces":["apps/*","packages/*"]}"#,
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@atlas/api","dependencies":{"@atlas/core":"workspace:*"}}"#,
    );
    write(
        &repo.path().join("packages/core/package.json"),
        r#"{"name":"@atlas/core"}"#,
    );
    write(
        &repo.path().join("apps/api/src/server.ts"),
        "export const server = true;\n",
    );
    write(
        &repo.path().join("apps/api/openapi/service.yaml"),
        "openapi: 3.0.0\n",
    );
    write(
        &repo.path().join("apps/api/db/migrations/001.sql"),
        "create table atlas(id int);\n",
    );
    write(
        &repo.path().join("apps/api/tests/server.test.ts"),
        "test('server', () => {});\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "jobs:\n  test:\n    steps:\n      - run: npm test\n",
    );
    write(&repo.path().join("deploy/api.yaml"), "kind: Deployment\n");

    let ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    assert_surface_examples(&ls, "domain", &["apps/", "packages/"]);
    assert_surface_examples(
        &ls,
        "package:javascript",
        &["package.json", "apps/api/", "packages/core/"],
    );
    assert_surface_examples(&ls, "runtime_container", &["apps/api/"]);
    assert_surface_examples(&ls, "contract_container", &["apps/api/openapi/"]);
    assert_surface_examples(&ls, "data_container", &["apps/api/db/"]);
    assert_surface_examples(
        &ls,
        "deployment_container",
        &[".github/", "deploy/"],
    );
    assert_surface_examples(&ls, "verification_container", &["apps/api/tests/"]);
    assert!(
        ls["edges"].as_array().expect("atlas edges").iter().any(|edge| {
            edge["from"] == "apps/api/"
                && edge["to"] == "packages/core/"
                && edge["type"] == "package_internal"
                && edge["evidence"] == "package_manifest:@atlas/core"
        }),
        "package dependency must be a current-level owner crossing: {ls:#}"
    );
    assert!(
        ls["directory"]
            .as_array()
            .expect("surfaces")
            .iter()
            .flat_map(|surface| surface["examples"].as_array().expect("examples"))
            .all(|example| example != "apps/api/src/server.ts"),
        "root atlas must not dump recursive source files: {ls:#}"
    );

    let graph = run_json(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "causal", "--limit", "20", "--format", "json"],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    assert!(
        graph["edges"].as_array().expect("graph edges").iter().any(|edge| {
            edge["from"] == "apps/api/"
                && edge["to"] == "packages/core/"
                && edge["type"] == "package_internal"
        }),
        "root graph must project the same package crossing: {graph:#}"
    );
}

#[test]
fn rust_workspace_and_non_code_roots_keep_real_atlas_entries() {
    let rust = TempDir::new().expect("Rust atlas repo");
    let rust_cache = TempDir::new().expect("Rust atlas cache");
    git(rust.path(), &["init", "-q"]);
    write(
        &rust.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"2\"\n",
    );
    write(
        &rust.path().join("crates/core/Cargo.toml"),
        "[package]\nname=\"core\"\nversion=\"0.1.0\"\n",
    );
    write(
        &rust.path().join("crates/app/Cargo.toml"),
        "[package]\nname=\"app\"\nversion=\"0.1.0\"\n[dependencies]\ncore={path=\"../core\"}\n",
    );
    write(&rust.path().join("crates/core/src/lib.rs"), "pub fn core() {}\n");
    write(&rust.path().join("crates/app/src/main.rs"), "fn main() {}\n");
    let graph = run_json(
        rust.path(),
        rust_cache.path(),
        &["graph", "--lens", "causal", "--format", "json"],
    );
    assert!(graph["edges"].as_array().expect("Rust graph edges").iter().any(|edge| {
        edge["from"] == "crates/app/"
            && edge["to"] == "crates/core/"
            && edge["type"] == "package_internal"
    }), "Rust path dependency must survive the bounded atlas: {graph:#}");

    let docs = TempDir::new().expect("non-code atlas repo");
    let docs_cache = TempDir::new().expect("non-code atlas cache");
    git(docs.path(), &["init", "-q"]);
    write(&docs.path().join("README.md"), "# Docs\n");
    write(&docs.path().join("contracts/openapi.yaml"), "openapi: 3.0.0\n");
    write(&docs.path().join("data/migrations/001.sql"), "select 1;\n");
    write(&docs.path().join("infra/terraform/main.tf"), "resource \"x\" \"y\" {}\n");
    write(&docs.path().join("tests/contract.spec.js"), "test('contract', () => {});\n");
    let ls = run_json(docs.path(), docs_cache.path(), &["ls", ".", "--format", "json"]);
    for (kind, example) in [
        ("contract_container", "contracts/"),
        ("data_container", "data/"),
        ("deployment_container", "infra/"),
        ("verification_container", "tests/"),
    ] {
        assert_surface_examples(&ls, kind, &[example]);
    }
}

fn assert_surface_examples(report: &Value, kind: &str, expected: &[&str]) {
    let surface = report["directory"]
        .as_array()
        .expect("directory surfaces")
        .iter()
        .find(|surface| surface["kind"] == kind)
        .unwrap_or_else(|| panic!("missing `{kind}`: {report:#}"));
    let examples = surface["examples"].as_array().expect("surface examples");
    for expected in expected {
        assert!(
            examples.iter().any(|example| example == expected),
            "missing `{expected}` in `{kind}`: {report:#}"
        );
    }
}
