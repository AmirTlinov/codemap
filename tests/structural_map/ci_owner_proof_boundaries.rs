// Responsibility: ci-owner-proof-boundaries
#[test]
fn ci_owner_proof_preserves_scoped_cd_shell_and_steps_for_run() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"root-scope-fixture","private":true,"scripts":{"test":"pwd > npm-pwd-root.txt"}}"#,
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"api-scope-fixture","private":true,"scripts":{"test":"pwd > ../../npm-pwd.txt"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cd apps/api && npm test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci scoped cd proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".github/workflows/ci.yml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs
            .iter()
            .any(|surface| surface["command"] == "cd apps/api && npm test")
            && !proofs.iter().any(|surface| surface["command"] == "npm test"),
        "CI scoped proof should preserve cd scope instead of running root tests: {proof:#}"
    );

    let pwd_out = repo.path().join("npm-pwd.txt");
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", ".github/workflows/ci.yml", "--run"])
        .output()
        .expect("proof --run should execute npm");
    if cfg!(windows) {
        assert!(
            !output.status.success()
                && String::from_utf8_lossy(&output.stderr).contains("POSIX hosts"),
            "Windows must expose the explicit proof --run boundary"
        );
        return;
    }
    assert!(
        output.status.success(),
        "proof --run should accept scoped cd proof command: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let executed_cwd = fs::read_to_string(&pwd_out).expect("npm wrote cwd");
    let expected_cwd = fs::canonicalize(repo.path().join("apps/api")).expect("canonical api cwd");
    assert_eq!(
        executed_cwd.trim(),
        expected_cwd.to_string_lossy(),
        "proof --run must execute npm in the scoped package directory"
    );
}

#[test]
fn ci_owner_proof_rejects_unsplit_shell_control_validation() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"ci-shell-control-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: vitest; echo bad\n      - run: pnpm test | cat\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci shell control proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".github/workflows/ci.yml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().all(|surface| {
            surface["command"]
                .as_str()
                .is_none_or(|command| {
                    !command.contains(';') && !command.contains('|')
                })
        }),
        "CI shell-control commands rejected by --run must not render as runnable proof: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "ci_validation_step_not_found"),
        "rejected shell-control validation should leave explicit CI validation Unknown: {proof:#}"
    );
}

#[test]
fn ci_owner_proof_treats_readonly_migration_status_as_validation() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"ci-migrate-status","private":true,"scripts":{"db:migrate:status":"prisma migrate status --schema prisma/schema.prisma","db:migrate:deploy":"prisma migrate deploy --schema prisma/schema.prisma"}}"#,
    );
    write(
        &repo.path().join("prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\ngenerator client { provider = \"prisma-client-js\" }\nmodel User { id String @id }\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  db:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm run db:migrate:status\n      - run: prisma migrate status --schema prisma/schema.prisma\n      - run: pnpm run db:migrate:deploy\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci migrate status proof"]);

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
                && surface["command"] == "pnpm run db:migrate:status"
        }) && proofs.iter().any(|surface| {
            surface["evidence"] == "ci_run_step"
                && surface["strength"] == "hard"
                && surface["command"] == "prisma migrate status --schema prisma/schema.prisma"
        }),
        "read-only migration status steps should be CI validation command surfaces: {proof:#}"
    );
    assert!(
        !proofs.iter().any(|surface| surface["command"] == "pnpm run db:migrate:deploy"),
        "mutating/deploy migration scripts must not become CI validation command surfaces: {proof:#}"
    );
    assert!(
        !proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "ci_validation_step_not_found"),
        "readonly migration status validation should not leave stale CI validation Unknown: {proof:#}"
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
        "missing CI validation surface should keep conservative fallback commands visible: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "ci_validation_step_not_found"),
        "missing CI validation surface should be explicit Unknown, not soft-surface silence: {proof:#}"
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
        "proof changed should keep the missing CI validation surface visible for dirty workflows: {changed:#}"
    );
}
