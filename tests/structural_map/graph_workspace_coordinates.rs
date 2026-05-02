#[test]
fn graph_causal_root_uses_one_directory_coordinate_for_workspace_packages() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "workspace-coordinate-fixture",
  "private": true,
  "workspaces": ["packages/*"]
}
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "dependencies": { "@fixture/lib": "workspace:*" }
}
"#,
    );
    write(
        &repo.path().join("packages/lib/package.json"),
        r#"{"name":"@fixture/lib"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "workspace coordinate fixture"]);

    let graph = run_json(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "causal", "--format", "json"],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    let nodes = graph["nodes"].as_array().expect("nodes");
    assert!(
        nodes.iter().any(|node| node == "packages/app/")
            && nodes.iter().any(|node| node == "packages/lib/"),
        "root graph should expose package directories with directory coordinates: {graph:#}"
    );
    assert!(
        nodes
            .iter()
            .all(|node| node != "packages/app" && node != "packages/lib"),
        "root graph must not duplicate package coordinates with and without slash: {graph:#}"
    );
    assert!(
        graph["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| {
                edge["from"] == "packages/app/"
                    && edge["to"] == "packages/lib/"
                    && edge["type"] == "package_internal"
                    && edge["evidence"] == "package_manifest:@fixture/lib"
            }),
        "root graph should preserve package dependency edge on the normalized coordinates: {graph:#}"
    );
}
