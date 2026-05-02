#[test]
fn proof_links_mixed_e2e_layout_through_test_support_import_chain() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/mixed-layout-panel.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/mixed-layout.spec.ts"
            && surface["evidence"] == "test_support_import"
            && surface["strength"] == "high"
            && surface["command"]
                .as_str()
                .unwrap_or_default()
                .contains("test:e2e")),
        "e2e spec should link through test support/page-object import chain, not fallback: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "support import chain should avoid broad fallback: {proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/mixed-layout-panel.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof edges")
            .iter()
            .any(
                |edge| edge["from"] == "packages/app/tests/e2e/mixed-layout.spec.ts"
                    && edge["evidence"] == "test_support_import"
            ),
        "cone should show the same structural proof edge: {cone:#}"
    );
}


#[test]
fn support_import_chain_beats_matching_test_name_for_e2e_specs() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/foo.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(
            |surface| surface["path"] == "packages/app/tests/e2e/foo.spec.ts"
                && surface["evidence"] == "test_support_import"
        ),
        "e2e support import chain is stronger map evidence than matching test name: {proof:#}"
    );
    assert!(
        proofs.iter().all(
            |surface| !(surface["path"] == "packages/app/tests/e2e/foo.spec.ts"
                && surface["evidence"] == "test_name")
        ),
        "matching e2e spec name must not mask the import chain: {proof:#}"
    );
}


#[test]
fn python_proof_without_package_manifest_uses_pytest_file_and_skips_init_support() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo
            .path()
            .join("economy_analytics/metric_contract/surface_specs/catalog.py"),
        "SURFACES = {}\n",
    );
    write(
        &repo.path().join("tests/__init__.py"),
        "from economy_analytics.metric_contract.surface_specs.catalog import SURFACES\n",
    );
    write(
        &repo.path().join("tests/economy_analytics/__init__.py"),
        "from economy_analytics.metric_contract.surface_specs.catalog import SURFACES\n",
    );
    write(
        &repo.path().join("tests/economy_analytics/test_catalog.py"),
        "from economy_analytics.metric_contract.surface_specs.catalog import SURFACES\n\n\ndef test_catalog_exports_surfaces():\n    assert SURFACES == {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "economy_analytics/metric_contract/surface_specs/catalog.py",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(
            |surface| surface["path"] == "tests/economy_analytics/test_catalog.py"
                && surface["evidence"] == "test_import"
                && surface["command"] == "pytest tests/economy_analytics/test_catalog.py"
                && surface["locations"][0]["kind"] == "import_statement"
                && surface["locations"][0]["line_start"] == 1
        ),
        "python test file proof should be runnable without package manifest: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "tests/__init__.py"
                && surface["path"] != "tests/economy_analytics/__init__.py"),
        "python package marker files are test support, not proof: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "file-level pytest proof should suppress broad fallback: {proof:#}"
    );
}
