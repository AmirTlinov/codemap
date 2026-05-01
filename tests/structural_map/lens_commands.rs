#[test]
fn new_lenses_return_deterministic_structural_maps() {
    let (repo, cache) = fixture();

    let contract = run_json(
        repo.path(),
        cache.path(),
        &[
            "contract",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/contract.schema.json", &contract);
    assert_eq!(contract["kind"], "contract_report");
    assert!(
        contract["consumers"]
            .as_array()
            .expect("contract consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/index.ts"),
        "contract lens should show structural consumers, not rank files: {contract:#}"
    );

    let runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert_eq!(runtime["kind"], "runtime_report");
    assert!(
        runtime["scripts"]
            .as_array()
            .expect("runtime scripts")
            .iter()
            .any(|surface| surface["kind"] == "script"),
        "runtime lens should expose package scripts as runtime surfaces: {runtime:#}"
    );

    let boundary_map = run_json(
        repo.path(),
        cache.path(),
        &["boundary-map", ".", "--format", "json"],
    );
    assert_schema("schemas/boundary-map.schema.json", &boundary_map);
    assert!(
        boundary_map["actual_cross_edges"]
            .as_array()
            .expect("cross edges")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/badInternal.ts"
                && edge["to"] == "packages/replay/src/internal.ts"),
        "boundary-map should show actual cross-package imports as a map: {boundary_map:#}"
    );

    let delete_map = run_json(
        repo.path(),
        cache.path(),
        &["delete", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/delete.schema.json", &delete_map);
    assert!(
        delete_map["direct_users"]
            .as_array()
            .expect("direct users")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/index.ts"),
        "delete lens should show blockers instead of claiming safety: {delete_map:#}"
    );
    assert_eq!(delete_map.get("safe_to_delete"), None);

    let siblings = run_json(
        repo.path(),
        cache.path(),
        &["siblings", "packages/replay/src", "--format", "json"],
    );
    assert_schema("schemas/siblings.schema.json", &siblings);
    assert!(
        !siblings["same_kind"]
            .as_array()
            .expect("same kind")
            .is_empty(),
        "siblings lens should show local structural groups: {siblings:#}"
    );

    let place = run_json(
        repo.path(),
        cache.path(),
        &[
            "place",
            "packages/replay",
            "--kind",
            "test",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/place.schema.json", &place);
    assert_eq!(place["requested_kind"], "test");
    assert!(
        place["existing_surfaces"]
            .as_array()
            .expect("existing surfaces")
            .iter()
            .any(|surface| surface["examples"]
                .as_array()
                .is_some_and(|examples| examples.iter().any(|example| example
                    == "packages/replay/tests/session.test.ts"))),
        "place lens should show existing local placement convention: {place:#}"
    );
}

#[test]
fn lens_hidden_expands_use_concrete_scope_commands() {
    let (repo, cache) = fixture();

    let siblings = run_json(
        repo.path(),
        cache.path(),
        &["siblings", ".", "--limit", "1", "--format", "json"],
    );
    assert_schema("schemas/siblings.schema.json", &siblings);
    let sibling_hidden = siblings["hidden"].as_array().expect("sibling hidden");
    assert!(
        !sibling_hidden.is_empty(),
        "fixture should force hidden sibling groups: {siblings:#}"
    );
    assert!(
        sibling_hidden.iter().all(|group| group["expand"]
            .as_str()
            .is_some_and(|expand| expand.starts_with("codemap ")
                && !expand.contains("<scope>")
                && !expand.contains("<anchor>"))),
        "siblings hidden expands must be concrete runnable commands: {siblings:#}"
    );

    let boundary_map = run_json(
        repo.path(),
        cache.path(),
        &["boundary-map", ".", "--limit", "1", "--format", "json"],
    );
    assert_schema("schemas/boundary-map.schema.json", &boundary_map);
    let boundary_hidden = boundary_map["hidden"].as_array().expect("boundary hidden");
    assert!(
        !boundary_hidden.is_empty(),
        "fixture should force hidden boundary-map groups: {boundary_map:#}"
    );
    assert!(
        boundary_hidden.iter().all(|group| group["expand"]
            .as_str()
            .is_some_and(|expand| expand == "codemap boundary-map . --include-hidden")),
        "boundary-map hidden expands must be concrete runnable commands: {boundary_map:#}"
    );

    let changed_boundary_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "boundary-map",
            ".",
            "--changed",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/boundary-map.schema.json", &changed_boundary_map);
    assert!(
        changed_boundary_map["hidden"]
            .as_array()
            .expect("changed boundary hidden")
            .iter()
            .all(|group| group["expand"].as_str().is_some_and(|expand| {
                expand == "codemap boundary-map . --changed --include-hidden"
            })),
        "boundary-map changed hidden expands must preserve the changed selector: {changed_boundary_map:#}"
    );
}

#[test]
fn edge_locations_and_typed_unknowns_are_first_class() {
    let (repo, cache) = fixture();

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    let import_edge = ls["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .find(|edge| edge["type"] == "imports")
        .expect("import edge");
    assert!(
        !import_edge["locations"]
            .as_array()
            .expect("locations")
            .is_empty(),
        "import edges must carry evidence locations: {ls:#}"
    );

    let cone = run_json(repo.path(), cache.path(), &["cone", ".", "--format", "json"]);
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "directory_aggregate"
                && unknown["effect"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("file-level edges")),
        "unknowns must be typed map facts, not free-form strings: {cone:#}"
    );
}

#[test]
fn diff_map_uses_selected_git_delta_mode() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/staged-delta.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport const stagedDelta: FrameDto = { frame: new Timeline().frameAt(1) };\n",
    );
    git(repo.path(), &["add", "packages/replay/src/staged-delta.ts"]);

    let staged = run_json(repo.path(), cache.path(), &["diff-map", "--staged", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &staged);
    assert!(
        staged["added_edges"]
            .as_array()
            .expect("staged added edges")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/staged-delta.ts"
                && edge["type"] == "added_structural_line"
                && edge["locations"][0]["kind"] == "diff_added_line:1"),
        "diff-map --staged must read the staged delta, not the unstaged working tree: {staged:#}"
    );
    assert!(
        staged["expand"]
            .as_array()
            .expect("staged expand")
            .iter()
            .any(|expand| expand == "codemap impact --staged"),
        "diff-map --staged next impact lens should preserve staged selector: {staged:#}"
    );

    git(repo.path(), &["commit", "-qm", "add staged delta fixture"]);
    let since = run_json(
        repo.path(),
        cache.path(),
        &[
            "diff-map",
            "--since",
            "HEAD~1",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/diff-map.schema.json", &since);
    assert!(
        since["added_edges"]
            .as_array()
            .expect("since added edges")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/staged-delta.ts"
                && edge["type"] == "added_structural_line"
                && edge["locations"][0]["kind"] == "diff_added_line:1"),
        "diff-map --since must read the selected base delta, not the ambient working tree: {since:#}"
    );
    assert!(
        since["expand"]
            .as_array()
            .expect("since expand")
            .iter()
            .any(|expand| expand == "codemap impact --since 'HEAD~1'"),
        "diff-map --since next impact lens should preserve since selector: {since:#}"
    );
    assert!(
        since["hidden"]
            .as_array()
            .expect("since hidden")
            .iter()
            .any(|group| group["reason"] == "added structural edges hidden by limit"
                && group["expand"] == "codemap diff-map --since 'HEAD~1' --limit 3"),
        "diff-map --since hidden expand should preserve selector and concrete limit: {since:#}"
    );
}

#[test]
fn diff_map_changed_includes_untracked_new_file_structural_lines() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/untracked-delta.ts"),
        "import { Timeline } from './timeline';\n\nexport const untrackedDelta = new Timeline();\n",
    );

    let changed = run_json(repo.path(), cache.path(), &["diff-map", "--changed", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &changed);
    assert!(
        changed["added_edges"]
            .as_array()
            .expect("changed added edges")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/untracked-delta.ts"
                && edge["type"] == "added_structural_line"
                && edge["locations"][0]["kind"] == "diff_added_line:1"),
        "diff-map --changed must synthesize structural lines for untracked files selected by git status: {changed:#}"
    );
    assert!(
        changed["added_exports"]
            .as_array()
            .expect("changed added exports")
            .iter()
            .any(|surface| surface["path"] == "packages/replay/src/untracked-delta.ts"),
        "diff-map --changed must expose export surfaces for untracked files: {changed:#}"
    );
}

#[test]
fn delete_missing_symbol_anchor_fails_closed() {
    let (repo, cache) = fixture();

    let delete_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "delete",
            "packages/replay/src/session.ts#NotARealSymbol",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/delete.schema.json", &delete_map);
    assert_eq!(
        delete_map["anchor"]["path"],
        "packages/replay/src/session.ts#NotARealSymbol"
    );
    assert!(
        delete_map["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "missing_symbol_anchor"),
        "missing symbol anchor must be an explicit unknown, not a file-level fallback: {delete_map:#}"
    );
    assert!(
        delete_map["direct_users"]
            .as_array()
            .expect("direct users")
            .is_empty(),
        "missing symbol anchor must not silently show whole-file deletion blockers: {delete_map:#}"
    );
}

#[test]
fn runtime_lens_extracts_framework_routes_unknowns_and_flow_side_effects() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "const prefix = '/v1';\nconst cached = lookup.get('/not-a-route');\nrouter.get('/auth/login', loginHandler);\nrouter.post(prefix + '/auth/logout', logoutHandler);\nconst envName = 'TOKEN';\nconst token = process.env[envName];\nconst users = db.query(`SELECT * FROM users`);\nexport async function loginHandler() {\n  await fetch('/api/session');\n  return token ?? users;\n}\nexport function logoutHandler() {\n  localStorage.setItem('session', '');\n}\n",
    );
    write(
        &repo.path().join("packages/app/src/api.py"),
        "from fastapi import FastAPI\napp = FastAPI()\n\n@app.get('/health')\ndef health():\n    return {'ok': True}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime lens fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["path"] == "/auth/login"
                && route["method"] == "GET"
                && route["evidence"] == "javascript_route_registration"),
        "runtime lens should extract static JS route registrations: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .any(|route| route["path"] == "/health"
                && route["method"] == "GET"
                && route["evidence"] == "python_route_decorator"),
        "runtime lens should extract static Python route decorators: {runtime:#}"
    );
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .all(|route| route["path"] != "/not-a-route"),
        "runtime lens must not treat arbitrary map.get string lookups as routes: {runtime:#}"
    );
    for kind in ["route_string_concat", "env_dynamic_lookup", "raw_sql_literal"] {
        assert!(
            runtime["unknowns"]
                .as_array()
                .expect("runtime unknowns")
                .iter()
                .any(|unknown| unknown["kind"] == kind),
            "runtime lens should expose typed unknown `{kind}`: {runtime:#}"
        );
    }

    let flow = run_json(
        repo.path(),
        cache.path(),
        &["flow", "packages/app/src/server.ts", "--format", "json"],
    );
    assert_schema("schemas/flow.schema.json", &flow);
    assert!(
        flow["side_effects"]
            .as_array()
            .expect("side effects")
            .iter()
            .any(|surface| surface["kind"] == "network_call"
                && surface["examples"]
                    .as_array()
                    .is_some_and(|examples| examples.iter().any(|example| example == "packages/app/src/server.ts:9"))),
        "flow lens should expose deterministic side-effect surfaces with line examples: {flow:#}"
    );
    let markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["flow", "packages/app/src/server.ts"])
        .output()
        .expect("flow markdown should run");
    assert!(markdown.status.success());
    let stdout = String::from_utf8(markdown.stdout).expect("flow markdown utf8");
    assert!(
        stdout.contains("## Side Effects") && stdout.contains("network_call"),
        "default flow output must render side-effect surfaces, not only JSON: {stdout}"
    );
}

#[test]
fn siblings_lens_exposes_route_service_test_triplets_by_convention() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/auth/auth-route.ts"),
        "router.post('/auth/login', login);\nexport function login() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/auth/auth-service.ts"),
        "export function loginService() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/auth/auth-route.test.ts"),
        "import { login } from './auth-route';\n\ntest('auth login route', () => {\n  expect(login()).toBe(true);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "siblings triplet fixture"]);

    let siblings = run_json(
        repo.path(),
        cache.path(),
        &["siblings", "packages/app/src/auth", "--format", "json"],
    );
    assert_schema("schemas/siblings.schema.json", &siblings);
    let triplet = siblings["route_service_test_triplets"]
        .as_array()
        .expect("triplets")
        .iter()
        .find(|surface| surface["kind"] == "route_service_test_triplet")
        .unwrap_or_else(|| panic!("siblings lens should expose deterministic triplet: {siblings:#}"));
    let examples = triplet["examples"].as_array().expect("triplet examples");
    for expected in [
        "packages/app/src/auth/auth-route.ts",
        "packages/app/src/auth/auth-service.ts",
        "packages/app/src/auth/auth-route.test.ts",
    ] {
        assert!(
            examples.iter().any(|example| example == expected),
            "triplet should include {expected}: {siblings:#}"
        );
    }
}
