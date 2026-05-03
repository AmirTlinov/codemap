#[test]
fn package_test_wrapper_with_watch_body_is_not_runnable_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"watch-body-fixture","private":true,"scripts":{"test":"vitest --watch","verify":"vitest run"}}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "watch body proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "package.json", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(|surface| surface["command"] == "npm run verify")
            && proofs.iter().any(|surface| {
                surface["command"] == "npm test" && surface["evidence"] == "manifest_script_setup"
            })
            && !proofs.iter().any(|surface| {
                surface["command"] == "npm test" && surface["evidence"] == "manifest_script"
            }),
        "test wrapper with watch body must be setup/support, not runnable proof: {proof:#}"
    );
}

#[test]
fn ci_cargo_fmt_requires_check_to_be_runnable_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("Cargo.toml"), "[package]\nname = \"fmt-proof\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo fmt\n      - run: cargo fmt --check\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fmt proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".github/workflows/ci.yml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let commands = proof["proofs"]
        .as_array()
        .expect("proofs")
        .iter()
        .filter_map(|surface| surface["command"].as_str())
        .collect::<Vec<_>>();
    assert!(
        commands.contains(&"cargo fmt --check") && !commands.contains(&"cargo fmt"),
        "cargo fmt without --check must not be runnable proof: {proof:#}"
    );
}
