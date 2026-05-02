#[test]
fn proof_and_impact_expose_structural_edges_without_scores() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/plain-value.ts"),
        "export const plainValue = 1;\n",
    );
    write(
        &repo.path().join("packages/replay/src/plain-consumer.ts"),
        "import { plainValue } from './plain-value';\n\nexport const doubled = plainValue * 2;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "plain direct consumer"]);

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/plain-value.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    let cluster = &impact["clusters"][0];
    assert_eq!(
        cluster.get("risk"),
        None,
        "impact JSON must not expose score-like verdict fields: {impact:#}"
    );
    assert!(
        cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/plain-consumer.ts"),
        "plain direct consumers should remain visible as source-backed edges: {impact:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/plain-value.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert_eq!(
        proof.get("risk"),
        None,
        "proof JSON must list proof surfaces without a score-like verdict: {proof:#}"
    );
}

#[test]
fn proof_markdown_renders_compact_summary_without_scores() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "packages/replay/src/session.ts"])
        .output()
        .expect("proof markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Summary\n") && markdown.contains("- target anchors: `"),
        "proof markdown should render compact summary bullets: {markdown}"
    );
    assert!(
        !markdown.contains("| Field | Value |"),
        "proof markdown should not use Field/Value table for summary facts: {markdown}"
    );
}

#[test]
fn impact_hidden_changed_expand_preserves_explicit_files_and_depth() {
    let (repo, cache) = fixture();

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/session.ts,packages/replay/src/types.ts",
            "--depth",
            "3",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    let hidden = impact["hidden"].as_array().expect("hidden");
    assert!(
        hidden.iter().any(|group| group["reason"] == "changed anchors hidden by limit"
            && group["expand"].as_str().is_some_and(|expand| {
                expand
                    == "codemap impact --files packages/replay/src/session.ts,packages/replay/src/types.ts --depth 3 --limit 2"
            })),
        "impact hidden changed anchors expand should preserve file selector and depth: {impact:#}"
    );
    assert!(
        hidden.iter().all(|group| group["expand"]
            .as_str()
            .is_none_or(|expand| !expand.contains("<larger-number>"))),
        "impact hidden expands should not emit placeholder limits: {impact:#}"
    );
}

#[test]
fn impact_and_proof_are_structural_without_structural_flag() {
    let (repo, cache) = fixture();
    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/types.ts",
            "--depth",
            "2",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    assert_eq!(impact["kind"], "impact_report");
    assert_eq!(impact["schema_version"], "4");
    let cluster = &impact["clusters"][0];
    assert_eq!(cluster.get("risk"), None);
    assert!(
        cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/session.ts")
    );
    assert!(
        cluster["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/tests/session.test.ts")
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(
                |proof| proof["path"] == "packages/replay/tests/session.test.ts"
                    && proof["command"]
                        .as_str()
                        .unwrap_or_default()
                        .contains("vitest run")
                    && proof["locations"][0]["path"] == "packages/replay/tests/session.test.ts"
                    && proof["locations"][0]["line_start"] == 1
                    && proof["locations"][0]["kind"] == "import_statement"
            )
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|proof| proof["path"] != "packages/replay/tests/session-surface-smoke.test.ts"),
        "token-only unit proof should stay hidden when direct import proof exists"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|proof| proof["path"] != "packages/replay/tests/support/setup.ts"),
        "test support files are map surfaces, not runnable proof"
    );
}
