#[test]
fn ci_owner_package_script_wrapper_checks_manifest_body_before_run() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"ci-wrapper-body","scripts":{"test":"vitest --watch","verify":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    steps:\n      - run: npm test\n      - run: npm run verify\n",
    );
    write(
        &repo.path().join("bin/npm"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> npm-args.txt\n",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(repo.path().join("bin/npm"), fs::Permissions::from_mode(0o755))
            .expect("chmod npm");
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci wrapper body"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".github/workflows/ci.yml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(|surface| {
            surface["command"] == "npm test" && surface["evidence"] == "ci_run_setup"
        }) && proofs.iter().any(|surface| {
            surface["command"] == "npm run verify" && surface["evidence"] == "ci_run_step"
        }) && !proofs.iter().any(|surface| {
            surface["command"] == "npm test" && surface["evidence"] == "ci_run_step"
        }),
        "CI wrapper proof must use package script body before marking proof runnable: {proof:#}"
    );

    let path = format!(
        "{}:{}",
        repo.path().join("bin").to_string_lossy(),
        env::var("PATH").unwrap_or_default()
    );
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("PATH", path)
        .args(["proof", ".github/workflows/ci.yml", "--run"])
        .output()
        .expect("proof --run should execute safe wrapper only");
    assert!(
        output.status.success(),
        "proof --run should accept only safe CI wrapper proof: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let args = fs::read_to_string(repo.path().join("npm-args.txt")).expect("npm args");
    assert_eq!(args.trim(), "run verify");
}

#[test]
fn ci_owner_unsafe_only_wrapper_does_not_fallback_to_same_script() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"ci-wrapper-body-only","scripts":{"test":"vitest --watch"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    steps:\n      - run: npm test\n",
    );
    write(
        &repo.path().join("bin/npm"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> npm-args.txt\n",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(repo.path().join("bin/npm"), fs::Permissions::from_mode(0o755))
            .expect("chmod npm");
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci unsafe wrapper only"]);

    let path = format!(
        "{}:{}",
        repo.path().join("bin").to_string_lossy(),
        env::var("PATH").unwrap_or_default()
    );
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("PATH", path)
        .args(["proof", ".github/workflows/ci.yml", "--run"])
        .output()
        .expect("proof --run should refuse unsafe-only wrapper plan");
    assert!(
        !output.status.success(),
        "unsafe-only wrapper should not become runnable fallback proof"
    );
    assert!(
        !repo.path().join("npm-args.txt").exists(),
        "unsafe wrapper must not execute through proof or fallback"
    );
}
