#[test]
fn proof_help_exposes_stable_rfc_sections() {
    let output = codemap()
        .args(["proof", "--help"])
        .output()
        .expect("proof help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help utf8");
    for expected in ["observed", "links", "roles", "proof", "unknown", "hidden"] {
        assert!(
            stdout.contains(expected),
            "proof help should expose RFC section `{expected}`: {stdout}"
        );
    }
}

#[test]
fn proof_section_filter_does_not_filter_json_report() {
    let (repo, cache) = fixture();
    let json = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/session.ts",
            "--section",
            "unknown",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &json);
    assert!(
        !json["proofs"].as_array().expect("proofs").is_empty(),
        "section filter should affect markdown only, not JSON report facts: {json:#}"
    );
}
