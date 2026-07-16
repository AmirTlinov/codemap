#[test]
fn ci_owner_bare_tool_proof_is_run_safe() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("package.json"), r#"{"name":"bare-tool-proof"}"#);
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: vitest run\n      - run: playwright test tests/app.spec.ts\n",
    );
    write(
        &repo.path().join("bin/vitest"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > vitest-args.txt\n",
    );
    write(
        &repo.path().join("bin/playwright"),
        "#!/bin/sh\nprintf '%s\\n' \"$*\" > playwright-args.txt\n",
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for tool in [repo.path().join("bin/vitest"), repo.path().join("bin/playwright")] {
            fs::set_permissions(tool, fs::Permissions::from_mode(0o755)).expect("chmod tool");
        }
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "bare tool proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".github/workflows/ci.yml", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(|surface| surface["command"] == "vitest run")
            && proofs
                .iter()
                .any(|surface| surface["command"] == "playwright test tests/app.spec.ts"),
        "bare CI validation tools should render as proof: {proof:#}"
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
        .expect("proof --run should execute bare tools");
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
        "proof --run should accept rendered bare tool proof commands: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("vitest-args.txt"))
            .expect("vitest args")
            .trim(),
        "run"
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("playwright-args.txt"))
            .expect("playwright args")
            .trim(),
        "test tests/app.spec.ts"
    );
}
