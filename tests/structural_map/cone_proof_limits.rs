#[test]
fn cone_json_keeps_all_proof_edges_and_readable_counts_hidden_before_limit() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/multi-proof.ts"),
        "export function multiProof() {\n  return true;\n}\n",
    );
    for index in 1..=5 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/tests/multi-proof-{index}.test.ts")),
            "import { multiProof } from '../src/multi-proof';\n\ntest('multi proof', () => {\n  expect(multiProof()).toBe(true);\n});\n",
        );
    }

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/replay/src/multi-proof.ts",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(
        cone["proof"].as_array().expect("proof").len(),
        5,
        "machine output must serialize every observed proof edge: {cone:#}"
    );
    assert!(
        cone["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .all(|group| group["reason"] != "verification edges hidden by limit"),
        "full JSON must not claim that serialized proof edges are hidden: {cone:#}"
    );
    let markdown = run_markdown(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/replay/src/multi-proof.ts",
            "--limit",
            "1",
        ],
    );
    assert!(
        markdown.contains("verification edges hidden by limit: 4"),
        "bounded readable cone must count all proof edges before projection: {markdown}"
    );
}
