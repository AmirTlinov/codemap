#[test]
fn runtime_root_scope_is_current_level_until_include_hidden() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"fixture-cli\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
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
