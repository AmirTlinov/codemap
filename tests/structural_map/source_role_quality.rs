#[test]
fn source_role_classifiers_keep_doctor_unclassified_noise_low() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/app/orders.service.ts"),
        "export function listOrders() { return []; }\n",
    );
    write(
        &repo.path().join("src/app/orders.controller.ts"),
        "export function routeOrders() { return Response.json({ ok: true }); }\n",
    );
    write(
        &repo.path().join("src/domain/order.ts"),
        "export type Order = { id: string };\n",
    );
    write(
        &repo.path().join("src/modules/billing.module.ts"),
        "export const billingModule = true;\n",
    );
    write(
        &repo.path().join("src/repositories/order_repository.ts"),
        "export const orderRepository = {};\n",
    );
    write(
        &repo.path().join("src/map/lenses/diff_map.rs"),
        "pub fn diff_map() {}\n",
    );
    write(
        &repo.path().join("src/repo/surfaces_core.rs"),
        "pub fn extract_surfaces() {}\n",
    );
    write(
        &repo.path().join("src/repo/js_imports.rs"),
        "pub fn scan_js_imports() {}\n",
    );
    write(
        &repo.path().join("src/repo/scripts_make.rs"),
        "pub fn make_targets() {}\n",
    );
    write(
        &repo.path().join("src/proof_classification.rs"),
        "pub fn classify_proof() {}\n",
    );
    write(
        &repo.path().join("src/cli/args.rs"),
        "pub fn parse_args() {}\n",
    );
    write(
        &repo.path().join("src/repo/project.rs"),
        "pub struct Project { pub root: String }\n",
    );
    write(
        &repo.path().join("src/repo/tests.rs"),
        "#[cfg(test)]\nmod tests {}\n",
    );
    write(
        &repo.path().join("src/render/helpers.rs"),
        "pub fn render_helper() {}\n",
    );
    write(
        &repo.path().join("src/render/prelude.rs"),
        "pub fn prelude() {}\n",
    );
    write(
        &repo.path().join("src/render/proof_wiring.rs"),
        "pub fn render_proof_wiring() {}\n",
    );
    write(
        &repo.path().join("src/repo/component_contracts_core.rs"),
        "pub fn component_contract() {}\n",
    );
    write(
        &repo.path().join("src/repo/component_render_analysis.rs"),
        "pub fn component_render_analysis() {}\n",
    );
    write(
        &repo.path().join("src/render/teach.rs"),
        "pub fn teach() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "source role fixture"]);

    for (path, expected_role) in [
        ("src/app/orders.service.ts", "service"),
        ("src/app/orders.controller.ts", "controller"),
        ("src/domain/order.ts", "domain"),
        ("src/modules/billing.module.ts", "module"),
        ("src/repositories/order_repository.ts", "repository"),
        ("src/map/lenses/diff_map.rs", "map_surface"),
        ("src/repo/surfaces_core.rs", "extractor"),
        ("src/repo/js_imports.rs", "extractor"),
        ("src/repo/scripts_make.rs", "script_catalog"),
        ("src/proof_classification.rs", "role_classifier"),
        ("src/cli/args.rs", "cli_surface"),
        ("src/repo/project.rs", "state_model"),
        ("src/repo/tests.rs", "test_support"),
        ("src/render/helpers.rs", "helper_surface"),
        ("src/render/prelude.rs", "helper_surface"),
        ("src/render/proof_wiring.rs", "proof_surface"),
        (
            "src/repo/component_contracts_core.rs",
            "contract_surface",
        ),
        (
            "src/repo/component_render_analysis.rs",
            "analysis_surface",
        ),
        ("src/render/teach.rs", "teach_surface"),
    ] {
        let ls = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &ls);
        assert!(
            ls["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|role| role == expected_role),
            "{path} should carry deterministic role `{expected_role}`: {ls:#}"
        );
    }

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(
        doctor["unclassified_count"], 0,
        "fixture source files should not show up as unclassified doctor noise: {doctor:#}"
    );

    let changed = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "changed",
            "--files",
            "src/app/orders.service.ts,src/map/lenses/diff_map.rs,src/cli/args.rs",
            "--section",
            "roles",
        ])
        .output()
        .expect("changed roles should run");
    assert!(
        changed.status.success(),
        "changed roles failed: {}",
        String::from_utf8_lossy(&changed.stderr)
    );
    let markdown = String::from_utf8(changed.stdout).expect("markdown utf8");
    for role in ["`service`", "`map_surface`", "`cli_surface`"] {
        assert!(
            markdown.contains(role),
            "changed roles should use the same source role catalog as scanner for {role}: {markdown}"
        );
    }

    let role_markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "src/render/proof_wiring.rs", "--section", "roles"])
        .output()
        .expect("ls roles should run");
    assert!(
        role_markdown.status.success(),
        "ls roles failed: {}",
        String::from_utf8_lossy(&role_markdown.stderr)
    );
    let role_markdown = String::from_utf8(role_markdown.stdout).expect("markdown utf8");
    assert!(
        role_markdown.contains("`proof_surface`"),
        "file role section should preserve first-class proof_surface hints: {role_markdown}"
    );

    let cone_roles = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "src/render/proof_wiring.rs", "--section", "roles"])
        .output()
        .expect("cone roles should run");
    assert!(
        cone_roles.status.success(),
        "cone roles failed: {}",
        String::from_utf8_lossy(&cone_roles.stderr)
    );
    let cone_roles = String::from_utf8(cone_roles.stdout).expect("markdown utf8");
    assert!(
        cone_roles.contains("`proof_surface`"),
        "cone role section should preserve first-class proof_surface hints: {cone_roles}"
    );

    let cone = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "src/render/proof_wiring.rs"])
        .output()
        .expect("cone should run");
    assert!(
        cone.status.success(),
        "cone failed: {}",
        String::from_utf8_lossy(&cone.stderr)
    );
    let cone = String::from_utf8(cone.stdout).expect("markdown utf8");
    assert!(
        cone.contains("`proof_surface` `src/render/proof_wiring.rs`"),
        "x-ray role should expose proof_surface instead of only generic source: {cone}"
    );
}

#[test]
fn source_file_extensions_do_not_become_extractor_roles() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("src/plain.js"), "export const plain = true;\n");
    write(
        &repo.path().join("src/plain.jsx"),
        "export const alsoPlain = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "plain js sources"]);

    for path in ["src/plain.js", "src/plain.jsx"] {
        let ls = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &ls);
        if path.ends_with(".js") {
            assert_eq!(
                ls["anchor"]["kind"], "source",
                "{path} should stay source: {ls:#}"
            );
        }
        assert!(
            !ls["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|role| role == "extractor"),
            "file extension `{path}` must not count as extractor evidence: {ls:#}"
        );
    }

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(
        doctor["unclassified_count"], 1,
        "plain .js should remain honest unclassified source, not be hidden by extension tokens: {doctor:#}"
    );
}

#[test]
fn role_surface_match_is_soft_evidence_without_closing_direct_unknown() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"role-proof-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("src/repo/roles_source.rs"),
        "pub fn classify_source_roles() -> bool { true }\n",
    );
    write(
        &repo
            .path()
            .join("tests/structural_map/source_role_quality.rs"),
        "#[test]\nfn source_role_quality() { assert!(true); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "role proof fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "--files",
            "src/repo/roles_source.rs",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|surface| surface["evidence"] == "test_role_surface_match"
                && surface["strength"] == "medium"
                && surface["target_anchor"] == "src/repo/roles_source.rs"
                && surface["command"] == "cargo test"),
        "shared role should create a soft proof sensor with the package test command and explicit target anchor: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "direct_test_import_not_found"),
        "soft role evidence must not close the direct proof unknown: {proof:#}"
    );

    let markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "--files", "src/repo/roles_source.rs"])
        .output()
        .expect("proof markdown should run");
    assert!(
        markdown.status.success(),
        "proof markdown failed: {}",
        String::from_utf8_lossy(&markdown.stderr)
    );
    let markdown = String::from_utf8(markdown.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Soft Evidence")
            && markdown.contains("test_role_surface_match")
            && markdown.contains("-> `src/repo/roles_source.rs`")
            && markdown.contains("direct_test_import_not_found"),
        "markdown should keep role evidence soft, anchored, and the direct unknown visible: {markdown}"
    );
}
