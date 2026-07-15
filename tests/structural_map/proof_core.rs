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
    assert_eq!(impact["schema_version"], "7");
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

#[test]
fn proof_token_surfaces_do_not_cross_package_boundaries() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/order-route.ts"),
        "export function createPartnerOrderRoute() {\n  return 'order-route';\n}\n",
    );
    write(
        &repo.path().join("packages/replay/tests/order-route-smoke.test.ts"),
        "test('order route smoke', () => {\n  expect('create partner order route').toBeTruthy();\n});\n",
    );
    write(
        &repo.path().join("packages/app/tests/order-route-smoke.test.ts"),
        "test('order route smoke', () => {\n  expect('create partner order route').toBeTruthy();\n});\n",
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/order-route.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let paths = proof["proofs"]
        .as_array()
        .expect("proofs")
        .iter()
        .filter_map(|proof| proof["path"].as_str())
        .collect::<Vec<_>>();
    assert!(
        paths.contains(&"packages/replay/tests/order-route-smoke.test.ts"),
        "same-package soft proof should remain visible: {proof:#}"
    );
    let same_package_proof = proof["proofs"]
        .as_array()
        .expect("proofs")
        .iter()
        .find(|proof| proof["path"] == "packages/replay/tests/order-route-smoke.test.ts")
        .expect("same-package proof");
    assert_eq!(same_package_proof["evidence"], "test_surface_tokens");
    assert_eq!(same_package_proof["locations"][0]["line_start"], 1);
    assert_eq!(same_package_proof["locations"][0]["kind"], "test_surface");
    assert!(
        !paths.contains(&"packages/app/tests/order-route-smoke.test.ts"),
        "soft token proof must not jump to a sibling package without hard evidence: {proof:#}"
    );
}

#[test]
fn proof_token_surface_locations_skip_unrelated_first_tests() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/order-route.ts"),
        "export function createPartnerOrderRoute() {\n  return 'order-route';\n}\n",
    );
    write(
        &repo.path().join("packages/replay/tests/order-route-smoke.test.ts"),
        "test('unrelated smoke', () => {\n  expect('health').toBeTruthy();\n});\n\ntest('order route smoke', () => {\n  expect('create partner order route').toBeTruthy();\n});\n",
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/order-route.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let surface = proof["proofs"]
        .as_array()
        .expect("proofs")
        .iter()
        .find(|proof| proof["path"] == "packages/replay/tests/order-route-smoke.test.ts")
        .expect("soft proof");
    assert_eq!(surface["evidence"], "test_surface_tokens");
    assert_eq!(surface["locations"][0]["kind"], "test_surface");
    assert_eq!(surface["locations"][0]["line_start"], 5);
}

#[test]
fn proof_import_locations_find_multiline_named_imports() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/multiline-direct.ts"),
        "export function multilineDirectValue() {\n  return 1;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/replay/tests/multiline-direct.test.ts"),
        "import {\n  multilineDirectValue,\n} from '../src/multiline-direct';\n\ntest('multiline direct value', () => {\n  expect(multilineDirectValue()).toBe(1);\n});\n",
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/multiline-direct.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let direct = proof["proofs"]
        .as_array()
        .expect("proofs")
        .iter()
        .find(|proof| proof["path"] == "packages/replay/tests/multiline-direct.test.ts")
        .expect("direct proof");
    assert_eq!(direct["evidence"], "test_import");
    assert_eq!(direct["locations"][0]["kind"], "import_statement");
    assert_eq!(direct["locations"][0]["line_start"], 2);
}
