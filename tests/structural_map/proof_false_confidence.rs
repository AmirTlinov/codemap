#[test]
fn soft_token_proof_does_not_hide_missing_deterministic_proof_or_fallback() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"soft-proof-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/routes.ts"),
        "export function createPartnerOrderRoute() {\n  return 'partner-order-route';\n}\n",
    );
    write(
        &repo.path().join("tests/routes.test.ts"),
        "test('partner order route smoke', () => {\n  expect('create partner order route').toBeTruthy();\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "soft proof fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "src/routes.ts", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|proof| proof["strength"] == "medium"
                && matches!(
                    proof["evidence"].as_str(),
                    Some("test_name" | "test_surface_tokens" | "test_surface_phrase")
                )),
        "fixture should expose only soft token/name proof evidence: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "soft proof must not suppress fallback proof commands: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_deterministic_proof"),
        "soft proof must keep missing deterministic proof visible as Unknown: {proof:#}"
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "src/routes.ts", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        !proof_map["missing_direct"]
            .as_array()
            .expect("missing_direct")
            .is_empty(),
        "proof-map should keep missing direct proof visible when only soft evidence exists: {proof_map:#}"
    );
    assert!(
        proof_map["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_deterministic_proof"),
        "proof-map should expose soft-proof uncertainty as Unknown: {proof_map:#}"
    );

    let changed = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", "src/routes.ts", "--section", "proof"])
        .output()
        .expect("changed proof should run");
    assert!(
        changed.status.success(),
        "changed proof failed: {}",
        String::from_utf8_lossy(&changed.stderr)
    );
    let markdown = String::from_utf8(changed.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("- missing_direct_unknown: `1`") && markdown.contains("### Fallback"),
        "changed proof should show missing deterministic proof and fallback with soft evidence: {markdown}"
    );

    let proof_unknown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "src/routes.ts", "--section", "unknown"])
        .output()
        .expect("proof unknown section should run");
    assert!(
        proof_unknown.status.success(),
        "proof unknown section failed: {}",
        String::from_utf8_lossy(&proof_unknown.stderr)
    );
    let proof_unknown_markdown =
        String::from_utf8(proof_unknown.stdout).expect("unknown markdown utf8");
    assert!(
        proof_unknown_markdown.contains("## Unknown")
            && proof_unknown_markdown.contains("missing_deterministic_proof"),
        "proof --section unknown should isolate proof Unknown entries: {proof_unknown_markdown}"
    );
    assert!(
        !proof_unknown_markdown.contains("## Proof")
            && !proof_unknown_markdown.contains("## Fallback"),
        "proof --section unknown should not dump proof/fallback sections: {proof_unknown_markdown}"
    );

    let proof_only = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "src/routes.ts", "--section", "proof"])
        .output()
        .expect("proof section should run");
    assert!(
        proof_only.status.success(),
        "proof section failed: {}",
        String::from_utf8_lossy(&proof_only.stderr)
    );
    let proof_only_markdown = String::from_utf8(proof_only.stdout).expect("proof markdown utf8");
    assert!(
        proof_only_markdown.contains("## Soft Evidence")
            && proof_only_markdown.contains("## Fallback")
            && proof_only_markdown.contains("does not replace deterministic proof"),
        "proof --section proof should label soft proof separately and keep fallback commands: {proof_only_markdown}"
    );
    assert!(
        !proof_only_markdown.contains("## Unknown"),
        "proof --section proof should not dump Unknown entries: {proof_only_markdown}"
    );
}
