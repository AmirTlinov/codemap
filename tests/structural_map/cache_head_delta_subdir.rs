#[test]
fn head_delta_normalizes_paths_for_exact_subdir_root() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("pkg/src/a.ts"),
        "export const oldSymbol = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "subdir fixture"]);

    let root = repo.path().join("pkg");
    let _ = run_json_with_exact_root(&root, cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join("pkg/src/a.ts"),
        "export const newSymbol = true;\n",
    );
    git(repo.path(), &["add", "pkg/src/a.ts"]);
    git(repo.path(), &["commit", "-qm", "change subdir file"]);

    let doctor = run_json_with_exact_root(&root, cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_visited"], 1,
        "HEAD delta under exact subdir root should visit only changed relative path: {doctor:#}"
    );

    let ls = run_json_with_exact_root(&root, cache.path(), &["ls", "src/a.ts", "--format", "json"]);
    let symbols = ls["anchor"]["symbols"].as_array().expect("symbols");
    assert!(
        symbols.iter().any(|symbol| symbol["name"] == "newSymbol"),
        "subdir root cache must expose new committed facts: {ls:#}"
    );
    assert!(
        !symbols.iter().any(|symbol| symbol["name"] == "oldSymbol"),
        "subdir root cache must not keep stale committed facts: {ls:#}"
    );
}

fn run_json_with_exact_root(root: &Path, cache: &Path, args: &[&str]) -> Value {
    let output = codemap()
        .env("CODEMAP_CACHE_DIR", cache)
        .arg("--root")
        .arg(root)
        .args(args)
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap --root {:?} {:?} failed: {}",
        root,
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid json")
}
