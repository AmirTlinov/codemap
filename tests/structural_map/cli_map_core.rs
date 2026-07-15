#[test]
fn help_exposes_only_map_first_commands() {
    let output = codemap().arg("--help").output().expect("help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help utf8");
    for expected in [
        "Choose one proportional map entry:",
        "known symbol:     codemap where <symbol>",
        "known anchor:     codemap cone <file-or-file#symbol>",
        "known scope:      codemap ls <file-or-directory>",
        "unfamiliar scope: codemap ls .",
        "root orientation is only for an unknown scope",
        "codemap changed",
        "codemap proof changed",
        "Focused map lenses:",
        "runtime, contract, flow",
        "Diagnostics:",
    ] {
        assert!(
            stdout.contains(expected),
            "help should make the daily surface and drill-down groups obvious: {stdout}"
        );
    }
    for command in ["ls", "where", "cone", "changed", "proof", "graph", "boundaries"] {
        assert!(stdout.contains(command), "help should expose {command}");
    }
    let commands = stdout
        .split("Commands:")
        .nth(1)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .take(6)
        .collect::<Vec<_>>();
    assert_eq!(
        commands,
        vec!["ls", "where", "cone", "changed", "proof", "help"],
        "help should put daily commands first: {stdout}"
    );
    for forbidden in ["start", "locate", "find", "verify", "widen", "read_first"] {
        assert!(
            !stdout.contains(forbidden),
            "help must not expose removed surface {forbidden}"
        );
    }
}
#[test]
fn changed_help_exposes_only_stable_rfc_sections() {
    let output = codemap()
        .args(["changed", "--help"])
        .output()
        .expect("changed help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help utf8");
    for expected in ["observed", "links", "roles", "proof", "unknown", "hidden"] {
        assert!(
            stdout.contains(expected),
            "changed help should expose RFC section `{expected}`: {stdout}"
        );
    }
    for old_section in ["overview", "diff", "impact", "unknowns"] {
        assert!(
            !stdout.contains(old_section),
            "changed help should hide legacy section alias `{old_section}`: {stdout}"
        );
    }
}
#[test]
fn bootstrap_instruction_teaches_map_lenses_not_removed_router_flow() {
    let output = codemap()
        .args(["bootstrap", "--global-instruction"])
        .output()
        .expect("bootstrap should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("bootstrap utf8");
    assert_map_bootstrap_text(&stdout);
    let (repo, cache) = fixture();
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["init", "--agents"])
        .output()
        .expect("init --agents should run");
    assert!(
        output.status.success(),
        "init --agents should write the tiny bootloader: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let agents = fs::read_to_string(repo.path().join("AGENTS.md")).expect("bootloader");
    assert_map_bootstrap_text(&agents);
}
fn assert_map_bootstrap_text(text: &str) {
    for expected in [
        "codemap ls .",
        "codemap ls <file-or-directory>",
        "codemap cone <file-or-file#symbol> --depth 1",
        "codemap where <exact-symbol>",
        "root orientation first when an exact anchor is already known",
        "codemap changed",
        "codemap proof changed",
    ] {
        assert!(
            text.contains(expected),
            "bootstrap should teach current map workflow command `{expected}`: {text}"
        );
    }
    for forbidden in [
        "codemap start",
        "codemap verify",
        "read_first",
        "ranking engine",
        "when that lens matches",
    ] {
        assert!(
            !text.contains(forbidden),
            "bootstrap must not revive removed or ambiguous wording `{forbidden}`: {text}"
        );
    }
}
#[test]
fn root_ls_is_a_bounded_domain_and_package_map() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("fixtures/example/package.json"),
        r#"{"name":"fixture-package","scripts":{"test":"vitest run"}}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture support package"]);

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    assert_eq!(json["kind"], "ls_report");
    assert_eq!(json["schema_version"], "11");
    assert_eq!(json["mode"], "directory");
    let surfaces = json["directory"].as_array().expect("directory surfaces");
    assert!(surfaces.iter().any(|surface| surface["kind"] == "domain"));
    assert!(
        surfaces
            .iter()
            .any(|surface| surface["kind"] == "package:javascript")
    );
    assert!(surfaces.iter().any(|surface| surface["kind"] == "dir"));
    assert!(
        json["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["type"] == "package_internal"
                && edge["from"] == "packages/app/"
                && edge["to"] == "packages/replay/")
    );
    assert!(
        surfaces.iter().any(|surface| surface["kind"]
            .as_str()
            .unwrap_or_default()
            .starts_with("support_package:")),
        "full JSON must serialize support-package facts hidden by readable output: {json:#}"
    );
    let horizons = json["observations"]["horizons"].as_array().expect("horizons");
    let packages = horizons.iter().find(|h| h["group"] == "packages");
    assert!(
        packages.is_some_and(|h| h["hidden"] == 0 && h["shown"] == h["count"]["observed"]),
        "full JSON must close the package projection without a hidden remainder: {json:#}"
    );
    assert_eq!(json.get("read_first"), None);
    assert_eq!(json.get("confidence"), None);

    let root_with_hidden = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--all", "--format", "json"],
    );
    assert!(
        root_with_hidden["directory"]
            .as_array()
            .expect("root include-hidden directory")
            .iter()
            .any(|surface| surface["kind"]
                .as_str()
                .unwrap_or_default()
                .starts_with("support_package:")),
        "include-hidden should reveal support packages at root on explicit request: {root_with_hidden:#}"
    );

    let fixture_scope = run_json(
        repo.path(),
        cache.path(),
        &["ls", "fixtures", "--format", "json"],
    );
    assert!(
        fixture_scope["directory"]
            .as_array()
            .expect("fixture directory")
            .iter()
            .any(|surface| surface["kind"] == "package:javascript"),
        "explicit fixture scope should still show its local packages: {fixture_scope:#}"
    );

    let tests_scope = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/replay/tests", "--format", "json"],
    );
    let test_surfaces = tests_scope["directory"].as_array().expect("test surfaces");
    assert!(
        test_surfaces
            .iter()
            .any(|surface| surface["kind"] == "e2e_test")
    );
    assert!(
        test_surfaces
            .iter()
            .any(|surface| surface["kind"] == "test_support")
    );
}


#[test]
fn file_ls_and_cone_show_symbols_edges_proof_and_boundary() {
    let (repo, cache) = fixture();
    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["anchor"]["path"], "packages/replay/src/session.ts");
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "seek" && symbol["kind"] == "function")
    );
    assert!(
        ls["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(
                |edge| edge["from"] == "packages/replay/tests/session.test.ts"
                    && edge["type"] == "tests"
            )
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/badInternal.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["boundary"]
            .as_array()
            .expect("boundary")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/badInternal.ts"
                && edge["to"] == "packages/replay/src/internal.ts"
                && edge["strength"] == "hard")
    );
}


#[test]
fn proof_directory_aggregates_member_file_proofs_without_broad_fallback() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "packages/replay/src", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(
            |surface| surface["path"] == "packages/replay/tests/session.test.ts"
                && surface["evidence"] == "test_import"
        ),
        "directory proof should include direct member-file unit proof: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .any(|surface| surface["path"] == "packages/replay/tests/e2e/seek.e2e.ts"),
        "directory proof should preserve e2e proof for files inside the directory: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "specific directory proofs should suppress broad package fallback: {proof:#}"
    );
    assert_eq!(proof.get("read_first"), None);
}


#[test]
fn proof_root_stays_bounded_without_expanding_test_galaxy() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "root proof should not enumerate every repository test: {proof:#}"
    );
    assert!(
        proof["fallback"]
            .as_array()
            .expect("fallback")
            .iter()
            .any(|command| command
                .as_str()
                .is_some_and(|value| value.ends_with(" test"))),
        "root proof should stay at broad command level instead of expanding the map: {proof:#}"
    );
    assert_eq!(proof.get("read_first"), None);
}


#[test]
fn proof_ignores_fixture_tests_for_production_anchors() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"fixture-proof-noise","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/repo.ts"),
        "export function replaySessionRepo() {\n  return true;\n}\n",
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed-monorepo/domains/replay/tests/replay-session.test.ts"),
        "test('replay session repo fixture', () => {\n  expect(true).toBe(true);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "src/repo.ts", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| !surface["path"]
                .as_str()
                .unwrap_or_default()
                .starts_with("fixtures/")),
        "fixture tests must not become proof for production anchors: {proof:#}"
    );
}

#[test]
fn file_ls_exports_async_symbols_from_symbol_map() {
    let (repo, cache) = fixture();
    let ls = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "packages/app/tests/e2e/support/mixed-layout-page.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "openMixedLayout" && symbol["exported"] == true),
        "symbol map should mark exported async functions: {ls:#}"
    );
    assert!(
        ls["anchor"]["exports"]
            .as_array()
            .expect("exports")
            .iter()
            .any(|export| export == "openMixedLayout"),
        "file export surface should include exported async functions discovered by the symbol map: {ls:#}"
    );
    assert!(
        ls["anchor"]["exports"]
            .as_array()
            .expect("exports")
            .iter()
            .all(|export| export != "page"),
        "file export surface must not promote non-exported parameters or local bindings: {ls:#}"
    );
}
