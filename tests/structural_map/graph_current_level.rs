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
                    && edge["evidence"] == "current_level_surface"
                    && edge["strength"] == "medium"
                    && edge["locations"][0]["path"] == target
            }),
            "root causal graph should keep deterministic containment edge with evidence for {target}: {graph:#}"
        );
    }
    assert!(
        nodes
            .iter()
            .all(|node| node.as_str() != Some("src/index.ts")),
        "root causal graph should stay current-level instead of filling an empty import graph with file dumps: {graph:#}"
    );
}

#[test]
fn graph_causal_file_edges_carry_import_evidence_locations() {
    let (repo, cache) = fixture();
    let graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--path",
            "packages/replay/src/session.ts",
            "--lens",
            "causal",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    assert!(
        graph["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| {
                edge["from"] == "packages/replay/src/session.ts"
                    && edge["to"] == "packages/replay/src/timeline.ts"
                    && edge["type"] == "imports"
                    && edge["evidence"] == "resolved_import"
                    && edge["strength"] == "high"
                    && edge["locations"][0]["path"] == "packages/replay/src/session.ts"
                    && edge["locations"][0]["line_start"] == 1
                    && edge["locations"][0]["kind"] == "import_statement"
            }),
        "file causal graph import edges should point at the import statement: {graph:#}"
    );
}

#[test]
fn cache_graph_edges_preserve_evidence_locations() {
    let (repo, cache) = fixture();
    let _graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--path",
            "packages/replay/src/session.ts",
            "--lens",
            "causal",
            "--format",
            "json",
        ],
    );
    let cache_repo_dir = fs::read_dir(cache.path())
        .expect("cache root")
        .next()
        .expect("cache repo entry")
        .expect("cache repo dir")
        .path();
    let cached_graph: Value = serde_json::from_str(
        &fs::read_to_string(cache_repo_dir.join("graph.json")).expect("cache graph"),
    )
    .expect("cache graph json");
    let edges = cached_graph["edges"].as_array().expect("cache graph edges");
    assert!(
        edges.iter().all(|edge| {
            edge.get("kind").is_none()
                && edge.get("provenance").is_none()
                && edge.get("type").is_some()
                && edge.get("evidence").is_some()
                && edge.get("strength").is_some()
                && edge.get("locations").is_some()
        }),
        "cache graph should use the same structural edge vocabulary as graph lenses: {cached_graph:#}"
    );
    assert!(
        edges.iter().any(|edge| {
            edge["from"] == "packages/replay/src/session.ts"
                && edge["to"] == "packages/replay/src/timeline.ts"
                && edge["type"] == "imports"
                && edge["evidence"] == "resolved_import"
                && edge["strength"] == "high"
                && edge["locations"][0]["path"] == "packages/replay/src/session.ts"
                && edge["locations"][0]["line_start"] == 1
                && edge["locations"][0]["kind"] == "import_statement"
        }),
        "cache graph import edge should preserve exact evidence locations: {cached_graph:#}"
    );
}
