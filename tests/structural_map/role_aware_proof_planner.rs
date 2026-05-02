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
    let commands = proof["fallback"]
        .as_array()
        .expect("fallback")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
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
    let fallback = proof["fallback"].as_array().expect("fallback");
    let commands = fallback
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
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
    assert!(
        !commands.contains(&"pytest"),
        "role-aware Makefile commands should beat generic pytest fallback: {proof:#}"
    );
    assert!(
        !commands.iter().any(|command| command.contains("deploy")),
        "mutating deploy target must not be inferred as proof: {proof:#}"
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
        !markdown.contains("pytest"),
        "changed proof should not fall back to generic pytest when role-aware commands exist: {markdown}"
    );
}
