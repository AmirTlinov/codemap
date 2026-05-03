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
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo run --bin build\n      - run: test -n \"${DATABASE_URL}\" || exit 1\n      - run: |\n          pnpm --filter @fixture/api db:generate\n          pnpm --filter @fixture/api generate:test\n          pnpm --filter @fixture/api test\n          ./scripts/e2e_smoke.sh\n          total=$(grep \"^test result\" /tmp/test-output.txt | awk '{sum+=$4} END {printf \"%d\", sum}')\n",
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
        proofs.iter().any(|surface| {
            surface["evidence"] == "ci_run_step"
                && surface["strength"] == "hard"
                && surface["command"] == "./scripts/e2e_smoke.sh"
        }),
        "CI owner proof should expose direct e2e/smoke scripts as hard workflow proof: {proof:#}"
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
                    || command.contains("test-output.txt")
            })),
        "CI owner proof must not leak broad cargo, mutating commands, or support artifacts: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["evidence"] == "ci_run_step"
                && surface["strength"] == "hard"),
        "CI owner proof should expose hard workflow validation run steps, not generic soft role-aware scripts: {proof:#}"
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

#[test]
fn ci_owner_proof_fails_open_when_workflow_has_no_validation_run_step() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"release-only-ci","private":true,"scripts":{"test":"vitest run","release:prod":"node scripts/release.js"}}"#,
    );
    write(&repo.path().join("scripts/release.js"), "console.log('release')\n");
    write(
        &repo.path().join(".github/workflows/deploy.yml"),
        "name: deploy\non: [workflow_dispatch]\njobs:\n  deploy:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm release:prod\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "release only ci"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".github/workflows/deploy.yml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "release/setup workflow run lines must not become proof commands: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "missing CI validation proof should keep conservative fallback commands visible: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "ci_validation_step_not_found"),
        "missing CI validation proof should be explicit Unknown, not soft-proof silence: {proof:#}"
    );

    write(
        &repo.path().join(".github/workflows/deploy.yml"),
        "name: deploy\non: [workflow_dispatch]\njobs:\n  deploy:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm release:prod\n      - run: echo done\n",
    );
    let changed = run_json(
        repo.path(),
        cache.path(),
        &["proof", "changed", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &changed);
    assert!(
        changed["proofs"].as_array().expect("proofs").is_empty(),
        "proof changed must not turn release/setup workflow lines into proof commands: {changed:#}"
    );
    assert!(
        changed["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "ci_validation_step_not_found"),
        "proof changed should keep missing CI validation proof visible for dirty workflows: {changed:#}"
    );
}
