#[test]
fn flat_huge_directory_ls_stays_bounded_without_expanding_the_galaxy() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "flat-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    for index in 0..80 {
        write(
            &repo.path().join(format!("src/flat/module-{index:02}.ts")),
            &format!("export const module{index:02} = {index};\n"),
        );
    }
    write(
        &repo.path().join("src/flat/deep/nested-owner.ts"),
        "export function nestedOwner() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/flat", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &json);
    let surfaces = json["directory"].as_array().expect("directory surfaces");
    let source_surface = surfaces
        .iter()
        .find(|surface| surface["kind"] == "source")
        .expect("source surface");
    assert_eq!(source_surface["count"], 80);
    assert!(
        source_surface["examples"]
            .as_array()
            .expect("examples")
            .len()
            <= 5,
        "flat directory examples must stay bounded"
    );
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"] == "recursive files below this level hidden"),
        "recursive detail must stay hidden unless explicitly expanded"
    );
    assert_eq!(json.get("read_first"), None);
}


#[test]
fn inventory_prunes_untracked_build_dirs_before_config_discovery() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"untracked-build-prune-fixture","private":true}"#,
    );
    write(
        &repo.path().join("src/tracked.ts"),
        "export const tracked = 1;\n",
    );
    write(
        &repo.path().join("target/generated/.codemap.yml"),
        "version: 999\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    write(
        &repo.path().join("src/untracked.ts"),
        "export const untracked = 2;\n",
    );
    write(
        &repo.path().join("build/generated/.codemap.yml"),
        "version: 999\n",
    );
    write(
        &repo.path().join("target/generated/noise.ts"),
        "export const targetNoise = true;\n",
    );
    write(
        &repo.path().join("build/generated/noise.ts"),
        "export const buildNoise = true;\n",
    );

    let files = run_json(repo.path(), cache.path(), &["files", "--format", "json"]);
    assert_schema("schemas/files.schema.json", &files);
    let indexed = files["files"].as_array().expect("files");
    assert!(
        indexed.iter().any(|path| path == "src/untracked.ts"),
        "normal untracked source files should still be visible to the map: {files:#}"
    );
    assert!(
        indexed.iter().all(|path| {
            let path = path.as_str().unwrap_or_default();
            !path.starts_with("target/") && !path.starts_with("build/")
        }),
        "common build dirs must be pruned before they enter inventory: {files:#}"
    );

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(
        validation["ok"], true,
        "ignored build-dir .codemap.yml files must not become loaded config errors: {validation:#}"
    );
    assert_eq!(validation["config"], Value::Null);
}


#[test]
fn root_ls_does_not_bubble_nested_fixture_roles_to_workspace_containers() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = ["crates/core"]
resolver = "2"
"#,
    );
    write(
        &repo.path().join("crates/core/Cargo.toml"),
        r#"[package]
name = "core"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(
        &repo.path().join("crates/core/src/lib.rs"),
        "pub fn core() {}\n",
    );
    write(
        &repo.path().join("crates/core/tests/fixtures/sample.json"),
        "{}\n",
    );
    write(
        &repo.path().join("py/service/app.py"),
        "def app():\n    return True\n",
    );
    write(&repo.path().join("py/service/fixtures/sample.json"), "{}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "nested fixture containers"]);

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    let surfaces = json["directory"].as_array().expect("directory surfaces");
    assert!(
        surfaces.iter().all(|surface| {
            !matches!(
                surface["kind"].as_str().unwrap_or_default(),
                "fixture" | "test_support" | "e2e_test"
            ) || !surface["examples"]
                .as_array()
                .expect("examples")
                .iter()
                .any(|example| example == "crates/" || example == "py/")
        }),
        "top-level workspace containers must not inherit nested fixture roles: {json:#}"
    );
    assert!(
        surfaces.iter().any(|surface| surface["kind"] == "dir"
            && surface["examples"]
                .as_array()
                .expect("examples")
                .iter()
                .any(|example| example == "crates/")
            && surface["examples"]
                .as_array()
                .expect("examples")
                .iter()
                .any(|example| example == "py/")),
        "workspace containers should remain ordinary current-level dirs: {json:#}"
    );
}

