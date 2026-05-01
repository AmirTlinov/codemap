#[test]
fn runtime_and_flow_ignore_detector_string_literals_as_runtime_facts() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cache = TempDir::new().expect("cache tempdir");

    let flow = run_json(
        repo,
        cache.path(),
        &[
            "flow",
            "src/map/lenses/runtime_extractors.rs",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/flow.schema.json", &flow);
    assert!(
        flow["side_effects"]
            .as_array()
            .expect("side effects")
            .is_empty(),
        "detector string literals like `fetch(` or `INSERT INTO` must not become side effects: {flow:#}"
    );
    assert!(
        flow["unknown_breaks"]
            .as_array()
            .expect("unknown breaks")
            .iter()
            .all(|unknown| unknown["kind"] != "raw_sql_literal"
                && unknown["kind"] != "env_dynamic_lookup"),
        "detector pattern strings must not become SQL/env unknowns: {flow:#}"
    );
}

#[test]
fn runtime_lens_extracts_static_env_across_languages() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/.env.example"),
        "AUTH_TOKEN=\nVITE_API=\nDENO_TOKEN=\nRUST_TOKEN=\nRUST_OTHER=\nPY_TOKEN=\nPY_OTHER=\n",
    );
    write(
        &repo.path().join("packages/app/src/env.ts"),
        "export const auth = process.env.AUTH_TOKEN;\nexport const api = import.meta.env.VITE_API;\nexport const deno = Deno.env.get('DENO_TOKEN');\n",
    );
    write(
        &repo.path().join("packages/app/src/env.rs"),
        "pub fn envs() {\n    let _ = std::env::var(\"RUST_TOKEN\");\n    let _ = env::var(\"RUST_OTHER\");\n}\n",
    );
    write(
        &repo.path().join("packages/app/src/env.py"),
        "import os\nTOKEN = os.getenv(\"PY_TOKEN\")\nOTHER = os.environ['PY_OTHER']\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env lens fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    let env = runtime["env"].as_array().expect("env surfaces");
    for expected in [
        "AUTH_TOKEN",
        "VITE_API",
        "DENO_TOKEN",
        "RUST_TOKEN",
        "RUST_OTHER",
        "PY_TOKEN",
        "PY_OTHER",
    ] {
        assert!(
            env.iter().any(|surface| surface["name"] == expected
                && surface["declaration"] == "packages/app/.env.example"),
            "runtime lens should expose static env `{expected}` with nearest declaration: {runtime:#}"
        );
    }
}

#[test]
fn delete_lens_reports_package_manifest_export_blocker() {
    let (repo, cache) = fixture();

    let delete_map = run_json(
        repo.path(),
        cache.path(),
        &["delete", "packages/replay/src/index.ts", "--format", "json"],
    );
    assert_schema("schemas/delete.schema.json", &delete_map);
    assert!(
        delete_map["package_exports"]
            .as_array()
            .expect("package exports")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/package.json"
                && edge["to"] == "packages/replay/src/index.ts"
                && edge["type"] == "package_export"),
        "delete lens must show package manifest exports as deletion blockers: {delete_map:#}"
    );
    assert!(
        delete_map["checklist"]
            .as_array()
            .expect("checklist")
            .iter()
            .any(|item| item
                .as_str()
                .is_some_and(|text| text.contains("package public exports"))),
        "delete lens checklist should point at the manifest blocker without claiming safety: {delete_map:#}"
    );
}

#[test]
fn flow_lens_starts_from_runtime_route_anchor() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "import { seek } from '@fixture/replay';\n\nrouter.get('/auth/login', loginHandler);\n\nexport function loginHandler() {\n  return seek(1).frame;\n}\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/auth.spec.ts"),
        "test('auth route', async ({ page }) => {\n  await page.goto('/auth/login');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "route flow fixture"]);

    let flow = run_json(repo.path(), cache.path(), &["flow", "/auth/login", "--format", "json"]);
    assert_schema("schemas/flow.schema.json", &flow);
    assert!(
        flow["steps"]
            .as_array()
            .expect("flow steps")
            .iter()
            .any(|step| step["kind"] == "route_anchor"
                && step["anchor"] == "GET /auth/login"
                && step["locations"][0]["path"] == "packages/app/src/server.ts"),
        "flow should start from the exact runtime route anchor: {flow:#}"
    );
    assert!(
        flow["steps"]
            .as_array()
            .expect("flow steps")
            .iter()
            .any(|step| step["anchor"] == "packages/replay/src/index.ts"
                && step["kind"] == "direct_dependency"),
        "flow should follow structural imports from the route owner file: {flow:#}"
    );
    assert!(
        flow["proof"]
            .as_array()
            .expect("flow proof")
            .iter()
            .any(|edge| edge["from"] == "packages/app/tests/e2e/auth.spec.ts"
                && edge["type"] == "runtime_reference"
                && edge["locations"][0]["kind"] == "route_visit"),
        "flow should attach e2e route-visit proof to the route anchor: {flow:#}"
    );
}

#[test]
fn flow_bare_route_anchor_fails_closed_when_route_is_ambiguous() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/auth-get.ts"),
        "router.get('/auth/login', getLogin);\nexport function getLogin() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/auth-post.ts"),
        "router.post('/auth/login', postLogin);\nexport function postLogin() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "ambiguous route fixture"]);

    let bare = run_json(repo.path(), cache.path(), &["flow", "/auth/login", "--format", "json"]);
    assert_schema("schemas/flow.schema.json", &bare);
    assert!(
        bare["steps"].as_array().expect("steps").is_empty(),
        "bare ambiguous route flow must not choose by file order: {bare:#}"
    );
    assert!(
        bare["unknown_breaks"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "route_anchor_ambiguous"),
        "ambiguous route should be a typed unknown: {bare:#}"
    );

    let get = run_json(
        repo.path(),
        cache.path(),
        &["flow", "GET /auth/login", "--format", "json"],
    );
    assert_schema("schemas/flow.schema.json", &get);
    assert!(
        get["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .any(|step| step["anchor"] == "GET /auth/login"
                && step["locations"][0]["path"] == "packages/app/src/auth-get.ts"),
        "method-specific route flow should stay exact when the bare path is ambiguous: {get:#}"
    );
}
