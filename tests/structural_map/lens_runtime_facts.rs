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

#[test]
fn proof_map_shows_e2e_route_sensors_for_runtime_routes() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "router.get('/auth/login', loginHandler);\nexport function loginHandler() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/auth.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('auth route', async ({ page }) => {\n  await page.goto('/auth/login');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "route proof fixture"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["e2e"]
            .as_array()
            .expect("e2e proof surfaces")
            .iter()
            .any(|proof| proof["path"] == "packages/app/tests/e2e/auth.spec.ts"
                && proof["evidence"] == "e2e_visited_route"
                && proof["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("GET /auth/login"))),
        "proof-map should expose e2e route sensors from runtime route facts: {proof_map:#}"
    );
    assert!(
        proof_map["commands"]
            .as_array()
            .expect("commands")
            .iter()
            .any(|proof| proof["command"]
                .as_str()
                .is_some_and(|command| command.contains("test:e2e"))),
        "route sensor should carry package-local e2e command: {proof_map:#}"
    );
}

#[test]
fn proof_map_changed_shows_route_sensor_for_changed_route_file() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "router.get('/auth/login', loginHandler);\nexport function loginHandler() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/auth.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('auth route', async ({ page }) => {\n  await page.goto('/auth/login');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "route proof fixture"]);
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "router.get('/auth/login', loginHandler);\nexport function loginHandler() { return false; }\n",
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "--changed", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["changed"]
            .as_array()
            .expect("changed")
            .iter()
            .any(|path| path == "packages/app/src/server.ts"),
        "proof-map --changed should anchor on the changed route file: {proof_map:#}"
    );
    assert!(
        proof_map["e2e"]
            .as_array()
            .expect("e2e proof surfaces")
            .iter()
            .any(|proof| proof["path"] == "packages/app/tests/e2e/auth.spec.ts"
                && proof["evidence"] == "e2e_visited_route"),
        "proof-map --changed should carry route-visit sensors for changed route owners: {proof_map:#}"
    );
}

#[test]
fn proof_map_keeps_multiple_route_sensors_from_one_e2e_file() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "router.get('/auth/login', loginHandler);\nrouter.get('/auth/logout', logoutHandler);\nexport function loginHandler() { return true; }\nexport function logoutHandler() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/auth.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('auth routes', async ({ page }) => {\n  await page.goto('/auth/login');\n  await page.goto('/auth/logout');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "multi route proof fixture"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "packages/app/src/server.ts", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    for expected in ["GET /auth/login", "GET /auth/logout"] {
        assert!(
            proof_map["e2e"]
                .as_array()
                .expect("e2e proof surfaces")
                .iter()
                .any(|proof| proof["path"] == "packages/app/tests/e2e/auth.spec.ts"
                    && proof["reason"]
                        .as_str()
                        .is_some_and(|reason| reason.contains(expected))),
            "proof-map must not collapse distinct route sensors sharing one e2e file ({expected}): {proof_map:#}"
        );
    }
}

#[test]
fn proof_map_page_navigation_does_not_prove_post_route() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/auth-get.ts"),
        "router.get('/auth/login', getLogin);\nexport function getLogin() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/auth-post.ts"),
        "router.post('/auth/login', postLogin);\nexport function postLogin() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/auth.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('auth route', async ({ page }) => {\n  await page.goto('/auth/login');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "method route proof fixture"]);
    write(
        &repo.path().join("packages/app/src/auth-post.ts"),
        "router.post('/auth/login', postLogin);\nexport function postLogin() { return false; }\n",
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "--changed", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["e2e"]
            .as_array()
            .expect("e2e proof surfaces")
            .iter()
            .all(|proof| !proof["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("POST /auth/login")),
        "page.goto route visits must not prove POST route owners: {proof_map:#}"
    );
}

#[test]
fn proof_map_page_navigation_does_not_choose_between_duplicate_get_routes() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/auth-a.ts"),
        "router.get('/auth/login', firstLogin);\nexport function firstLogin() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/auth-b.ts"),
        "router.get('/auth/login', secondLogin);\nexport function secondLogin() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/auth.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('auth route', async ({ page }) => {\n  await page.goto('/auth/login');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "duplicate get route proof fixture"]);
    write(
        &repo.path().join("packages/app/src/auth-b.ts"),
        "router.get('/auth/login', secondLogin);\nexport function secondLogin() { return false; }\n",
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "--changed", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["e2e"]
            .as_array()
            .expect("e2e proof surfaces")
            .iter()
            .all(|proof| !proof["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("GET /auth/login")),
        "ambiguous duplicate GET route owners must fail closed instead of selecting by path: {proof_map:#}"
    );
}

#[test]
fn proof_map_page_navigation_does_not_cross_package_runtime_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "router.get('/auth/login', appLogin);\nexport function appLogin() { return true; }\n",
    );
    write(
        &repo.path().join("packages/admin/package.json"),
        r#"{
  "name": "@fixture/admin",
  "private": true,
  "scripts": { "test:e2e": "playwright test" }
}
"#,
    );
    write(
        &repo.path().join("packages/admin/tests/e2e/auth.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('admin auth route', async ({ page }) => {\n  await page.goto('/auth/login');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "cross package route proof fixture"]);
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "router.get('/auth/login', appLogin);\nexport function appLogin() { return false; }\n",
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "--changed", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["e2e"]
            .as_array()
            .expect("e2e proof surfaces")
            .iter()
            .all(|proof| proof["path"] != "packages/admin/tests/e2e/auth.spec.ts"),
        "same URL path in another package must not become proof for this route owner: {proof_map:#}"
    );
}
