#[test]
fn boundaries_check_transitive_package_dependency_graph_without_imports() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "package-boundary-fixture",
  "private": true,
  "workspaces": ["packages/*"]
}
"#,
    );
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: packages/app/src/**
      to: packages/replay/src/**
      reason: app must consume replay through the public API only
      recovery:
        - remove transitive package dependency
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "dependencies": { "@fixture/bridge": "workspace:*" }
}
"#,
    );
    write(
        &repo.path().join("packages/bridge/package.json"),
        r#"{
  "name": "@fixture/bridge",
  "dependencies": { "@fixture/replay": "workspace:*" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/package.json"),
        r#"{ "name": "@fixture/replay" }
"#,
    );
    write(
        &repo.path().join("packages/app/src/index.ts"),
        "export const app = true;\n",
    );
    write(
        &repo.path().join("packages/bridge/src/index.ts"),
        "export const bridge = true;\n",
    );
    write(
        &repo.path().join("packages/replay/src/index.ts"),
        "export const replay = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["boundaries", "--format", "json"])
        .output()
        .expect("codemap should run");
    assert!(
        !output.status.success(),
        "boundary violations should fail closed"
    );
    let boundaries: Value =
        serde_json::from_slice(&output.stdout).expect("boundary report should be json");
    assert_schema("schemas/boundaries.schema.json", &boundaries);
    assert!(
        boundaries["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(
                |finding| finding["provenance"] == "package_manifest_transitive+semantic_anchor"
                    && finding["from"] == "packages/app/package.json"
                    && finding["to"] == "packages/replay/package.json"
                    && finding["reason"]
                        .as_str()
                        .unwrap_or_default()
                        .contains("@fixture/bridge -> @fixture/replay")
            ),
        "transitive package manifest boundary must be reported without source imports: {boundaries:#}"
    );
}


#[test]
fn graph_causal_root_hides_support_packages_until_scoped() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("fixtures/example/package.json"),
        r#"{"name":"fixture-package","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("fixtures/example/src/index.ts"),
        "export const fixture = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture support package"]);

    let root_graph = run_json(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "causal", "--format", "json"],
    );
    assert_schema("schemas/graph.schema.json", &root_graph);
    assert!(
        root_graph["nodes"]
            .as_array()
            .expect("root graph nodes")
            .iter()
            .all(|node| !node.as_str().unwrap_or_default().starts_with("fixtures/")),
        "root graph should not be dominated by fixture/example package internals: {root_graph:#}"
    );

    let fixture_graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph", "--path", "fixtures", "--lens", "causal", "--format", "json",
        ],
    );
    assert!(
        fixture_graph["nodes"]
            .as_array()
            .expect("fixture graph nodes")
            .iter()
            .any(|node| node == "fixtures/example/package.json"),
        "explicit fixture graph scope should still reveal fixture package nodes: {fixture_graph:#}"
    );
}


#[test]
fn graph_proof_lens_uses_explicit_path_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/open-panel.ts"),
        "export function openPanel() {\n  return 'open';\n}\n",
    );
    write(
        &repo.path().join("packages/app/tests/open-panel.test.ts"),
        "import { openPanel } from '../src/open-panel';\n\ntest('opens the panel', () => {\n  expect(openPanel()).toBe('open');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof graph fixture"]);

    let root_graph = run_json(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "proof", "--format", "json"],
    );
    assert_schema("schemas/graph.schema.json", &root_graph);
    assert!(
        root_graph["nodes"]
            .as_array()
            .expect("root nodes")
            .is_empty(),
        "root proof graph should not expand into the whole test galaxy without an anchor: {root_graph:#}"
    );

    let scoped_graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--path",
            "packages/app/src/open-panel.ts",
            "--lens",
            "proof",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/graph.schema.json", &scoped_graph);
    assert!(
        scoped_graph["nodes"]
            .as_array()
            .expect("scoped nodes")
            .iter()
            .any(|node| node == "packages/app/tests/open-panel.test.ts"),
        "explicit path proof lens should show bounded proof nodes for that scope: {scoped_graph:#}"
    );
    assert!(
        scoped_graph["edges"]
            .as_array()
            .expect("scoped edges")
            .iter()
            .any(|edge| {
                edge["from"] == "packages/app/tests/open-panel.test.ts"
                    && edge["to"] == "packages/app/src/open-panel.ts"
                    && edge["type"] == "tests"
            }),
        "explicit path proof lens should render proof edges, not an empty graph: {scoped_graph:#}"
    );
}


#[test]
fn schema_manifest_has_no_removed_router_contracts_and_schema_command_is_side_effect_free() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest_text =
        fs::read_to_string(root.join("schemas/manifest.json")).expect("manifest should exist");
    let manifest: Value = serde_json::from_str(&manifest_text).expect("manifest json");
    assert_eq!(manifest["version"], 2);
    let schemas = manifest["schemas"].as_array().expect("schemas");
    let kinds = schemas
        .iter()
        .map(|entry| entry["kind"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    for forbidden in [
        "capsule",
        "find",
        "verify",
        "locate",
        "explain",
        "widen",
        "impact-v2",
    ] {
        assert!(!kinds.iter().any(|kind| kind == forbidden));
    }

    let actual_schema_files = fs::read_dir(root.join("schemas"))
        .expect("schemas dir")
        .map(|entry| entry.expect("schema dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("manifest.json"))
        .map(|path| format!("schemas/{}", path.file_name().unwrap().to_string_lossy()))
        .collect::<std::collections::BTreeSet<_>>();
    let manifest_schema_files = schemas
        .iter()
        .map(|entry| entry["file"].as_str().unwrap().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(manifest_schema_files, actual_schema_files);

    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    for entry in schemas {
        let kind = entry["kind"].as_str().unwrap();
        let rel = entry["file"].as_str().unwrap();
        let schema_json: Value =
            serde_json::from_str(&fs::read_to_string(root.join(rel)).expect("schema"))
                .expect("schema json");
        assert_eq!(
            schema_json["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema_json["$id"],
            format!("https://github.com/AmirTlinov/codemap/{rel}")
        );
        let output = codemap()
            .current_dir(outside.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args(["schema", kind])
            .output()
            .expect("schema command should run");
        assert!(output.status.success());
        let printed: Value = serde_json::from_slice(&output.stdout).expect("printed schema json");
        assert_eq!(printed, schema_json);
    }
    assert_eq!(fs::read_dir(cache.path()).expect("cache dir").count(), 0);
}

