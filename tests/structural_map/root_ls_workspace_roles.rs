#[test]
fn root_ls_preserves_rust_workspace_package_edges_under_shared_src_parent() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = ["src/app", "src/core", "src/config"]
resolver = "2"
"#,
    );
    write(
        &repo.path().join("src/app/Cargo.toml"),
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
masque-core = { path = "../core" }
silentway-config = { path = "../config" }
"#,
    );
    write(
        &repo.path().join("src/core/Cargo.toml"),
        r#"[package]
name = "masque-core"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(
        &repo.path().join("src/config/Cargo.toml"),
        r#"[package]
name = "silentway-config"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(&repo.path().join("src/app/src/lib.rs"), "pub fn app() {}\n");
    write(
        &repo.path().join("src/core/src/lib.rs"),
        "pub fn core() {}\n",
    );
    write(
        &repo.path().join("src/config/src/lib.rs"),
        "pub fn config() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    let edges = json["edges"].as_array().expect("edges");
    assert!(
        edges.iter().any(|edge| edge["type"] == "package_internal"
            && edge["from"] == "src/app/"
            && edge["to"] == "src/core/"
            && edge["evidence"]
                .as_str()
                .unwrap_or_default()
                .contains("masque-core")),
        "root map must keep package endpoints under shared src parent instead of collapsing them away: {json:#}"
    );
    assert!(
        edges.iter().any(|edge| edge["type"] == "package_internal"
            && edge["from"] == "src/app/"
            && edge["to"] == "src/config/"
            && edge["evidence"]
                .as_str()
                .unwrap_or_default()
                .contains("silentway-config")),
        "root map should preserve each structural package dependency under src/: {json:#}"
    );
    assert!(
        !edges
            .iter()
            .any(|edge| edge["from"] == "src/" && edge["to"] == "src/"),
        "self-collapsed src edges are not useful map output: {json:#}"
    );
    assert_eq!(json.get("read_first"), None);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", ".", "--depth", "2", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let outgoing = cone["outgoing"].as_array().expect("outgoing");
    assert!(
        outgoing
            .iter()
            .any(|edge| edge["type"] == "package_internal"
                && edge["from"] == "src/app/"
                && edge["to"] == "src/core/"),
        "directory cone should keep shared-src package endpoints instead of expanding to files: {cone:#}"
    );
    assert!(
        !outgoing
            .iter()
            .any(|edge| edge["from"] == "src/" && edge["to"] == "src/"),
        "directory cone should not expose self-collapsed shared-parent edges: {cone:#}"
    );
}


#[test]
fn zero_config_roles_do_not_label_project_maps_or_routes_as_codemap_engine() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "role-noise-fixture",
  "private": true
}
"#,
    );
    write(&repo.path().join(".agents/system_map.md"), "# System map\n");
    write(&repo.path().join("artifacts/proof-map.json"), "{}\n");
    write(
        &repo.path().join("app/api/auth/route.ts"),
        "export async function POST() {\n  return Response.json({ ok: true });\n}\n",
    );
    write(
        &repo.path().join("harness/cone-probe.ts"),
        "export const probe = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let root = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &root);
    assert!(
        root["directory"]
            .as_array()
            .expect("directory surfaces")
            .iter()
            .all(|surface| surface["kind"] != "map_engine"),
        "project-local maps/routes/proof artifacts should not be mislabeled as the codemap engine role: {root:#}"
    );

    for path in [
        ".agents/system_map.md",
        "artifacts/proof-map.json",
        "app/api/auth/route.ts",
        "harness/cone-probe.ts",
    ] {
        let file = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &file);
        assert_ne!(
            file["anchor"]["kind"], "map_engine",
            "{path} should keep its real file kind instead of codemap-specific noise: {file:#}"
        );
        assert!(
            file["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .all(|role| role != "map_engine"),
            "{path} should not carry codemap-specific map_engine role: {file:#}"
        );
    }
}

