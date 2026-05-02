#[test]
fn runtime_root_scope_is_current_level_until_include_hidden() {
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
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime root fixture"]);

    let runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
    assert_schema("schemas/runtime.schema.json", &runtime);
    assert!(
        runtime["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .any(|surface| surface["kind"] == "cli_entrypoint"
                && surface["path"] == "src/main.rs"),
        "root runtime should keep package manifest entrypoints as current-level runtime surfaces: {runtime:#}"
    );
    assert!(
        runtime["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .any(|surface| surface["kind"] == "runtime_container"
                && surface["path"] == "crates/worker"
                && surface["count"] == 1
                && surface["examples"]
                    .as_array()
                    .is_some_and(|examples| examples.iter().any(|example| example
                        .as_str()
                        .is_some_and(|value| value.contains("fixture-worker -> crates/worker/src/bin/worker.rs"))))),
        "root runtime should show package-level runtime containers without dumping recursive entrypoints: {runtime:#}"
    );
    assert!(
        runtime["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .all(|surface| surface["path"] != "crates/worker/src/bin/worker.rs"),
        "root runtime should keep recursive package entrypoints behind the scoped runtime expand: {runtime:#}"
    );
    assert!(
        runtime["expand"]
            .as_array()
            .expect("expand")
            .iter()
            .any(|command| command == "codemap runtime crates/worker"),
        "root runtime should expose deterministic expand for runtime containers: {runtime:#}"
    );
    assert!(
        runtime["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .all(|surface| !surface["path"]
                .as_str()
                .unwrap_or_default()
                .starts_with("fixtures/")),
        "root runtime must not recursively surface fixture entrypoints by default: {runtime:#}"
    );
    assert!(
        runtime["hidden"]
            .as_array()
            .expect("hidden groups")
            .iter()
            .any(|group| group["reason"] == "recursive runtime files hidden at root scope"
                && group["expand"] == "codemap runtime . --include-hidden"),
        "root runtime should expose the explicit expansion command for recursive runtime files: {runtime:#}"
    );

    let expanded = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--include-hidden", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &expanded);
    assert!(
        expanded["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .any(|surface| surface["path"] == "fixtures/go-workspace/apps/api/main.go"),
        "include-hidden should make recursive runtime entrypoints visible on demand: {expanded:#}"
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
