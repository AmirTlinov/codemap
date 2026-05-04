#[test]
fn proof_wiring_exposes_missing_command_dead_evidence_and_contract_gap() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Makefile"),
        "validate-receipts:\n\tpython3 tools/validate_receipts.py\n",
    );
    write(
        &repo.path().join("experiments/receipts/admission.json"),
        r#"{
  "schema_version": "1",
  "status": "pass",
  "controls": ["baseline"],
  "proof_command": "make missing-proof",
  "exit_code": 0
}
"#,
    );
    write(
        &repo.path().join("docs/receipt-contract.md"),
        "# Receipt Contract\n\nArtifact: experiments/receipts/admission.json\n\n- `schema_version`\n- `controls`\n- `missing_control`\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof wiring fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "experiments/receipts/admission.json",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let wiring = proof["wiring"].as_array().expect("wiring");
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "runner"
            && fact["status"] == "missing"
            && fact["subject"] == "make missing-proof"),
        "receipt-declared proof_command target should be reported as missing: {proof:#}"
    );
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "evidence_consumption"
            && fact["status"] == "unknown"
            && fact["subject"] == "experiments/receipts/admission.json"),
        "present receipt without consumer should surface unconsumed evidence: {proof:#}"
    );
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "contract_field"
            && fact["status"] == "missing"
            && fact["subject"] == "missing_control"),
        "markdown-declared absent field should surface as a missing contract field: {proof:#}"
    );
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "contract_field"
            && fact["status"] == "unknown"
            && fact["subject"] == "controls"),
        "present control field must stay unconsumed until a predicate/validator consumes it: {proof:#}"
    );
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "contract_field"
            && fact["status"] == "executed"
            && fact["subject"] == "exit_code"),
        "exit_code should be labeled as executed mechanical evidence, not proof correctness: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "proof_runner_unresolved"
                || unknown["kind"] == "consumer_not_found"
                || unknown["kind"] == "predicate_not_found"),
        "wiring gaps should also remain visible as Unknown: {proof:#}"
    );
}

#[test]
fn proof_wiring_reports_load_bearing_consumed_receipt_fields() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Makefile"),
        "validate-receipts:\n\tpython3 tools/validate_receipts.py\n",
    );
    write(
        &repo.path().join("experiments/receipts/admission.json"),
        r#"{
  "schema_version": "1",
  "status": "pass",
  "controls": ["baseline"],
  "proof_command": "make validate-receipts",
  "exit_code": 0
}
"#,
    );
    write(
        &repo.path().join("tools/validate_receipts.py"),
        r#"import json

data = json.load(open("experiments/receipts/admission.json"))
assert data["schema_version"] == "1"
assert "baseline" in data["controls"]
if data["exit_code"] != 0:
    raise SystemExit(1)
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "load bearing proof wiring fixture"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "experiments/receipts/admission.json",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let wiring = proof_map["wiring"].as_array().expect("wiring");
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "runner"
            && fact["status"] == "wired"
            && fact["subject"] == "make validate-receipts"),
        "declared proof command should resolve to Makefile target: {proof_map:#}"
    );
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "contract_field"
            && fact["status"] == "validated"
            && fact["subject"] == "schema_version"),
        "schema field consumed by validator/predicate should be marked validated: {proof_map:#}"
    );
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "contract_field"
            && fact["status"] == "load_bearing"
            && fact["subject"] == "controls"),
        "controls field should be load-bearing when consumed by predicate code: {proof_map:#}"
    );
    assert!(
        wiring.iter().any(|fact| fact["stage"] == "evidence_consumption"
            && fact["status"] == "load_bearing"
            && fact["subject"] == "experiments/receipts/admission.json"),
        "receipt artifact should be load-bearing only when consumed by predicate code: {proof_map:#}"
    );
}

#[test]
fn proof_wiring_resolves_package_scripts_and_missing_package_targets() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"validate:receipts":"node tools/validate_receipts.js receipts/ok.json"}}"#,
    );
    write(
        &repo.path().join("receipts/ok.json"),
        r#"{"status":"pass","proof_command":"pnpm run validate:receipts"}"#,
    );
    write(
        &repo.path().join("receipts/missing.json"),
        r#"{"status":"pass","proof_command":"pnpm run missing:receipt"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "package proof wiring fixture"]);

    let ok = run_json(
        repo.path(),
        cache.path(),
        &["proof", "receipts/ok.json", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &ok);
    let ok_wiring = ok["wiring"].as_array().expect("ok wiring");
    assert!(
        ok_wiring.iter().any(|fact| fact["stage"] == "runner"
            && fact["status"] == "wired"
            && fact["subject"] == "pnpm run validate:receipts"
            && fact["path"] == "package.json"),
        "declared pnpm proof command should resolve to package script: {ok:#}"
    );
    assert!(
        ok_wiring.iter().any(|fact| fact["stage"] == "artifact"
            && fact["status"] == "wired"
            && fact["subject"] == "pnpm run validate:receipts"
            && fact["path"] == "receipts/ok.json"),
        "artifact path mentioned by resolved package script should be wired: {ok:#}"
    );

    let missing = run_json(
        repo.path(),
        cache.path(),
        &["proof", "receipts/missing.json", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &missing);
    let missing_wiring = missing["wiring"].as_array().expect("missing wiring");
    assert!(
        missing_wiring.iter().any(|fact| fact["stage"] == "runner"
            && fact["status"] == "missing"
            && fact["subject"] == "pnpm run missing:receipt"
            && fact["path"] == "package.json"),
        "declared pnpm proof command should expose missing package script target: {missing:#}"
    );
}

#[test]
fn changed_proof_renders_wiring_without_recommendations() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("experiments/receipts/admission.json"),
        r#"{"status":"open","proof_command":"make missing-proof"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "changed proof wiring baseline"]);
    write(
        &repo.path().join("experiments/receipts/admission.json"),
        r#"{"status":"pass","proof_command":"make missing-proof"}"#,
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--section", "proof"])
        .output()
        .expect("changed proof should run");
    assert!(
        output.status.success(),
        "changed proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Proof Wiring")
            && markdown.contains("[missing]")
            && markdown.contains("make missing-proof")
            && markdown.contains("expand: `codemap"),
        "changed proof should render compact wiring gaps with exact expand: {markdown}"
    );
    assert!(
        !markdown.contains("codemap proof --section"),
        "changed proof wiring expand should keep an explicit target: {markdown}"
    );
    assert!(
        !markdown.contains("recommended")
            && !markdown.contains("best")
            && !markdown.contains("safe"),
        "proof wiring must not become advice or judgment: {markdown}"
    );
}

#[test]
fn proof_wiring_does_not_treat_artifact_named_test_files_as_artifacts() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        r#"[package]
name = "artifact-fixture"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(&repo.path().join("src/lib.rs"), "pub fn ok() -> bool { true }\n");
    write(
        &repo.path().join("tests/cache_lens_artifacts.rs"),
        r#"#[test]
fn mentions_fixture_artifact_path() {
    let fixture = "artifacts/example/proof.json";
    assert!(fixture.contains("proof.json"));
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "artifact named test baseline"]);
    write(
        &repo.path().join("tests/cache_lens_artifacts.rs"),
        r#"#[test]
fn mentions_fixture_artifact_path() {
    let fixture = "artifacts/example/proof.json";
    assert!(fixture.ends_with("proof.json"));
}
"#,
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--section", "proof"])
        .output()
        .expect("changed proof should run");
    assert!(
        output.status.success(),
        "changed proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        !markdown.contains("path=`artifacts/example/proof.json`")
            && !markdown.contains("artifact that is absent"),
        "test fixture artifact strings must not become missing proof artifacts: {markdown}"
    );
    assert!(
        markdown.contains("soft:")
            && !markdown
                .contains("[wired] `declared_command` `cargo test` path=`tests/cache_lens_artifacts.rs`"),
        "soft test-name proof surfaces must stay soft in proof wiring: {markdown}"
    );
}
