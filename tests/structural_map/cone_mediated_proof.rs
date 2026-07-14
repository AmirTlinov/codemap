#[test]
fn cone_shows_proof_edges_through_direct_consumers() {
    let (repo, cache) = fixture();
    let public_impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/public-only.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &public_impact);
    assert_eq!(public_impact["clusters"][0].get("risk"), None);

    let public_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/public-only.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &public_proof);
    assert_eq!(
        public_proof.get("risk"),
        None,
        "proof should expose proof surfaces without a score-like verdict: {public_proof:#}"
    );
    assert!(
        public_proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|proof| proof["path"] == "packages/replay/tests/public-api.test.ts"
                && proof["evidence"] == "test_import_via_direct_consumer"
                && proof["strength"] == "medium"),
        "proof should expose via-consumer evidence as mediated/medium, not a direct verification surface: {public_proof:#}"
    );
    assert!(
        public_proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "direct_test_import_not_found"),
        "via-consumer surface must not hide the missing direct verification surface for the anchor: {public_proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/replay/src/public-only.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/index.ts"
                && edge["to"] == "packages/replay/src/public-only.ts"),
        "direct public consumer should be visible before proof via consumer is trusted: {cone:#}"
    );
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/tests/public-api.test.ts"
                && edge["to"] == "packages/replay/src/public-only.ts"
                && edge["evidence"] == "test_import_via_direct_consumer"
                && edge["strength"] == "medium"),
        "cone should show mediated proof reachable through the direct consumer without calling it high/direct proof: {cone:#}"
    );
    let cone_markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "packages/replay/src/public-only.ts"])
        .output()
        .expect("cone markdown should run");
    assert!(
        cone_markdown.status.success(),
        "cone markdown failed: {}",
        String::from_utf8_lossy(&cone_markdown.stderr)
    );
    let cone_markdown = String::from_utf8(cone_markdown.stdout).expect("markdown utf8");
    assert!(
        cone_markdown.contains("## Soft Surface Matches")
            && cone_markdown.contains("test_import_via_direct_consumer")
            && !cone_markdown.contains("## Verification Surfaces\n\nproof:"),
        "cone markdown must not render mediated verification links under generic runnable surfaces: {cone_markdown}"
    );

    let session_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &session_cone);
    assert!(
        session_cone["proof"]
            .as_array()
            .expect("session proof")
            .iter()
            .all(|edge| edge["from"] != "packages/replay/tests/public-api.test.ts"),
        "a test importing a shared public consumer must still mention this anchor before becoming via-consumer proof: {session_cone:#}"
    );
}
