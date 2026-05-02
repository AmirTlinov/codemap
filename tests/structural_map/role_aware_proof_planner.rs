fn write_role_aware_lab_fixture(repo: &Path) {
    write(
        &repo.join("Makefile"),
        r#".PHONY: qwen-sparse-compute-admission-v0-00675 validate-receipts next doctor deploy
        field-v0-compute-conductance-bridge field-v0-one-slot-local-aperture-contest

qwen-sparse-compute-admission-v0-00675:
	python3 tools/run_qwen_sparse.py

validate-receipts:
	python3 tools/validate_receipts.py

next:
	python3 tools/next.py

doctor:
	python3 tools/doctor.py

deploy:
	echo deploy

field-v0-compute-conductance-bridge:
	python3 tools/internal_economy/compute_conductance_bridge_v0_13b.py

field-v0-one-slot-local-aperture-contest:
	python3 tools/internal_economy/one_slot_local_aperture_contest_v0_29.py
"#,
    );
    write(
        &repo.join("src/tbcm_lab/sparse_admission.py"),
        "def admit_sparse_compute():\n    return True\n",
    );
    write(
        &repo.join("tools/run_qwen_sparse.py"),
        "from tbcm_lab.sparse_admission import admit_sparse_compute\n",
    );
    write(
        &repo.join("tools/validate_receipts.py"),
        "import json\nprint(json.__name__)\n",
    );
    write(&repo.join("tools/doctor.py"), "print('ok')\n");
    write(&repo.join("tools/next.py"), "print('next')\n");
    write(
        &repo.join("tools/internal_economy/compute_conductance_bridge_v0_13b.py"),
        "def run():\n    return 'bridge'\n",
    );
    write(
        &repo.join("tools/internal_economy/one_slot_local_aperture_contest_v0_29.py"),
        "def run():\n    return 'contest'\n",
    );
    write(
        &repo.join("experiments/receipts/sparse-admission-v0.json"),
        r#"{"claim_status":"open","metrics":{"samples":1}}"#,
    );
    write(
        &repo.join("experiments/qwen-sparse-admission.md"),
        "# Sparse admission owner note\n",
    );
}

fn proof_surfaces(proof: &Value) -> Vec<&Value> {
    proof["proofs"].as_array().expect("proofs").iter().collect()
}

fn proof_surface_for<'a>(proof: &'a Value, command: &str) -> &'a Value {
    proof_surfaces(proof)
        .into_iter()
        .find(|surface| surface["command"].as_str() == Some(command))
        .unwrap_or_else(|| panic!("missing proof surface for {command}: {proof:#}"))
}

fn proof_surface_commands(proof: &Value) -> Vec<&str> {
    proof_surfaces(proof)
        .into_iter()
        .filter_map(|surface| surface["command"].as_str())
        .collect()
}

fn assert_makefile_proof_surface(
    surface: &Value,
    evidence: &str,
    strength: &str,
    line_start: u64,
) {
    assert_eq!(surface["path"], "Makefile", "surface path: {surface:#}");
    assert_eq!(surface["evidence"], evidence, "surface evidence: {surface:#}");
    assert_eq!(surface["strength"], strength, "surface strength: {surface:#}");
    let location = &surface["locations"]
        .as_array()
        .expect("locations")
        .first()
        .expect("first location");
    assert_eq!(location["path"], "Makefile", "location path: {surface:#}");
    assert_eq!(
        location["line_start"], line_start,
        "location line: {surface:#}"
    );
    assert_eq!(location["kind"], evidence, "location kind: {surface:#}");
}

fn assert_soft_proof_keeps_unknown(proof: &Value) {
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_deterministic_proof"),
        "soft role-aware evidence must not hide missing deterministic proof Unknown: {proof:#}"
    );
}

#[test]
fn makefile_define_blocks_do_not_emit_script_targets() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Makefile"),
        r#"define BLOCKER
{ printf "\033[31mBLOCKER:\033[0m %s\n" "$(1)" >&2; exit 2; }
endef

doctor:
	python3 tools/doctor.py
"#,
    );
    write(&repo.path().join("tools/doctor.py"), "print('ok')\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "make define fixture"]);

    let status = run_json(repo.path(), cache.path(), &["status", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &status);
    let scripts = status["scripts"].as_array().expect("scripts");
    assert!(
        scripts
            .iter()
            .any(|script| script.as_str() == Some("make doctor")),
        "real Makefile target should still be indexed: {status:#}"
    );
    assert!(
        scripts.iter().all(|script| {
            let value = script.as_str().unwrap_or_default();
            !value.contains("BLOCKER") && !value.contains("printf")
        }),
        "Makefile define macro body must not become script targets: {status:#}"
    );
}

#[test]
fn makefile_targets_can_match_source_files_by_exact_path_tokens() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write_role_aware_lab_fixture(repo.path());
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "role aware lab"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "tools/internal_economy/compute_conductance_bridge_v0_13b.py",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let commands = proof_surface_commands(&proof);
    assert!(
        commands.contains(&"make field-v0-compute-conductance-bridge"),
        "exact source/script token match should infer the matching Makefile target: {proof:#}"
    );
    assert!(
        !commands
            .iter()
            .any(|command| command.contains("one-slot-local-aperture-contest")),
        "common Makefile words must not select an unrelated target: {proof:#}"
    );
    assert_soft_proof_keeps_unknown(&proof);
    assert_makefile_proof_surface(
        proof_surface_for(&proof, "make field-v0-compute-conductance-bridge"),
        "script_path_token",
        "medium",
        19,
    );
}

#[test]
fn custom_lab_roles_are_first_class_file_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write_role_aware_lab_fixture(repo.path());
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "role aware lab"]);

    for (path, kind, role) in [
        (
            "experiments/receipts/sparse-admission-v0.json",
            "receipt",
            "receipt",
        ),
        ("tools/run_qwen_sparse.py", "proof_runner", "proof_runner"),
        (
            "experiments/qwen-sparse-admission.md",
            "owner_doc",
            "owner_doc",
        ),
    ] {
        let ls = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &ls);
        let anchor = &ls["anchor"];
        assert_eq!(anchor["kind"], kind, "{path} should have kind {kind}: {ls:#}");
        assert!(
            anchor["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|value| value.as_str() == Some(role)),
            "{path} should carry role {role}: {ls:#}"
        );
    }
}

#[test]
fn role_aware_proof_uses_repo_commands_before_generic_fallback() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write_role_aware_lab_fixture(repo.path());
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "role aware lab"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "experiments/receipts/sparse-admission-v0.json",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let commands = proof_surface_commands(&proof);
    assert!(
        commands.contains(&"make qwen-sparse-compute-admission-v0-00675"),
        "path-token matched proof target should be present: {proof:#}"
    );
    assert!(
        commands.contains(&"make validate-receipts"),
        "receipt validation target should be present: {proof:#}"
    );
    assert!(
        commands.contains(&"make doctor"),
        "doctor target should be present before generic fallback: {proof:#}"
    );
    assert_soft_proof_keeps_unknown(&proof);
    assert!(
        !commands.iter().any(|command| command.contains("deploy")),
        "mutating deploy target must not be inferred as proof: {proof:#}"
    );
    assert_makefile_proof_surface(
        proof_surface_for(&proof, "make qwen-sparse-compute-admission-v0-00675"),
        "script_path_token",
        "medium",
        4,
    );
    assert_makefile_proof_surface(
        proof_surface_for(&proof, "make validate-receipts"),
        "role_script_target",
        "medium",
        7,
    );
    assert_makefile_proof_surface(
        proof_surface_for(&proof, "make doctor"),
        "role_script_target",
        "medium",
        13,
    );
}

#[test]
fn changed_proof_section_reuses_role_aware_commands() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write_role_aware_lab_fixture(repo.path());
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "role aware lab"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "changed",
            "--files",
            "experiments/receipts/sparse-admission-v0.json",
            "--section",
            "proof",
        ])
        .output()
        .expect("changed proof should run");
    assert!(
        output.status.success(),
        "changed proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("make qwen-sparse-compute-admission-v0-00675"),
        "changed proof should show token-matched Makefile proof target: {markdown}"
    );
    assert!(
        markdown.contains("make validate-receipts"),
        "changed proof should show receipt validation target: {markdown}"
    );
    assert!(
        !markdown.contains("Unknown: None found"),
        "changed proof should stay fail-open when only role-aware soft evidence exists: {markdown}"
    );
}
