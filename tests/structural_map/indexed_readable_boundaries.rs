// Responsibility: indexed-readable-boundary-regressions
fn assert_map_output_omits(json: &Value, needles: &[&str], surface: &str) {
    let text = serde_json::to_string(json).expect("map output json");
    for needle in needles {
        assert!(
            !text.contains(needle),
            "{surface} followed unavailable repository content `{needle}`: {json:#}"
        );
    }
}

#[cfg(unix)]
#[test]
fn structural_symlink_bodies_are_path_facts_only_across_map_surfaces() {
    use std::os::unix::fs::symlink;

    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let external = TempDir::new().expect("external structural bodies");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &external.path().join("ci.yml"),
        "jobs:\n  leak:\n    steps:\n      - run: cargo test --package EXTERNAL_CI_BODY_LEAK\n",
    );
    write(
        &external.path().join("package.json"),
        r#"{"name":"EXTERNAL_PACKAGE_BODY_LEAK","scripts":{"test":"echo PACKAGE_SCRIPT_BODY_LEAK"}}"#,
    );
    fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflow directory");
    symlink(
        external.path().join("ci.yml"),
        repo.path().join(".github/workflows/ci.yml"),
    )
    .expect("workflow symlink");
    symlink(
        external.path().join("package.json"),
        repo.path().join("package.json"),
    )
    .expect("package symlink");
    write(&repo.path().join("README.md"), "# indexed path owner\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "external structural boundaries"]);

    let runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
    assert!(
        runtime["ci"].as_array().expect("ci facts").iter().any(|fact| {
            fact["path"] == ".github/workflows/ci.yml" && fact["evidence"] == "role:build_ci"
        }),
        "the indexed workflow path remains visible: {runtime:#}"
    );

    let needles = [
        "EXTERNAL_CI_BODY_LEAK",
        "EXTERNAL_PACKAGE_BODY_LEAK",
        "PACKAGE_SCRIPT_BODY_LEAK",
    ];
    assert_map_output_omits(&runtime, &needles, "runtime map");
    for (surface, args) in [
        ("ls root", vec!["ls", ".", "--format", "json"]),
        ("root cone", vec!["cone", ".", "--format", "json"]),
        (
            "proof map",
            vec!["proof-map", ".", "--raw-sensors", "--format", "json"],
        ),
        ("status", vec!["status", "--format", "json"]),
    ] {
        let json = run_json(repo.path(), cache.path(), &args);
        assert_map_output_omits(&json, &needles, surface);
    }
}

#[test]
fn oversized_structural_bodies_cannot_publish_semantic_facts() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    let padding = "x".repeat(901_000);
    write(
        &repo.path().join("package.json"),
        &format!(
            r#"{{"name":"OVERSIZED_PACKAGE_BODY_LEAK","scripts":{{"test":"echo OVERSIZED_SCRIPT_BODY_LEAK"}},"padding":"{padding}"}}"#
        ),
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        &format!(
            "jobs:\n  leak:\n    steps:\n      - run: cargo test --package OVERSIZED_CI_BODY_LEAK\n# {padding}\n"
        ),
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "oversized structural boundaries"]);

    let runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
    assert!(
        runtime["ci"].as_array().expect("ci facts").iter().any(|fact| {
            fact["path"] == ".github/workflows/ci.yml" && fact["evidence"] == "role:build_ci"
        }),
        "the oversized workflow keeps its indexed path fact: {runtime:#}"
    );
    let needles = [
        "OVERSIZED_CI_BODY_LEAK",
        "OVERSIZED_PACKAGE_BODY_LEAK",
        "OVERSIZED_SCRIPT_BODY_LEAK",
    ];
    assert_map_output_omits(&runtime, &needles, "runtime map");
    for (surface, args) in [
        (
            "package listing",
            vec!["ls", "package.json", "--format", "json"],
        ),
        (
            "package cone",
            vec!["cone", "package.json", "--format", "json"],
        ),
        (
            "proof map",
            vec!["proof-map", ".", "--raw-sensors", "--format", "json"],
        ),
        ("status", vec!["status", "--format", "json"]),
    ] {
        let json = run_json(repo.path(), cache.path(), &args);
        assert_map_output_omits(&json, &needles, surface);
    }
}

#[cfg(unix)]
#[test]
fn cold_root_inventory_never_follows_structural_symlinks() {
    use std::os::unix::fs::symlink;

    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let external = TempDir::new().expect("external inventory bodies");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &external.path().join("ci.yml"),
        "jobs:\n  leak:\n    steps:\n      - run: echo COLD_CI_BODY_LEAK\n",
    );
    write(
        &external.path().join("package.json"),
        r#"{"name":"COLD_PACKAGE_BODY_LEAK","scripts":{"test":"echo COLD_SCRIPT_BODY_LEAK"}}"#,
    );
    fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflow directory");
    symlink(
        external.path().join("ci.yml"),
        repo.path().join(".github/workflows/ci.yml"),
    )
    .expect("workflow symlink");
    symlink(
        external.path().join("package.json"),
        repo.path().join("package.json"),
    )
    .expect("package symlink");
    for index in 0..805 {
        write(
            &repo.path().join(format!("src/load/file-{index:03}.ts")),
            &format!("export const value{index} = {index};\n"),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "cold external structural boundaries"]);

    let needles = [
        "COLD_CI_BODY_LEAK",
        "COLD_PACKAGE_BODY_LEAK",
        "COLD_SCRIPT_BODY_LEAK",
    ];
    let listing = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_map_output_omits(&listing, &needles, "cold root listing");
    let proof = run_json(repo.path(), cache.path(), &["proof-map", ".", "--format", "json"]);
    assert_map_output_omits(&proof, &needles, "cold root proof map");
}

#[cfg(unix)]
#[test]
fn diff_map_treats_a_symlink_as_its_git_blob_not_external_body() {
    use std::os::unix::fs::symlink;

    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let external = TempDir::new().expect("external diff body");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/current.ts"),
        "export const current = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "regular source base"]);
    write(
        &external.path().join("current.ts"),
        "export const EXTERNAL_DIFF_BODY_LEAK = true;\n",
    );
    fs::remove_file(repo.path().join("src/current.ts")).expect("remove regular source");
    symlink(
        external.path().join("current.ts"),
        repo.path().join("src/current.ts"),
    )
    .expect("source symlink");

    let diff = run_json(
        repo.path(),
        cache.path(),
        &["diff-map", "--changed", "--format", "json"],
    );
    assert_map_output_omits(&diff, &["EXTERNAL_DIFF_BODY_LEAK"], "working-tree diff map");
}

#[test]
fn cold_workspace_edges_require_an_indexed_pattern_owner() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".gitignore"),
        "/package.json\n/pnpm-workspace.yaml\n",
    );
    write(
        &repo.path().join("package.json"),
        r#"{"name":"ignored-root","workspaces":["packages/*"]}"#,
    );
    write(
        &repo.path().join("pnpm-workspace.yaml"),
        "packages:\n  - packages/*\n",
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{"name":"tracked-app"}"#,
    );
    for index in 0..805 {
        write(
            &repo.path().join(format!("src/load/file-{index:03}.ts")),
            &format!("export const value{index} = {index};\n"),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ignored workspace owners"]);

    let listing = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert!(
        listing["edges"]
            .as_array()
            .expect("root edges")
            .iter()
            .all(|edge| edge["type"] != "workspace_member"),
        "ignored filesystem manifests cannot declare cold inventory workspace edges: {listing:#}"
    );
}
