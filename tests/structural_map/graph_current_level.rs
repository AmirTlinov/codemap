#[test]
fn graph_causal_root_keeps_current_level_relationships_without_import_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "root-graph-empty-import-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/index.ts"),
        "export const value = true;\n",
    );
    write(
        &repo.path().join("tests/index.test.ts"),
        "test('value', () => {\n  expect(true).toBe(true);\n});\n",
    );
    write(&repo.path().join("schemas/capsule.schema.json"), "{}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "root graph relationship fixture"]);

    let graph = run_json(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "causal", "--limit", "20", "--format", "json"],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    let nodes = graph["nodes"].as_array().expect("nodes");
    assert!(
        nodes.iter().any(|node| node == ".")
            && nodes.iter().any(|node| node == "package.json")
            && nodes.iter().any(|node| node == "src/")
            && nodes.iter().any(|node| node == "tests/")
            && nodes.iter().any(|node| node == "schemas/"),
        "root causal graph should include a current-level scope and visible surfaces: {graph:#}"
    );
    let edges = graph["edges"].as_array().expect("edges");
    for target in ["package.json", "src/", "tests/", "schemas/"] {
        assert!(
            edges.iter().any(|edge| {
                edge["from"] == "." && edge["to"] == target && edge["type"] == "contains"
            }),
            "root causal graph should keep deterministic containment edge for {target}: {graph:#}"
        );
    }
    assert!(
        nodes
            .iter()
            .all(|node| node.as_str() != Some("src/index.ts")),
        "root causal graph should stay current-level instead of filling an empty import graph with file dumps: {graph:#}"
    );
}
