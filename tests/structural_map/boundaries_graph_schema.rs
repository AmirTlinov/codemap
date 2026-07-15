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
        &repo.path().join(".codemap.yml"),
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
fn graph_causal_root_uses_ls_level_directory_map() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "single-package-map-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/auth/login.ts"),
        "import { session } from '../shared/session';\n\nexport function login() {\n  return session();\n}\n",
    );
    write(
        &repo.path().join("src/shared/session.ts"),
        "export function session() {\n  return 'ok';\n}\n",
    );
    write(
        &repo.path().join("tests/auth-login.test.ts"),
        "import { login } from '../src/auth/login';\n\ntest('login', () => {\n  expect(login()).toBe('ok');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "single package map fixture"]);

    let graph = run_json(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "causal", "--limit", "20", "--format", "json"],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    let nodes = graph["nodes"].as_array().expect("nodes");
    assert!(
        nodes.iter().any(|node| node == "package.json")
            && nodes.iter().any(|node| node == "src/")
            && nodes.iter().any(|node| node == "tests/"),
        "root causal graph should show ls-level package and top-level folders: {graph:#}"
    );
    assert!(
        nodes
            .iter()
            .all(|node| node.as_str() != Some("src/auth/login.ts")),
        "root causal graph should not dump nested files when a directory surface is enough: {graph:#}"
    );
    assert!(
        graph["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| {
                edge["from"] == "tests/" && edge["to"] == "src/" && edge["type"] == "outgoing_import"
            }),
        "root causal graph should preserve aggregate directory edges: {graph:#}"
    );
}

#[test]
fn graph_causal_directory_scope_uses_current_level_not_recursive_file_dump() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"scoped-directory-map-fixture","private":true}"#,
    );
    write(
        &repo.path().join("src/features/login.ts"),
        "import { session } from '../shared/session';\n\nexport function login() {\n  return session();\n}\n",
    );
    write(
        &repo.path().join("src/shared/session.ts"),
        "export function session() {\n  return 'ok';\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "scoped map fixture"]);

    let graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--path",
            "src",
            "--lens",
            "causal",
            "--limit",
            "20",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/graph.schema.json", &graph);
    let nodes = graph["nodes"].as_array().expect("nodes");
    assert!(
        nodes.iter().any(|node| node == "src/features/")
            && nodes.iter().any(|node| node == "src/shared/"),
        "scoped causal graph should show current-level child surfaces: {graph:#}"
    );
    assert!(
        nodes
            .iter()
            .all(|node| node.as_str() != Some("src/features/login.ts")),
        "scoped directory graph should not recursively dump nested files by default: {graph:#}"
    );
    assert!(
        graph["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| {
                edge["from"] == "src/features/"
                    && edge["to"] == "src/shared/"
                    && edge["type"] == "outgoing_import"
            }),
        "scoped causal graph should keep aggregate import edges between current-level surfaces: {graph:#}"
    );
}

#[test]
fn directory_ls_points_to_current_level_graph_before_cone() {
    let (repo, cache) = fixture();

    let root = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &root);
    assert_eq!(
        root["next"]
            .as_array()
            .expect("root next")
            .first()
            .and_then(|command| command.as_str()),
        Some("codemap graph --lens causal"),
        "root ls should send agents to the current-level map before a deeper cone: {root:#}"
    );

    let scoped = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &scoped);
    assert_eq!(
        scoped["next"]
            .as_array()
            .expect("scoped next")
            .first()
            .and_then(|command| command.as_str()),
        Some("codemap graph --path packages/app --lens causal"),
        "directory ls should keep graph scope exact before suggesting cone: {scoped:#}"
    );
    assert!(
        scoped["next"]
            .as_array()
            .expect("scoped next")
            .iter()
            .any(|command| command == "codemap cone packages/app --depth 1"),
        "cone remains available after the current-level graph: {scoped:#}"
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
    assert_eq!(manifest["version"], 17);
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
    for required in [
        "doctor",
        "status",
        "files",
        "ls",
        "cone",
        "graph",
        "runtime",
        "contract",
        "flow",
        "boundary-map",
        "siblings",
        "place",
        "delete",
        "changed",
        "diff-map",
        "impact",
        "proof-map",
        "proof",
    ] {
        assert!(
            kinds.iter().any(|kind| kind == required),
            "schema manifest should list public report kind `{required}`"
        );
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

#[test]
fn doctor_json_has_an_explicit_schema_alias() {
    let (repo, cache) = fixture();
    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["kind"], "status_report");

    let doctor_schema = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["schema", "doctor"])
        .output()
        .expect("doctor schema command should run");
    let status_schema = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["schema", "status"])
        .output()
        .expect("status schema command should run");
    assert!(doctor_schema.status.success());
    assert!(status_schema.status.success());
    assert_eq!(
        doctor_schema.stdout, status_schema.stdout,
        "doctor uses the status_report JSON shape, but should be discoverable without guessing"
    );
}
