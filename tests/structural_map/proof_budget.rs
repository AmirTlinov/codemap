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
    assert_eq!(proof["schema_version"], "5");
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
