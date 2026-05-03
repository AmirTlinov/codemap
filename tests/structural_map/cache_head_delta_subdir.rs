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

#[test]
fn write_enabled_command_refreshes_cached_head_after_exact_content_hit() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("package.json"), r#"{"name":"head-refresh"}"#);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "initial cache fixture"]);

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let first_head = git_head_for_cache_test(repo.path());
    git(repo.path(), &["commit", "--allow-empty", "-qm", "advance head only"]);
    let second_head = git_head_for_cache_test(repo.path());
    assert_ne!(first_head, second_head, "fixture must advance HEAD");

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let fingerprints = cached_fingerprints_for_cache_test(cache.path());
    assert_eq!(
        fingerprints["git_head"].as_str(),
        Some(second_head.as_str()),
        "write-enabled exact-hit refresh should update cached git head for fast lens reuse: {fingerprints:#}"
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

fn git_head_for_cache_test(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .expect("git rev-parse should run");
    assert!(output.status.success(), "git rev-parse failed");
    String::from_utf8(output.stdout)
        .expect("git head utf8")
        .trim()
        .to_string()
}

fn cached_fingerprints_for_cache_test(cache_root: &Path) -> Value {
    let repo_cache = fs::read_dir(cache_root)
        .expect("cache root")
        .next()
        .expect("cache repo dir")
        .expect("cache repo dir");
    let text = fs::read_to_string(repo_cache.path().join("fingerprints.json"))
        .expect("fingerprints json");
    serde_json::from_str(&text).expect("fingerprints json value")
}
