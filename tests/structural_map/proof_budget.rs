#[test]
fn proof_limit_reports_hidden_surfaces_with_exact_target_expand() {
    let (repo, cache) = fixture();

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert_eq!(proof["schema_version"], "7");
    assert_eq!(proof["proofs"].as_array().expect("proofs").len(), 1);
    assert!(
        proof["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "proof surfaces hidden by limit"
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.starts_with(
                        "codemap proof packages/replay/src --depth 1 --limit ",
                    ) && !expand.contains("<larger-number>")
                })),
        "proof should expose hidden proof surfaces instead of silently truncating: {proof:#}"
    );
}

#[test]
fn proof_exact_file_target_counts_hidden_direct_tests_before_limit() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/multi-proof.ts"),
        "export function multiProof() {\n  return true;\n}\n",
    );
    for index in 1..=3 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/tests/multi-proof-{index}.test.ts")),
            "import { multiProof } from '../src/multi-proof';\n\ntest('multi proof', () => {\n  expect(multiProof()).toBe(true);\n});\n",
        );
    }

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/multi-proof.ts",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert_eq!(proof["proofs"].as_array().expect("proofs").len(), 1);
    assert!(
        proof["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "proof surfaces hidden by limit"
                && group["count"] == 2
                && group["expand"]
                    == "codemap proof packages/replay/src/multi-proof.ts --depth 1 --limit 3"),
        "exact file proof should count hidden direct tests before display truncation: {proof:#}"
    );
}

#[test]
fn proof_hidden_expand_preserves_explicit_files_selector() {
    let (repo, cache) = fixture();

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "--files",
            "packages/replay/src/session.ts,packages/replay/src/types.ts",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["expand"].as_str().is_some_and(|expand| {
                expand.starts_with("codemap proof --files packages/replay/src/session.ts,packages/replay/src/types.ts --depth 1 --limit ")
                    && !expand.contains("--changed")
                    && !expand.contains("<larger-number>")
            })),
        "proof hidden expand should preserve the explicit files selector: {proof:#}"
    );
}

#[test]
fn workspace_manifest_proof_budget_keeps_script_and_ci_evidence_visible() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"proof-budget-workspace","private":true,"packageManager":"pnpm@9.15.0","scripts":{"build":"turbo build","lint":"turbo lint","test":"turbo test","verify:a":"node scripts/a.mjs","verify:b":"node scripts/b.mjs","verify:c":"node scripts/c.mjs","verify:d":"node scripts/d.mjs","verify:e":"node scripts/e.mjs"}}"#,
    );
    write(&repo.path().join("pnpm-workspace.yaml"), "packages:\n  - \"apps/*\"\n");
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm verify:a\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "workspace proof budget"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "pnpm-workspace.yaml",
            "--limit",
            "4",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert_eq!(
        proofs.len(),
        4,
        "visible proof surfaces should honor --limit: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .any(|surface| surface["evidence"] == "workspace_manifest_script"),
        "bounded workspace proof should keep root script evidence visible: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .any(|surface| surface["evidence"] == "workspace_manifest_ci_reference"),
        "bounded workspace proof should keep CI evidence visible: {proof:#}"
    );
    assert!(
        proof["hidden"].as_array().expect("hidden").iter().any(|group| {
            group["reason"] == "proof surfaces hidden by limit"
                && group["count"].as_u64().unwrap_or_default() > 0
                && group["expand"]
                    .as_str()
                    .is_some_and(|expand| expand.starts_with("codemap proof pnpm-workspace.yaml --depth 1 --limit "))
        }),
        "hidden proof surfaces should remain explicit with exact expand: {proof:#}"
    );
}

#[test]
fn workspace_manifest_ci_proof_rejects_shell_test_builtin_as_script() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"proof-shell-test-workspace","private":true,"packageManager":"pnpm@9.15.0","scripts":{"test":"turbo test"}}"#,
    );
    write(&repo.path().join("pnpm-workspace.yaml"), "packages:\n  - \"apps/*\"\n");
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: test -n \"${PROD_HOST}\" || exit 1\n      - run: pnpm test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "workspace shell test proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "pnpm-workspace.yaml",
            "--limit",
            "20",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(|surface| {
            surface["evidence"] == "workspace_manifest_ci_reference"
                && surface["command"] == "pnpm test"
        }),
        "real package script runner should remain deterministic CI proof: {proof:#}"
    );
    assert!(
        !proofs.iter().any(|surface| {
            surface["evidence"] == "workspace_manifest_ci_reference"
                && surface["command"]
                    .as_str()
                    .is_some_and(|command| command.starts_with("test -n"))
        }),
        "shell builtin `test` must not be treated as invoking the package script: {proof:#}"
    );
}
