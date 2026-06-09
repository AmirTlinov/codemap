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
fn manifest_ci_reference_uses_package_reference_boundaries() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"boundary-fixture","private":true,"workspaces":["apps/*"]}"#,
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"api","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm --filter capitan test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci package boundary"]);

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
            .any(|proof| proof["evidence"] == "manifest_ci_reference"
                && proof["command"]
                    .as_str()
                    .is_some_and(|command| command.contains("capitan"))),
        "`api` must not match substring inside unrelated package selector `capitan`: {proof:#}"
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
        "substring package selector should keep CI reference Unknown open: {markdown}"
    );
}

#[test]
fn manifest_script_substrings_do_not_become_hard_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"manifest-substring-fixture","private":true,"scripts":{"contest":"echo not validation"}}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "manifest substring"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "package.json", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        !proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|proof| proof["evidence"] == "manifest_script"
                && proof["command"]
                    .as_str()
                    .is_some_and(|command| command.contains("contest"))),
        "`contest` must not become manifest_script proof through substring matching: {proof:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", "package.json", "--section", "unknown"])
        .output()
        .expect("changed unknown should run");
    assert!(
        output.status.success(),
        "changed unknown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("package_local_script_not_found")
            && markdown.contains("codemap graph --path package.json --lens causal")
            && !markdown.contains("codemap graph --lens causal package.json"),
        "substring-only script names should keep package-local script Unknown open with executable expands: {markdown}"
    );

    let graph = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["graph", "--path", "package.json", "--lens", "causal"])
        .output()
        .expect("graph expand should run");
    assert!(
        graph.status.success(),
        "manifest package_consumer expand should be executable: {}",
        String::from_utf8_lossy(&graph.stderr)
    );
}

#[test]
fn cargo_version_ci_step_does_not_become_manifest_ci_reference() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"cargo-version-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  probe:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo --version\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "cargo version ci"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Cargo.toml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        !proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|proof| proof["evidence"] == "manifest_ci_reference"
                && proof["command"] == "cargo --version"),
        "`cargo --version` must not become manifest_ci_reference proof: {proof:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", "Cargo.toml", "--section", "unknown"])
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
        "`cargo --version` should keep manifest CI reference Unknown open: {markdown}"
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
