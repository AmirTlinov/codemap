#[test]
fn deleted_env_file_reports_removed_keys_from_base_not_current_fallback() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join(".env.example"), "DATABASE_URL=\nAPI_TOKEN=\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env baseline"]);
    std::fs::remove_file(repo.path().join(".env.example")).expect("remove env file");

    let changed = run_json(
        repo.path(),
        cache.path(),
        &["changed", "--files", ".env.example", "--format", "json"],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    let events = changed["structural_events"]
        .as_array()
        .expect("structural events");
    for key in ["DATABASE_URL", "API_TOKEN"] {
        assert!(
            events.iter().any(|event| event["kind"] == "removed_env_key"
                && event["effect"]
                    .as_str()
                    .is_some_and(|effect| effect.contains(key))),
            "deleted env file should report removed key `{key}` from base: {changed:#}"
        );
    }
    assert!(
        !events.iter().any(|event| event["kind"] == "added_env_key"),
        "deleted env file must not read HEAD as current content and invent added keys: {changed:#}"
    );
}

#[test]
fn manifest_ci_reference_requires_package_specific_evidence() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"ci-specificity-fixture","private":true,"workspaces":["apps/*"]}"#,
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("apps/other/package.json"),
        r#"{"name":"@fixture/other","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm --filter @fixture/other test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci specificity baseline"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "apps/api/package.json", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        !proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|proof| proof["evidence"] == "manifest_ci_reference"),
        "unrelated workspace CI run must not become manifest_ci_reference proof: {proof:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "changed",
            "--files",
            "apps/api/package.json",
            "--section",
            "unknown",
        ])
        .output()
        .expect("changed unknown should run");
    assert!(
        output.status.success(),
        "changed unknown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("ci_reference_not_found"),
        "unrelated workspace CI should keep package CI reference Unknown open: {markdown}"
    );
}

#[test]
fn env_ci_reference_ignores_comment_only_mentions() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join(".env.example"), "DATABASE_URL=\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    steps:\n      # DATABASE_URL is documented here but not used\n      - run: pnpm test # DATABASE_URL documented only\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env ci comment"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".env.example", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        !proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|proof| proof["evidence"] == "env_ci_reference"),
        "comment-only env mention must not become env_ci_reference proof: {proof:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", ".env.example", "--section", "unknown"])
        .output()
        .expect("changed unknown should run");
    assert!(
        output.status.success(),
        "changed unknown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("env_ci_reference_not_found"),
        "comment-only env mention should keep CI reference Unknown open: {markdown}"
    );
}
