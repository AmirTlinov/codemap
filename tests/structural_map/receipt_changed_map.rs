#[test]
fn changed_receipt_json_reports_buckets_not_internal_key_spam() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("experiments/receipts/admission.json"),
        r#"{
  "schema_version": "1",
  "claim_status": "open",
  "claim_boundary": "baseline",
  "metrics": { "accepted": 1 },
  "controls": ["baseline"],
  "proof_command": "make old-proof",
  "token_ids": [101, 102]
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "receipt baseline"]);

    write(
        &repo.path().join("experiments/receipts/admission.json"),
        r#"{
  "schema_version": "2",
  "claim_status": "closed",
  "claim_boundary": "sparse admission",
  "metrics": { "accepted": 3 },
  "controls": ["baseline", "counterfactual"],
  "proof_command": "make validate-receipts",
  "token_ids": [101, 102, 999],
  "internal_trace_ids": ["abc", "def"]
}
"#,
    );

    let changed = run_json(
        repo.path(),
        cache.path(),
        &[
            "changed",
            "--files",
            "experiments/receipts/admission.json",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    let events = changed["structural_events"]
        .as_array()
        .expect("structural events");
    for bucket in [
        "claim_status",
        "claim_boundary",
        "metrics",
        "controls",
        "proof_command",
        "schema",
    ] {
        assert!(
            events.iter().any(|event| event["kind"] == "changed_receipt_section"
                && event["effect"]
                    .as_str()
                    .is_some_and(|effect| effect.contains(bucket))
                && event["locations"][0]["line_start"]
                    .as_u64()
                    .unwrap_or_default()
                    > 0),
            "receipt changed map should expose `{bucket}` bucket with line provenance: {changed:#}"
        );
    }
    assert!(
        !events.iter().any(|event| event["kind"] == "added_config_key"
            || event["kind"] == "removed_config_key"),
        "receipt files should not fall back to config-key spam: {changed:#}"
    );
    let rendered = serde_json::to_string(&events).expect("events json");
    assert!(
        !rendered.contains("token_ids") && !rendered.contains("internal_trace_ids"),
        "receipt changed map should not surface internal witness keys as structural events: {changed:#}"
    );
}
