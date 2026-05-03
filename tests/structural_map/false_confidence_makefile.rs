#[test]
fn makefile_does_not_emit_ci_workflow_run_step_unknown() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Makefile"),
        "test:\n\tcargo test\n\nbuild:\n\tcargo build\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "makefile owner"]);

    let changed_unknown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", "Makefile", "--section", "unknown"])
        .output()
        .expect("changed unknown should run");
    assert!(
        changed_unknown.status.success(),
        "changed unknown failed: {}",
        String::from_utf8_lossy(&changed_unknown.stderr)
    );
    let changed_unknown = String::from_utf8(changed_unknown.stdout).expect("markdown utf8");
    assert!(
        !changed_unknown.contains("ci_run_step_not_found")
            && !changed_unknown.contains("ci_validation_step_not_found"),
        "Makefile is a script/build surface, not a CI workflow run-step surface: {changed_unknown}"
    );

    let proof_unknown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "Makefile", "--section", "unknown"])
        .output()
        .expect("proof unknown should run");
    assert!(
        proof_unknown.status.success(),
        "proof unknown failed: {}",
        String::from_utf8_lossy(&proof_unknown.stderr)
    );
    let proof_unknown = String::from_utf8(proof_unknown.stdout).expect("markdown utf8");
    assert!(
        !proof_unknown.contains("ci_validation_step_not_found"),
        "proof Makefile should not ask for GitHub-style CI run steps: {proof_unknown}"
    );
}
