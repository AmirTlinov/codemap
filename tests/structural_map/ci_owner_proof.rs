#[test]
fn ci_owner_proof_uses_hard_validation_steps_without_shell_builtin_noise() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"ci-owner-proof","private":true,"workspaces":["apps/*"],"scripts":{"test":"vitest run","deploy:test":"echo deploy","migrate:test":"echo migrate","codegen:test":"echo codegen","generate:test":"echo generate","install:test":"echo install"}}"#,
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run","db:generate":"prisma generate","generate:test":"echo generate"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo run --bin build\n      - run: test -n \"${DATABASE_URL}\" || exit 1\n      - run: |\n          pnpm --filter @fixture/api db:generate\n          pnpm --filter @fixture/api generate:test\n          pnpm --filter @fixture/api test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci owner proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".github/workflows/ci.yml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(|surface| {
            surface["evidence"] == "ci_run_step"
                && surface["strength"] == "hard"
                && surface["command"] == "pnpm --filter @fixture/api test"
                && surface["locations"]
                    .as_array()
                    .expect("locations")
                    .iter()
                    .any(|location| location["path"] == ".github/workflows/ci.yml"
                        && location["line_start"] == 12)
        }),
        "CI proof should reuse deterministic workflow run-step evidence: {proof:#}"
    );
    assert!(
        !proofs.iter().any(|surface| {
            surface["evidence"] == "ci_run_step"
                && surface["command"] == "pnpm --filter @fixture/api db:generate"
        }),
        "CI owner proof must not expose non-validation codegen as a runnable proof command: {proof:#}"
    );
    assert!(
        !proofs.iter().any(|surface| {
            surface["evidence"] == "ci_run_step"
                && surface["command"]
                    .as_str()
                    .is_some_and(|command| command.starts_with("test -n"))
        }),
        "CI owner proof must not treat shell builtin `test` as package test proof: {proof:#}"
    );
    assert!(
        !proofs.iter().any(|surface| surface["command"]
            .as_str()
            .is_some_and(|command| {
                command.contains("cargo run --bin build")
                    || command.contains("deploy:test")
                    || command.contains("migrate:test")
                    || command.contains("codegen:test")
                    || command.contains("generate:test")
                    || command.contains("install:test")
            })),
        "CI owner proof must not leak broad cargo or mutating role-aware commands: {proof:#}"
    );
    assert!(
        !proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_deterministic_proof"),
        "hard CI run-step proof should not be masked by soft-proof Unknown: {proof:#}"
    );
}
