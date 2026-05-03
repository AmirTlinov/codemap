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
fn ci_owner_proof_splits_safe_shell_and_validation_steps() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"ci-shell-and-proof","private":true,"scripts":{"test":"vitest run","lint":"eslint . --max-warnings=0"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm test && pnpm run lint\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ci shell and proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".github/workflows/ci.yml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(|surface| surface["command"] == "pnpm test")
            && proofs
                .iter()
                .any(|surface| surface["command"] == "pnpm run lint"),
        "CI shell-and validation should split into independently runnable proof commands: {proof:#}"
    );
    assert!(
        !proofs
            .iter()
            .any(|surface| surface["command"] == "pnpm test && pnpm run lint"),
        "composed shell command must not render as a runnable proof command: {proof:#}"
    );
}

#[test]
fn ci_owner_package_selector_proof_is_run_safe() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"selector-run-fixture","private":true,"workspaces":["apps/*"]}"#,
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"node -e \"process.exit(0)\"","db:migrate:deploy":"prisma migrate deploy"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm --filter @fixture/api test\n      - run: pnpm --filter @fixture/api db:migrate:deploy\n",
    );
    write(
        &repo.path().join("bin/pnpm"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > pnpm-args.txt\n",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(repo.path().join("bin/pnpm"), fs::Permissions::from_mode(0o755))
            .expect("chmod fake pnpm");
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "selector proof"]);

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
            .any(|surface| surface["command"] == "pnpm --filter @fixture/api test")
            && !proofs
                .iter()
                .any(|surface| surface["command"] == "pnpm --filter @fixture/api db:migrate:deploy"),
        "package-selector CI proof should keep validation runnable and deploy out of Proof: {proof:#}"
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
        .expect("proof --run should execute fake pnpm");
    assert!(
        output.status.success(),
        "proof --run should accept rendered package-selector proof command: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let args = fs::read_to_string(repo.path().join("pnpm-args.txt")).expect("pnpm args");
    assert_eq!(args.trim(), "--filter @fixture/api test");
}

#[test]
fn ci_owner_direct_script_proof_is_run_safe() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("package.json"), r#"{"name":"direct-script-proof"}"#);
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: ./scripts/e2e_smoke.sh --headed\n      - run: ./scripts/deploy_test.sh\n",
    );
    let smoke_script = repo.path().join("scripts/e2e_smoke.sh");
    let deploy_script = repo.path().join("scripts/deploy_test.sh");
    write(&smoke_script, "#!/bin/sh\nprintf ok > script-proof-ran.txt\n");
    write(&deploy_script, "#!/bin/sh\nprintf bad > deploy-proof-ran.txt\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for script in [&smoke_script, &deploy_script] {
            fs::set_permissions(script, fs::Permissions::from_mode(0o755)).expect("chmod script");
        }
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "direct script proof"]);

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
            .any(|surface| surface["command"] == "./scripts/e2e_smoke.sh --headed")
            && !proofs
                .iter()
                .any(|surface| surface["command"] == "./scripts/deploy_test.sh"),
        "direct validation scripts should be runnable proof, deploy-like scripts should not: {proof:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", ".github/workflows/ci.yml", "--run"])
        .output()
        .expect("proof --run should execute script");
    assert!(
        output.status.success(),
        "proof --run should accept rendered direct script proof command: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("script-proof-ran.txt"))
            .expect("script proof ran"),
        "ok"
    );
    assert!(
        !repo.path().join("deploy-proof-ran.txt").exists(),
        "deploy-like script must not run as proof"
    );
}

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
        "read-only migration status steps should be CI validation proof: {proof:#}"
    );
    assert!(
        !proofs.iter().any(|surface| surface["command"] == "pnpm run db:migrate:deploy"),
        "mutating/deploy migration scripts must not become CI validation proof: {proof:#}"
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
