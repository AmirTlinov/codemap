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
        "soft proof must keep missing direct-link uncertainty visible as Unknown: {proof:#}"
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "src/routes.ts", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["soft_evidence"]
            .as_array()
            .expect("soft evidence")
            .iter()
            .any(|proof| proof["strength"] == "medium"
                && matches!(
                    proof["evidence"].as_str(),
                    Some("test_name" | "test_surface_tokens" | "test_surface_phrase")
                )),
        "proof-map JSON should keep token/name/path overlap in soft_evidence, not a direct bucket: {proof_map:#}"
    );
    assert!(
        proof_map["hard"].as_array().expect("hard").is_empty()
            && proof_map["direct_evidence"]
                .as_array()
                .expect("direct evidence")
                .is_empty(),
        "soft token/name evidence must not be promoted into hard or direct evidence JSON buckets: {proof_map:#}"
    );
    assert!(
        !proof_map["missing_direct"]
            .as_array()
            .expect("missing_direct")
            .is_empty(),
        "proof-map should keep missing direct links visible when only soft matches exist: {proof_map:#}"
    );
    assert!(
        proof_map["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_deterministic_proof"),
        "proof-map should expose soft-proof uncertainty as Unknown: {proof_map:#}"
    );

    let proof_map_markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof-map", "src/routes.ts", "--raw-sensors"])
        .output()
        .expect("proof-map markdown should run");
    assert!(
        proof_map_markdown.status.success(),
        "proof-map markdown failed: {}",
        String::from_utf8_lossy(&proof_map_markdown.stderr)
    );
    let proof_map_text = String::from_utf8(proof_map_markdown.stdout).expect("proof-map utf8");
    assert!(
        proof_map_text.contains("## Soft Surface Matches")
            && proof_map_text.contains("## Unknown")
            && proof_map_text.contains("missing_deterministic_proof")
            && proof_map_text.contains("do not create a direct linked verification surface")
            && !proof_map_text.contains("missing_direct_proof")
            && !proof_map_text.contains("\n## Direct\n")
            && !proof_map_text.contains("\n## Runnable Verification Surfaces\n"),
        "proof-map markdown must keep Unknown when token/name evidence is the only proof sensor: {proof_map_text}"
    );

    let impact = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["impact", "--files", "src/routes.ts"])
        .output()
        .expect("impact should run");
    assert!(
        impact.status.success(),
        "impact failed: {}",
        String::from_utf8_lossy(&impact.stderr)
    );
    let impact_markdown = String::from_utf8(impact.stdout).expect("impact markdown utf8");
    assert!(
        impact_markdown.contains("verification=0; soft="),
        "soft-only verification edges should not inflate the main verification count: {impact_markdown}"
    );

    let changed_links = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", "src/routes.ts", "--section", "links"])
        .output()
        .expect("changed links should run");
    assert!(
        changed_links.status.success(),
        "changed links failed: {}",
        String::from_utf8_lossy(&changed_links.stderr)
    );
    let changed_links_markdown =
        String::from_utf8(changed_links.stdout).expect("changed links markdown utf8");
    assert!(
        ((changed_links_markdown.contains("verification=0; soft=")
            && !changed_links_markdown.contains("verification=1]"))
            || (changed_links_markdown.contains("verification links: 0")
                && changed_links_markdown.contains("soft links: 1")))
            && !changed_links_markdown.contains("proof links: 1"),
        "changed links should not count soft-only proof edges as verification mass: {changed_links_markdown}"
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
        (markdown.contains("- missing_direct_unknown: `1`")
            || markdown.contains("missing_direct_unknown=`1`"))
            && markdown.contains("### Fallback"),
        "changed proof should show missing direct-link uncertainty and fallback with soft matches: {markdown}"
    );
    assert!(
        markdown.contains("Map Snapshot: root=`") && markdown.contains("snapshot=`"),
        "changed maps should carry a compact snapshot boundary in markdown output: {markdown}"
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
        !proof_unknown_markdown.contains("## Verification Surfaces")
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
        proof_only_markdown.contains("## Soft Surface Matches")
            && proof_only_markdown.contains("## Fallback")
            && proof_only_markdown.contains("do not create a direct linked verification surface"),
        "proof --section proof should label soft proof separately and keep fallback commands: {proof_only_markdown}"
    );
    assert!(
        !proof_only_markdown.contains("## Unknown"),
        "proof --section proof should not dump Unknown entries: {proof_only_markdown}"
    );

    let proof_all = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "src/routes.ts", "--all"])
        .output()
        .expect("proof --all should run");
    assert!(
        proof_all.status.success(),
        "proof --all failed: {}",
        String::from_utf8_lossy(&proof_all.stderr)
    );
}

#[test]
fn proof_help_shows_format_escape_hatch_without_making_json_primary() {
    let output = codemap()
        .args(["proof", "--help"])
        .output()
        .expect("proof help should run");
    assert!(
        output.status.success(),
        "proof --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8(output.stdout).expect("help utf8");
    assert!(
        help.contains("--format <FORMAT>")
            && help.contains("markdown is the agent default"),
        "proof help should reveal json as an integration escape hatch without changing the main UX: {help}"
    );
}
