#[test]
fn cold_large_root_graph_uses_bounded_inventory_map() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "cold-root-graph-fixture",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": {
    "test": "vitest run",
    "build": "tsc -b"
  }
}
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{"name":"@fixture/app","private":true}"#,
    );
    write(&repo.path().join(".env.example"), "DATABASE_URL=\n");
    write(&repo.path().join("README.md"), "# Cold Root Graph\n");
    write(
        &repo.path().join("apps/api/prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm test\n      - run: |\n          pnpm build\n",
    );
    for index in 0..820 {
        write(
            &repo.path().join(format!("src/bulk/file_{index:03}.ts")),
            &format!("export const value{index} = {index};\n"),
        );
    }

    let graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--lens",
            "causal",
            "--limit",
            "80",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    assert_eq!(graph["kind"], "graph_lens");
    assert_eq!(graph["domain"]["path"], ".");
    let nodes = graph["nodes"].as_array().expect("nodes");
    for expected in [
        ".",
        "package.json",
        "packages/app/",
        ".env.example",
        "README.md",
        ".github/workflows/ci.yml",
        "apps/api/prisma/",
    ] {
        assert!(
            nodes.iter().any(|node| node == expected),
            "cold root graph should include current-level inventory node `{expected}`: {graph:#}"
        );
    }
    let edges = graph["edges"].as_array().expect("edges");
    assert!(
        edges.iter().any(|edge| {
            edge["from"] == "."
                && edge["to"] == "package.json"
                && edge["type"] == "contains"
                && edge["evidence"] == "current_level_inventory_surface"
        }),
        "cold root graph should include source-backed containment edges: {graph:#}"
    );
    assert!(
        edges.iter().any(|edge| {
            edge["from"] == "package.json"
                && edge["type"] == "declares_script"
                && edge["evidence"] == "root_inventory_script"
                && edge["locations"][0]["path"] == "package.json"
        }),
        "cold root graph should keep root manifest script edges with provenance: {graph:#}"
    );
    assert!(
        edges.iter().any(|edge| {
            edge["from"] == ".github/workflows/ci.yml"
                && edge["type"] == "runs_command"
                && edge["evidence"] == "ci_run_step"
                && edge["to"] == "command:pnpm test"
        }),
        "cold root graph should keep CI run command edges with provenance: {graph:#}"
    );
    assert!(
        edges.iter().all(|edge| edge["to"] != "command:|")
            && edges.iter().any(|edge| {
                edge["from"] == ".github/workflows/ci.yml"
                    && edge["type"] == "declares_run_block"
            }),
        "cold root graph should not turn multiline CI blocks into fake commands: {graph:#}"
    );
    assert!(
        edges.iter().any(|edge| {
            edge["from"] == "package.json"
                && edge["to"] == "packages/app/"
                && edge["type"] == "workspace_member"
                && edge["evidence"] == "root_inventory_workspace_pattern"
        }),
        "cold root graph should keep workspace manifest edges: {graph:#}"
    );
    assert!(
        graph["hidden"].as_array().expect("hidden").iter().any(|hidden| {
            hidden["reason"] == "full-index source edges hidden by bounded root inventory"
        }),
        "cold root graph must disclose omitted full-index source edges: {graph:#}"
    );

    let compact = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--lens",
            "causal",
            "--limit",
            "12",
            "--format",
            "json",
        ],
    );
    let compact_nodes = compact["nodes"].as_array().expect("compact nodes");
    for expected in [
        ".",
        "package.json",
        ".github/",
        ".env.example",
        "README.md",
        "apps/api/prisma/",
    ] {
        assert!(
            compact_nodes.iter().any(|node| node == expected),
            "compact cold root graph should show representative structural surfaces before one edge source dominates: {compact:#}"
        );
    }
}
