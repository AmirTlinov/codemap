#[test]
fn proof_map_large_cold_root_uses_bounded_inventory_with_exact_expand() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "large-proof-map-root",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "build": "tsc -b",
    "build:dev": "vite build --watch"
  }
}
"#,
    );
    write(
        &repo.path().join("tests/root-smoke.test.ts"),
        "test('root smoke', () => expect(true).toBe(true));\n",
    );
    write(
        &repo.path().join("tests/root_rust.rs"),
        "#[test]\nfn root_rust_smoke() { assert!(true); }\n",
    );
    write(&repo.path().join("tests/README.md"), "# notes\n");
    for index in 0..805 {
        write(
            &repo
                .path()
                .join(format!("src/generated-load/module-{index:03}.ts")),
            &format!("export const module{index:03} = {index};\n"),
        );
    }

    let proof_map = run_json(repo.path(), cache.path(), &["proof-map", ".", "--format", "json"]);
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["direct_evidence"]
            .as_array()
            .expect("direct evidence")
            .iter()
            .any(|proof| proof["path"] == "tests/"
                && proof["evidence"] == "current_level_proof_container"
                && proof["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("2 candidate test files"))),
        "large root inventory proof-map should keep current-level proof containers: {proof_map:#}"
    );
    assert!(
        proof_map["hard"]
            .as_array()
            .expect("hard")
            .iter()
            .any(|proof| proof["command"] == "npm test"
                && proof["evidence"] == "manifest_script"),
        "large root inventory proof-map should still expose declared runnable scripts: {proof_map:#}"
    );
    for section in ["hard", "setup_support", "commands"] {
        for proof in proof_map[section].as_array().expect("proof section") {
            let command = proof["command"].as_str().unwrap_or_default();
            assert!(
                !command.starts_with("command:"),
                "bounded root proof-map must not leak internal graph node ids as commands: {proof_map:#}"
            );
            assert!(
                !matches!(command, "vitest run" | "tsc -b"),
                "bounded root proof-map must expose declared script runners, not package script bodies: {proof_map:#}"
            );
        }
    }
    assert!(
        proof_map["setup_support"]
            .as_array()
            .expect("setup support")
            .iter()
            .any(|proof| proof["command"] == "npm run 'build:dev'"
                && proof["evidence"] == "manifest_script_setup"),
        "large root inventory proof-map should label watch/dev-like scripts as setup/support, not proof: {proof_map:#}"
    );
    assert!(
        proof_map["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "bounded_root_inventory"
                && unknown["expand"] == "codemap proof-map . --raw-sensors"),
        "bounded root inventory must keep file-level proof gaps explicit with exact expand: {proof_map:#}"
    );
    assert!(
        proof_map["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"] == "recursive proof seeds hidden at root scope"
                && hidden["expand"].as_str().is_some_and(|expand| expand
                    .starts_with("codemap proof-map . --raw-sensors --limit "))),
        "large root proof-map should expose recursive raw-sensors expansion: {proof_map:#}"
    );
}

#[test]
fn proof_map_large_cold_root_falls_back_to_fail_closed_ctx_validation_for_ignored_root_config() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join(".gitignore"), ".ctx.yml\n");
    write(
        &repo.path().join(".ctx.yml"),
        "version: 1\nconcepts:\n  empty:\n    files: []\n",
    );
    write(
        &repo.path().join("package.json"),
        r#"{"name":"large-invalid-ctx","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    for index in 0..805 {
        write(
            &repo.path().join(format!("src/load/file-{index:03}.ts")),
            &format!("export const file{index:03} = {index};\n"),
        );
    }

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof-map", ".", "--format", "json"])
        .output()
        .expect("codemap should run");
    assert!(
        !output.status.success(),
        "large root proof-map must not bypass invalid .ctx validation: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid .ctx semantic anchors")
            && stderr.contains("concept `empty` must declare at least one file"),
        "normal fail-closed anchor validation should own the error: {stderr}"
    );
}
