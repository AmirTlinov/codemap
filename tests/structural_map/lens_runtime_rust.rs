#[test]
fn runtime_lens_extracts_rust_axum_routes_and_handlers() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/main.rs"),
        "use axum::{routing::{get, post}, Router};\n\nfn app() -> Router {\n    Router::new()\n        .route(\"/dns-query\", get(handle_get).post(handle_post))\n        .route(\"/.well-known/odohconfigs\", get(handle_odoh_configs))\n        .route(dynamic_path(), get(dynamic_handler))\n}\n\nasync fn handle_get() {}\nasync fn handle_post() {}\nasync fn handle_odoh_configs() {}\nasync fn dynamic_handler() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "rust axum route fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/main.rs", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    for (method, path, handler) in [
        ("GET", "/dns-query", "handle_get"),
        ("POST", "/dns-query", "handle_post"),
        ("GET", "/.well-known/odohconfigs", "handle_odoh_configs"),
    ] {
        assert!(
            runtime["routes"]
                .as_array()
                .expect("runtime routes")
                .iter()
                .any(|route| route["method"] == method
                    && route["path"] == path
                    && route["handler_symbol"] == handler
                    && route["evidence"] == "rust_axum_route_registration"
                    && route["locations"][0]["path"] == "packages/app/src/main.rs"),
            "runtime lens should expose Rust axum route {method} {path} -> {handler}: {runtime:#}"
        );
    }
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .all(|route| route["path"] != "/dynamic"),
        "dynamic axum paths must not become exact runtime routes: {runtime:#}"
    );
}

#[test]
fn runtime_lens_rust_axum_rejects_non_axum_route_receivers() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/main.rs"),
        "use axum::routing::get;\n\nfn app(fake: FakeRouter) {\n    fake.route(\"/not-axum\", get(handler));\n    fake\n        .route(\"/not-axum-chain\", get(handler));\n    let _ = FakeRouter::new()\n        .route(\"/fake-router\", get(handler));\n    // Router::new()\n    fake\n        .route(\"/comment-authorized\", get(handler));\n}\n\nasync fn handler() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "rust axum route negative fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src/main.rs", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["routes"]
            .as_array()
            .expect("runtime routes")
            .iter()
            .all(|route| route["path"] != "/not-axum"
                && route["path"] != "/not-axum-chain"
                && route["path"] != "/fake-router"
                && route["path"] != "/comment-authorized"),
        "non-axum Rust `.route` receivers must not become high-strength route facts: {runtime:#}"
    );
}

#[test]
fn flow_lens_stitches_rust_axum_route_to_handler_symbol() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/main.rs"),
        "use axum::{routing::{get, post}, Router};\n\nfn app() -> Router {\n    Router::new().route(\"/dns-query\", get(handle_get).post(handle_post))\n}\n\nasync fn handle_get() {}\nasync fn handle_post() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "rust axum flow fixture"]);

    let flow = run_json(
        repo.path(),
        cache.path(),
        &["flow", "POST /dns-query", "--format", "json"],
    );
    assert_schema("schemas/flow.schema.json", &flow);
    assert!(
        flow["steps"]
            .as_array()
            .expect("flow steps")
            .iter()
            .any(|step| step["kind"] == "route_handler"
                && step["anchor"] == "packages/app/src/main.rs#handle_post"
                && step["evidence"] == "route_handler_symbol"
                && step["locations"][0]["kind"] == "route_handler"),
        "flow should stitch Rust axum route to its exact handler symbol: {flow:#}"
    );
}

#[test]
fn diff_map_rust_axum_added_route_uses_runtime_facts() {
    let (repo, cache) = fixture();
    let path = repo.path().join("packages/app/src/main.rs");
    write(
        &path,
        "use axum::{routing::get, Router};\n\nfn app() -> Router {\n    Router::new()\n}\n\nasync fn handler() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "rust axum diff baseline"]);
    write(
        &path,
        "use axum::{routing::get, Router};\n\nfn app() -> Router {\n    Router::new()\n        .route(\"/new-rust-route\", get(handler))\n}\n\nasync fn handler() {}\n",
    );

    let diff = run_json(repo.path(), cache.path(), &["diff-map", "--changed", "--format", "json"]);
    assert_schema("schemas/diff-map.schema.json", &diff);
    assert!(
        diff["added_runtime_routes"]
            .as_array()
            .expect("added runtime routes")
            .iter()
            .any(|route| route["method"] == "GET"
                && route["path"] == "/new-rust-route"
                && route["file"] == "packages/app/src/main.rs"
                && route["handler_symbol"] == "handler"
                && route["evidence"] == "rust_axum_route_registration"),
        "diff-map should reuse runtime facts for added Rust axum route lines: {diff:#}"
    );
}

#[test]
fn diff_map_rust_axum_removed_route_uses_base_runtime_facts() {
    let (repo, cache) = fixture();
    let path = repo.path().join("packages/app/src/main.rs");
    let routes = (0..12)
        .map(|index| format!("        .route(\"/removed-{index}\", get(handler))"))
        .collect::<Vec<_>>()
        .join("\n");
    write(
        &path,
        &format!(
            "use axum::{{routing::get, Router}};\n\nfn app() -> Router {{\n    Router::new()\n{routes}\n}}\n\nasync fn handler() {{}}\n"
        ),
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "rust axum removed route baseline"]);
    write(
        &path,
        "use axum::{routing::get, Router};\n\nfn app() -> Router {\n    Router::new()\n}\n\nasync fn handler() {}\n",
    );

    let trace = cache.path().join("git-trace.jsonl");
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("GIT_TRACE2_EVENT", &trace)
        .args(["diff-map", "--changed", "--format", "json"])
        .output()
        .expect("diff-map should run");
    assert!(
        output.status.success(),
        "diff-map failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let diff: serde_json::Value = serde_json::from_slice(&output.stdout).expect("diff-map JSON");
    assert_schema("schemas/diff-map.schema.json", &diff);
    assert!(
        diff["removed_runtime_routes"]
            .as_array()
            .expect("removed runtime routes")
            .iter()
            .any(|route| route["method"] == "GET"
                && route["path"] == "/removed-11"
                && route["file"] == "packages/app/src/main.rs"
                && route["handler_symbol"] == "handler"
                && route["evidence"] == "rust_axum_route_registration"),
        "diff-map should reuse base runtime facts for removed Rust axum route lines: {diff:#}"
    );
    let trace = std::fs::read_to_string(trace).expect("git trace");
    let batch_reads = trace
        .lines()
        .filter(|line| line.contains(r#""cat-file","--batch""#))
        .count();
    assert!(
        batch_reads <= 2,
        "removed Rust routes should reuse one batched base-text read, not spawn git per diff line; batch_reads={batch_reads}\n{trace}"
    );
}
