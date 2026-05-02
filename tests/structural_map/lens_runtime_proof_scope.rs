#[test]
fn proof_map_duplicate_routes_without_page_visit_do_not_emit_route_visit_unknown() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/auth-a.ts"),
        "router.get('/auth/login', firstLogin);\nexport function firstLogin() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/auth-b.ts"),
        "router.get('/auth/login', secondLogin);\nexport function secondLogin() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "duplicate get routes without e2e"]);
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
        proof_map["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .all(|unknown| unknown["kind"] != "ambiguous_route_visit_owner"),
        "duplicate routes alone are not a proof-map blind spot unless an in-scope page.goto visits them: {proof_map:#}"
    );
}

#[test]
fn runtime_and_flow_stitch_next_route_files_to_exported_method_handlers() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("app/api/login/route.ts"),
        "export async function GET() {\n  return Response.json({ ok: true });\n}\n\nexport async function POST() {\n  return Response.json({ ok: true });\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "next route handler fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "app/api/login/route.ts", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    for method in ["GET", "POST"] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("routes")
                .iter()
                .any(|route| route["method"] == method
                    && route["path"] == "/api/login"
                    && route["handler_symbol"] == method
                    && route["evidence"] == "file_route_convention"),
            "Next route.ts exported method `{method}` should be a runtime handler fact: {runtime:#}"
        );
    }

    let flow = run_json(
        repo.path(),
        cache.path(),
        &["flow", "GET /api/login", "--format", "json"],
    );
    assert_schema("schemas/flow.schema.json", &flow);
    assert!(
        flow["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .any(|step| step["kind"] == "route_handler"
                && step["anchor"] == "app/api/login/route.ts#GET"
                && step["locations"][0]["kind"] == "route_handler"),
        "flow should stitch method-specific Next runtime route to exported GET handler: {flow:#}"
    );
}
