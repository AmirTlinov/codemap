#[test]
fn test_kind_surfaces_exclude_bootstrap_docs_and_support_files() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "test-kind-surfaces",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(&repo.path().join("src/value.ts"), "export function value() { return true; }\n");
    write(&repo.path().join("tests/AGENTS.md"), "# Test Bootstrap\n");
    write(&repo.path().join("tests/README.md"), "# Test Notes\n");
    write(
        &repo.path().join("tests/unit.test.ts"),
        "import { value } from '../src/value';\n\ntest('unit', () => {\n  expect(value()).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("tests/support_core.ts"),
        "import { value } from '../src/value';\n\nexport const supportValue = value();\n",
    );
    write(
        &repo.path().join("tests/support/setup.rs"),
        "pub fn setup() -> bool { true }\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "test kind surface fixture"],
    );

    let place = run_json(
        repo.path(),
        cache.path(),
        &["place", "tests", "--kind", "test", "--format", "json"],
    );
    assert_schema("schemas/place.schema.json", &place);
    let examples = place["existing_surfaces"]
        .as_array()
        .expect("existing surfaces")
        .first()
        .and_then(|surface| surface["examples"].as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        examples.iter().any(|path| path == "tests/unit.test.ts"),
        "place --kind test should expose executable test files: {place:#}"
    );
    assert!(
        examples
            .iter()
            .all(|path| path != "tests/AGENTS.md"
                && path != "tests/README.md"
                && path != "tests/support_core.ts"
                && path != "tests/support/setup.rs"),
        "place --kind test should not expose bootstrap docs or support files as executable tests: {place:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "src/value.ts", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|surface| surface["path"] == "tests/unit.test.ts"),
        "proof should include executable tests that import the anchor: {proof:#}"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "tests/support_core.ts"),
        "support-like source files must not become runnable proof sensors just because they import the anchor: {proof:#}"
    );

    let support = run_json(
        repo.path(),
        cache.path(),
        &["cone", "tests/support_core.ts", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &support);
    assert!(
        support["anchor"]["roles"]
            .as_array()
            .expect("support roles")
            .iter()
            .all(|role| role != "test"),
        "support-like source files should not keep the executable test role: {support:#}"
    );
    assert!(
        support["anchor"]["roles"]
            .as_array()
            .expect("support roles")
            .iter()
            .any(|role| role == "test_support"),
        "support-like source files should be visible as test_support: {support:#}"
    );

    let ls = run_json(repo.path(), cache.path(), &["ls", "tests", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        ls["directory"]
            .as_array()
            .expect("directory surfaces")
            .iter()
            .filter(|surface| surface["kind"] == "test")
            .flat_map(|surface| surface["examples"].as_array().into_iter().flatten())
            .all(|path| path != "tests/AGENTS.md" && path != "tests/README.md"),
        "ls tests should not group markdown bootstrap/docs under test kind: {ls:#}"
    );
}
