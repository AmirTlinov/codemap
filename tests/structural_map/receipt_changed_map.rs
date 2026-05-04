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

#[test]
fn changed_artifact_proof_json_is_not_config_key_mutation() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    let rel = "artifacts/147/20260503T223955863568000Z-147-proof/proof.json";
    write(
        &repo.path().join(rel),
        r#"{
  "schema_version": "1",
  "claim_status": "open",
  "base_ref": "main",
  "controls": ["baseline"]
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "artifact proof baseline"]);

    write(
        &repo.path().join(rel),
        r#"{
  "schema_version": "2",
  "claim_status": "pass",
  "base_ref": "main",
  "head_ref": "slice",
  "controls": ["baseline", "doctor"]
}
"#,
    );

    let changed = run_json(
        repo.path(),
        cache.path(),
        &[
            "changed",
            "--files",
            rel,
            "--section",
            "observed",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    let events = changed["structural_events"]
        .as_array()
        .expect("structural events");
    assert!(
        events.iter().any(|event| event["kind"] == "changed_receipt_section"
            && event["effect"]
                .as_str()
                .is_some_and(|effect| effect.contains("claim_status"))),
        "artifact proof JSON should be treated as a receipt/witness payload with bucketed evidence: {changed:#}"
    );
    assert!(
        !events.iter().any(|event| event["kind"] == "added_config_key"
            || event["kind"] == "removed_config_key"),
        "artifact proof JSON must not become runtime/config key mutation: {changed:#}"
    );
}

#[test]
fn proof_artifact_anchor_does_not_get_broad_package_fallback() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"artifact-proof-fallback\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let rel = "artifacts/147/20260503T223955863568000Z-147-proof/proof.json";
    write(
        &repo.path().join(rel),
        r#"{"claim_status":"pass","proof_command":"cargo test"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "artifact proof anchor"]);

    let proof = run_json(repo.path(), cache.path(), &["proof", rel, "--format", "json"]);
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "support artifact anchors must not receive broad ritual package fallback: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_deterministic_proof"),
        "suppressing fallback must still leave explicit Unknown evidence: {proof:#}"
    );
}

#[test]
fn proof_files_artifact_change_stays_fail_open_without_fallback() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"artifact-proof-files\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let rel = "artifacts/147/foo-proof/proof.json";
    write(
        &repo.path().join(rel),
        r#"{"claim_status":"pass","proof_command":"cargo test"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "artifact proof files"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "--files", rel, "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "support artifact changed/file mode must not get broad package fallback: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_deterministic_proof"
                && unknown["path"] == rel),
        "support artifact changed/file mode must stay fail-open with an explicit Unknown: {proof:#}"
    );
}

#[test]
fn proof_changed_artifact_change_stays_fail_open_without_fallback() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"artifact-proof-changed\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let rel = "artifacts/147/foo-proof/proof.json";
    write(
        &repo.path().join(rel),
        r#"{"claim_status":"open","proof_command":"cargo test"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "artifact proof changed baseline"]);
    write(
        &repo.path().join(rel),
        r#"{"claim_status":"pass","proof_command":"cargo test"}"#,
    );

    let proof = run_json(repo.path(), cache.path(), &["proof", "changed", "--format", "json"]);
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "support artifact proof changed mode must not get broad package fallback: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_deterministic_proof"
                && unknown["path"] == rel),
        "support artifact proof changed mode must stay fail-open with explicit Unknown: {proof:#}"
    );
}

#[test]
fn changed_artifact_proof_section_surfaces_missing_deterministic_unknown() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"artifact-changed-unknown\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let rel = "artifacts/147/foo-proof/proof.json";
    write(
        &repo.path().join(rel),
        r#"{"claim_status":"open","proof_command":"cargo test"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "artifact changed baseline"]);
    write(
        &repo.path().join(rel),
        r#"{"claim_status":"pass","proof_command":"cargo test"}"#,
    );

    let markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", rel])
        .output()
        .expect("changed should run");
    assert!(
        markdown.status.success(),
        "changed failed: {}",
        String::from_utf8_lossy(&markdown.stderr)
    );
    let markdown = String::from_utf8(markdown.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Unknown") && markdown.contains("missing_deterministic_proof"),
        "changed map must not hide missing deterministic proof for support artifact changes: {markdown}"
    );
    assert!(
        !markdown.contains("### Fallback") && !markdown.contains("cargo test\n```"),
        "changed map must not replace missing artifact proof with broad package fallback: {markdown}"
    );
}
