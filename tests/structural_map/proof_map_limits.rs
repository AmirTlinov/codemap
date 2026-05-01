#[test]
fn proof_map_counts_hidden_direct_surfaces_before_limit() {
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

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "packages/replay/src/multi-proof.ts",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert_eq!(
        proof_map["direct"].as_array().expect("direct").len(),
        1
    );
    assert!(
        proof_map["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "direct proof surfaces hidden by limit"
                && group["count"] == 2
                && group["expand"]
                    == "codemap proof-map packages/replay/src/multi-proof.ts --limit 3"),
        "proof-map should count direct proof sensors before display truncation: {proof_map:#}"
    );
}
