#[test]
fn boundary_map_root_hides_support_fixture_boundaries_until_explicit_scope() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"boundary-support-root","private":true}"#,
    );
    write(
        &repo.path().join("src/index.ts"),
        "export const rootValue = true;\n",
    );
    write(
        &repo.path().join("fixtures/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "dependencies": { "@fixture/lib": "workspace:*" }
}
"#,
    );
    write(
        &repo.path().join("fixtures/lib/package.json"),
        r#"{"name":"@fixture/lib"}"#,
    );
    write(
        &repo.path().join("fixtures/lib/src/session.ts"),
        "export const session = true;\n",
    );
    write(
        &repo.path().join("fixtures/app/src/app.ts"),
        "import { session } from '../../lib/src/session';\nexport const app = session;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "support fixture boundary map"]);

    let root = run_json(
        repo.path(),
        cache.path(),
        &["boundary-map", ".", "--format", "json"],
    );
    assert_schema("schemas/boundary-map.schema.json", &root);
    assert!(
        !boundary_map_paths(&root)
            .iter()
            .any(|path| path.starts_with("fixtures/")),
        "root boundary-map must stay current-level and hide support fixture internals: {root:#}"
    );
    assert!(
        root["hidden"].as_array().expect("hidden").iter().any(|hidden| {
            hidden["reason"] == "support boundary artifacts hidden"
                && hidden["expand"] == "codemap boundary-map . --include-hidden"
        }),
        "root boundary-map should make hidden support boundaries explicit: {root:#}"
    );

    write(
        &repo.path().join("src/index.ts"),
        "export const rootValue = false;\n",
    );
    let changed_root = run_json(
        repo.path(),
        cache.path(),
        &["boundary-map", ".", "--changed", "--format", "json"],
    );
    assert_schema("schemas/boundary-map.schema.json", &changed_root);
    assert!(
        !boundary_map_paths(&changed_root)
            .iter()
            .any(|path| path.starts_with("fixtures/")),
        "non-support changed files must not reopen fixture boundary internals: {changed_root:#}"
    );

    let scoped = run_json(
        repo.path(),
        cache.path(),
        &["boundary-map", "fixtures", "--format", "json"],
    );
    assert_schema("schemas/boundary-map.schema.json", &scoped);
    assert!(
        boundary_map_paths(&scoped)
            .iter()
            .any(|path| path.starts_with("fixtures/")),
        "explicit support scope should reveal fixture boundary map facts: {scoped:#}"
    );
}

#[test]
fn boundary_map_changed_reveals_touched_support_boundary_facts() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"boundary-support-changed-root","private":true}"#,
    );
    write(
        &repo.path().join("fixtures/app/package.json"),
        r#"{"name":"@fixture/app"}"#,
    );
    write(
        &repo.path().join("fixtures/lib/package.json"),
        r#"{"name":"@fixture/lib"}"#,
    );
    write(
        &repo.path().join("fixtures/lib/src/session.ts"),
        "export const session = true;\n",
    );
    write(
        &repo.path().join("fixtures/app/src/app.ts"),
        "import { session } from '../../lib/src/session';\nexport const app = session;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "support fixture boundary map"]);
    write(
        &repo.path().join("fixtures/app/src/app.ts"),
        "import { session } from '../../lib/src/session';\nexport const app = !session;\n",
    );

    let changed = run_json(
        repo.path(),
        cache.path(),
        &["boundary-map", ".", "--changed", "--format", "json"],
    );
    assert_schema("schemas/boundary-map.schema.json", &changed);
    assert!(
        boundary_map_paths(&changed)
            .iter()
            .any(|path| path == "fixtures/app/src/app.ts"),
        "support changed files should reveal their touched boundary facts: {changed:#}"
    );
}

#[test]
fn boundary_map_changed_reveals_support_workspace_manifest_package_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"boundary-support-workspace-root\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("fixtures/Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"lib\"]\n\n[workspace.dependencies]\nfixture-lib = { path = \"lib\" }\n",
    );
    write(
        &repo.path().join("fixtures/app/Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nfixture-lib = { workspace = true }\n",
    );
    write(
        &repo.path().join("fixtures/lib/Cargo.toml"),
        "[package]\nname = \"fixture-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("fixtures/app/src/main.rs"),
        "fn main() {}\n",
    );
    write(
        &repo.path().join("fixtures/lib/src/lib.rs"),
        "pub fn value() -> bool { true }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "support workspace boundary map"]);
    write(
        &repo.path().join("fixtures/Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"lib\"]\n\n[workspace.dependencies]\nfixture-lib = { path = \"lib\" }\n# touched\n",
    );

    let changed = run_json(
        repo.path(),
        cache.path(),
        &["boundary-map", ".", "--changed", "--format", "json"],
    );
    assert_schema("schemas/boundary-map.schema.json", &changed);
    assert!(
        changed["package_edges"]
            .as_array()
            .expect("package edges")
            .iter()
            .any(|edge| edge["workspace_manifest"] == "fixtures/Cargo.toml"
                && edge["from_manifest"] == "fixtures/app/Cargo.toml"),
        "changed support workspace manifest should reveal package edges that depend on it: {changed:#}"
    );
}

fn boundary_map_paths(report: &serde_json::Value) -> Vec<String> {
    let mut paths = Vec::new();
    for section in ["actual_cross_edges", "test_only_crossings"] {
        for edge in report[section].as_array().expect("edge section") {
            for key in ["from", "to"] {
                if let Some(path) = edge[key].as_str() {
                    paths.push(path.to_string());
                }
            }
        }
    }
    for file in report["public_boundary_files"]
        .as_array()
        .expect("public boundary files")
    {
        if let Some(path) = file["path"].as_str() {
            paths.push(path.to_string());
        }
    }
    for edge in report["package_edges"].as_array().expect("package edges") {
        for key in ["from_manifest", "to_manifest"] {
            if let Some(path) = edge[key].as_str() {
                paths.push(path.to_string());
            }
        }
    }
    paths
}
