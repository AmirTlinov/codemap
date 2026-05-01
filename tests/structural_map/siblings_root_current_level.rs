#[test]
fn root_siblings_stays_current_level_until_include_hidden() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "root-siblings-current-level",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(&repo.path().join("README.md"), "# Fixture\n");
    write(&repo.path().join("index.ts"), "export function rootEntry() { return true; }\n");
    write(&repo.path().join("src/lib.ts"), "export const rootValue = true;\n");
    write(
        &repo.path().join("tests/index.test.ts"),
        "import { rootEntry } from '../index';\n\ntest('root entry', () => {\n  expect(rootEntry()).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("fixtures/noisy/src/replay.ts"),
        "export function replay() { return true; }\n",
    );
    write(
        &repo.path().join("fixtures/noisy/tests/replay.test.ts"),
        "import { replay } from '../src/replay';\n\ntest('replay fixture', () => {\n  expect(replay()).toBe(true);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "root siblings fixture"],
    );

    let siblings = run_json(repo.path(), cache.path(), &["siblings", ".", "--format", "json"]);
    assert_schema("schemas/siblings.schema.json", &siblings);
    assert!(
        siblings["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "recursive sibling files hidden at root scope"
                && group["expand"] == "codemap siblings . --include-hidden"),
        "root siblings should make recursive files an explicit expansion: {siblings:#}"
    );
    for section in ["same_kind", "route_service_test_triplets"] {
        assert!(
            siblings[section]
                .as_array()
                .expect("surface section")
                .iter()
                .flat_map(|surface| surface["examples"].as_array().into_iter().flatten())
                .all(|path| !path.as_str().unwrap_or_default().starts_with("fixtures/")),
            "root siblings should not surface recursive fixture paths by default in {section}: {siblings:#}"
        );
    }
    assert!(
        siblings["proof_pattern"]
            .as_array()
            .expect("proof pattern")
            .iter()
            .filter_map(|proof| proof["path"].as_str())
            .all(|path| !path.starts_with("fixtures/") && !path.starts_with("tests/")),
        "root siblings should not surface recursive proof by default, even for direct root files: {siblings:#}"
    );

    let expanded = run_json(
        repo.path(),
        cache.path(),
        &[
            "siblings",
            ".",
            "--include-hidden",
            "--limit",
            "50",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/siblings.schema.json", &expanded);
    assert!(
        expanded["same_kind"]
            .as_array()
            .expect("same kind")
            .iter()
            .flat_map(|surface| surface["examples"].as_array().into_iter().flatten())
            .any(|path| path == "fixtures/noisy/tests/replay.test.ts"),
        "include-hidden should reveal the recursive fixture layer: {expanded:#}"
    );
    assert!(
        expanded["proof_pattern"]
            .as_array()
            .expect("proof pattern")
            .iter()
            .filter_map(|proof| proof["path"].as_str())
            .any(|path| path == "tests/index.test.ts"),
        "include-hidden should reveal recursive proof for direct root files: {expanded:#}"
    );
}
