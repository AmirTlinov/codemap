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
