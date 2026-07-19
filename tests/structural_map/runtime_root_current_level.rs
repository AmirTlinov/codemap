#[test]
fn runtime_root_readable_is_current_level_while_json_is_complete() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"fixture-cli\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\nmembers = [\"crates/worker\"]\n",
    );
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
    write(
        &repo.path().join("crates/worker/Cargo.toml"),
        "[package]\nname = \"fixture-worker\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"fixture-worker\"\npath = \"src/bin/worker.rs\"\n",
    );
    write(
        &repo.path().join("crates/worker/src/bin/worker.rs"),
        "fn main() {}\n",
    );
    write(
        &repo.path().join("fixtures/go-workspace/apps/api/main.go"),
        "package main\n\nfunc main() {}\n",
    );
    // A nested HTTP route: routes are the high-signal, low-volume runtime category
    // surfaced from nested files even at root scope.
    write(
        &repo.path().join("services/api/routes.go"),
        "package api\n\ntype Router interface {\n\tHandleFunc(path string, handler func()) Route\n}\n\ntype Route interface {\n\tMethods(methods ...string) Route\n}\n\nfunc RegisterRoutes(router Router) {\n\trouter.HandleFunc(\"/health\", health).Methods(\"GET\")\n}\n\nfunc health() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime root fixture"]);

    let runtime = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    assert!(
        runtime.contains("- `src/main.rs` [cli_entrypoint"),
        "bounded root runtime should keep package manifest entrypoints as current-level runtime surfaces: {runtime}"
    );
    assert!(
        runtime.contains("- `crates/worker` [runtime_container")
            && runtime.contains("fixture-worker -> crates/worker/src/bin/worker.rs"),
        "bounded root runtime should show package-level runtime containers without dumping recursive entrypoints: {runtime}"
    );
    assert!(
        !runtime.contains("- `crates/worker/src/bin/worker.rs` ["),
        "bounded root runtime should keep recursive package entrypoints behind the scoped runtime expand: {runtime}"
    );
    assert!(
        runtime.contains("codemap runtime crates/worker"),
        "root runtime should expose deterministic expand for runtime containers: {runtime}"
    );
    assert!(
        !runtime.contains("- `fixtures/go-workspace/apps/api/main.go` ["),
        "bounded root runtime must not recursively surface fixture entrypoints by default: {runtime}"
    );
    assert!(
        runtime.contains("recursive runtime files hidden at root scope")
            && runtime.contains("codemap runtime . --all"),
        "bounded root runtime should expose the explicit expansion command for recursive runtime files: {runtime}"
    );
    assert!(
        runtime.contains("GET /health") && runtime.contains("services/api/routes.go"),
        "root runtime should surface nested routes as a high-signal category instead of hiding them: {runtime}"
    );

    let expanded = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &expanded);
    assert!(
        expanded["hidden"]
            .as_array()
            .is_some_and(|values| values.is_empty()),
        "full JSON must not retain readable-only hidden groups: {expanded:#}"
    );
    let route_horizon = expanded["observations"]["horizons"]
        .as_array()
        .expect("runtime horizons")
        .iter()
        .find(|horizon| horizon["group"] == "routes")
        .expect("route horizon");
    assert_eq!(
        route_horizon["shown"], route_horizon["count"]["observed"],
        "full JSON route list and visibility horizon must agree: {expanded:#}"
    );
    assert!(
        expanded["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .any(|surface| surface["path"] == "fixtures/go-workspace/apps/api/main.go"),
        "JSON should imply full visibility and expose recursive runtime entrypoints: {expanded:#}"
    );
    assert_eq!(
        expanded["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .filter(|surface| surface["path"] == "src/main.rs")
            .count(),
        1,
        "manifest-derived CLI entrypoint should suppress duplicate file-convention entrypoint for the same path: {expanded:#}"
    );
}

#[test]
fn runtime_root_does_not_reaggregate_entrypoints_already_visible_under_github() {
    let (repo, cache) = runtime_candidate_fixture();
    write(
        &repo.path().join(".github/actions/tool/package.json"),
        r#"{"name":"tool","bin":{"tool:run":"bin/tool.js"}}"#,
    );
    write(
        &repo.path().join(".github/actions/tool/bin/tool.js"),
        "export function main() {}\n",
    );
    commit_runtime_candidate_fixture(&repo, "visible github package entrypoint");

    let readable = run_markdown(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--limit", "3"],
    );
    assert!(
        readable.contains("entrypoints: counted-at-least(1,")
            && readable.contains("shown=1 hidden=0")
            && readable.contains(".github/actions/tool/bin/tool.js")
            && !readable.contains(".github/actions/tool` [runtime_container"),
        "a visible manifest fact must have exactly one current-level representation: {readable}"
    );
    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let item = horizon(&json["observations"], "entrypoints");
    assert_eq!(item["count"]["observed"], 1, "{json:#}");
    assert_eq!(item["shown"], 1, "{json:#}");
    assert_horizon_certificate_resolves(&json["observations"], item);
}
