#[test]
fn cone_counts_hidden_proof_edges_before_limit() {
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
    assert_eq!(cone["proof"].as_array().expect("proof").len(), 1);
    assert!(
        cone["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "proof edges hidden by limit"
                && group["count"] == 4
                && group["expand"]
                    == "codemap cone packages/replay/src/multi-proof.ts --depth 1 --include-hidden --limit 5"),
        "cone should count all direct proof edges before truncating the proof section: {cone:#}"
    );
}
