// Responsibility: runtime-git-index-boundary-regressions
#[cfg(unix)]
#[test]
fn runtime_gitlinks_are_one_external_placeholder_checked_out_and_deinitialized() {
    let target = TempDir::new().expect("submodule source");
    init_index_boundary_repo(target.path());
    write(
        &target.path().join("src/app.ts"),
        "app.get('/LEAKED_SUBMODULE_ROUTE', handler);\n",
    );
    write(
        &target.path().join("jobs/worker.ts"),
        "process.env.LEAKED_SUBMODULE_WORKER;\n",
    );
    write(
        &target.path().join(".github/workflows/ci.yml"),
        "jobs:\n  LEAKED_SUBMODULE_CI:\n    runs-on: ubuntu-latest\n",
    );
    git(target.path(), &["add", "."]);
    git(target.path(), &["commit", "-qm", "external tree"]);

    let repo = TempDir::new().expect("superproject");
    init_index_boundary_repo(repo.path());
    write(&repo.path().join("README.md"), "superproject\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "root"]);
    add_local_submodule(repo.path(), target.path(), "workers");
    add_local_submodule(repo.path(), target.path(), "node_modules/runtime-tree");
    git(repo.path(), &["commit", "-qam", "index external trees"]);

    let checked_cache = TempDir::new().expect("checked cache");
    let checked_cold = run_json(
        repo.path(),
        checked_cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let checked_warm = run_json(
        repo.path(),
        checked_cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(checked_cold, checked_warm, "checked-out gitlink cache drift");
    assert_gitlink_runtime_boundary(
        &checked_cold,
        &["workers", "node_modules/runtime-tree"],
    );
    let checked_readable_cache = TempDir::new().expect("checked readable cache");
    let checked_readable_cold = run_markdown(
        repo.path(),
        checked_readable_cache.path(),
        &["runtime", "."],
    );
    let checked_readable_warm = run_markdown(
        repo.path(),
        checked_readable_cache.path(),
        &["runtime", "."],
    );
    assert_lens_markdown_eq(
        &checked_readable_cold,
        &checked_readable_warm,
        "checked-out gitlink readable projection drifted warm",
    );

    git(repo.path(), &["submodule", "deinit", "-f", "--", "workers"]);
    git(
        repo.path(),
        &[
            "submodule",
            "deinit",
            "-f",
            "--",
            "node_modules/runtime-tree",
        ],
    );
    assert_git_clean(repo.path());

    let deinitialized_cache = TempDir::new().expect("deinitialized cache");
    let deinitialized_cold = run_json(
        repo.path(),
        deinitialized_cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let deinitialized_warm = run_json(
        repo.path(),
        deinitialized_cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(
        deinitialized_cold, deinitialized_warm,
        "deinitialized gitlink cache drift"
    );
    assert_eq!(
        checked_cold, deinitialized_cold,
        "gitlink meaning must not depend on checkout materialization"
    );
    assert_gitlink_runtime_boundary(
        &deinitialized_cold,
        &["workers", "node_modules/runtime-tree"],
    );
    assert_eq!(
        deinitialized_cold,
        run_runtime_without_cache(repo.path(), deinitialized_cache.path()),
        "warm gitlink report must equal a no-cache recomputation"
    );
    let deinitialized_readable_cache = TempDir::new().expect("deinitialized readable cache");
    let deinitialized_readable = run_markdown(
        repo.path(),
        deinitialized_readable_cache.path(),
        &["runtime", "."],
    );
    assert_lens_markdown_eq(
        &checked_readable_cold,
        &deinitialized_readable,
        "gitlink readable meaning must not depend on materialization",
    );
}

#[test]
fn runtime_sparse_materialization_toggles_rescan_cached_placeholders_both_ways() {
    let repo = TempDir::new().expect("sparse repo");
    let cache = TempDir::new().expect("sparse cache");
    init_index_boundary_repo(repo.path());
    write(&repo.path().join("README.md"), "sparse fixture\n");
    write(
        &repo.path().join("src/app.ts"),
        "app.get('/materialized', handler);\n",
    );
    write(
        &repo.path().join("workers/job.ts"),
        "export const worker = true;\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "jobs:\n  check:\n    runs-on: ubuntu-latest\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "sparse surfaces"]);

    let materialized = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(horizon(&materialized["observations"], "routes")["count"]["observed"], 1);
    assert_eq!(horizon(&materialized["observations"], "workers")["count"]["observed"], 1);
    assert_eq!(horizon(&materialized["observations"], "ci")["count"]["observed"], 1);
    let readable_cache = TempDir::new().expect("sparse readable cache");
    let materialized_readable = run_markdown(repo.path(), readable_cache.path(), &["runtime", "."]);

    git(repo.path(), &["sparse-checkout", "init", "--no-cone"]);
    git(repo.path(), &["sparse-checkout", "set", "README.md"]);
    assert!(!repo.path().join("src/app.ts").exists());
    assert!(!repo.path().join("workers/job.ts").exists());
    assert!(!repo.path().join(".github/workflows/ci.yml").exists());
    assert_git_clean(repo.path());

    let contracted = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_sparse_runtime_boundary(&contracted);
    assert_eq!(
        contracted,
        run_runtime_without_cache(repo.path(), cache.path()),
        "cached sparse contraction must equal no-cache truth"
    );
    let contracted_warm = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(contracted, contracted_warm, "sparse warm cache drift");
    let contracted_readable = run_markdown(repo.path(), readable_cache.path(), &["runtime", "."]);
    let contracted_readable_warm =
        run_markdown(repo.path(), readable_cache.path(), &["runtime", "."]);
    assert_lens_markdown_eq(
        &contracted_readable,
        &contracted_readable_warm,
        "sparse contracted readable projection drifted warm",
    );
    assert_lens_markdown_eq(
        &contracted_readable,
        &run_runtime_markdown_without_cache(repo.path(), readable_cache.path()),
        "cached sparse readable contraction must equal no-cache truth",
    );

    git(repo.path(), &["sparse-checkout", "disable"]);
    assert_git_clean(repo.path());
    let expanded = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(
        materialized, expanded,
        "cache must rescan unavailable tracked placeholders after expansion"
    );
    assert_eq!(
        expanded,
        run_runtime_without_cache(repo.path(), cache.path()),
        "cached sparse expansion must equal no-cache truth"
    );
    let expanded_readable = run_markdown(repo.path(), readable_cache.path(), &["runtime", "."]);
    assert_lens_markdown_eq(
        &materialized_readable,
        &expanded_readable,
        "readable cache must return to materialized truth after sparse expansion",
    );
}

fn init_index_boundary_repo(path: &Path) {
    git(path, &["init", "-q"]);
    git(path, &["config", "user.email", "a@example.com"]);
    git(path, &["config", "user.name", "a"]);
}

fn add_local_submodule(repo: &Path, target: &Path, destination: &str) {
    let status = Command::new("git")
        .args(["-c", "protocol.file.allow=always", "submodule", "add", "-q"])
        .arg(target)
        .arg(destination)
        .current_dir(repo)
        .status()
        .expect("local submodule add");
    assert!(status.success(), "submodule add failed for {destination}");
}

fn assert_git_clean(repo: &Path) {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .expect("git status");
    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "git status is not clean: {:?}", output);
}

fn run_runtime_without_cache(repo: &Path, cache: &Path) -> Value {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .env("CODEMAP_NO_CACHE", "1")
        .args(["runtime", ".", "--format", "json"])
        .output()
        .expect("no-cache runtime");
    assert!(
        output.status.success(),
        "no-cache runtime failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("runtime json")
}

fn run_runtime_markdown_without_cache(repo: &Path, cache: &Path) -> String {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .env("CODEMAP_NO_CACHE", "1")
        .args(["runtime", "."])
        .output()
        .expect("no-cache runtime markdown");
    assert!(
        output.status.success(),
        "no-cache runtime markdown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("runtime markdown")
}

fn assert_gitlink_runtime_boundary(json: &Value, paths: &[&str]) {
    let rendered = serde_json::to_string(json).expect("runtime json text");
    for leaked in [
        "LEAKED_SUBMODULE_ROUTE",
        "LEAKED_SUBMODULE_WORKER",
        "LEAKED_SUBMODULE_CI",
        "workers/src/app.ts",
        "workers/jobs/worker.ts",
    ] {
        assert!(!rendered.contains(leaked), "target fact leaked: {json:#}");
    }
    for group in ["routes", "workers", "ci", "proof"] {
        assert!(json[group].as_array().expect("runtime group").is_empty());
        assert_eq!(
            horizon(&json["observations"], group)["count"]["closure"],
            "open",
            "{group}: {json:#}"
        );
        assert_boundary_exclusions(json, group, paths, true);
    }
}

fn assert_sparse_runtime_boundary(json: &Value) {
    for (group, paths) in [
        ("routes", &["src/app.ts"][..]),
        ("workers", &["workers/job.ts"][..]),
        ("ci", &[".github/workflows/ci.yml"][..]),
        ("proof", &["src/app.ts"][..]),
    ] {
        assert!(json[group].as_array().expect("runtime group").is_empty());
        assert_eq!(
            horizon(&json["observations"], group)["count"]["closure"],
            "open",
            "{group}: {json:#}"
        );
        assert_boundary_exclusions(json, group, paths, false);
    }
}

fn assert_boundary_exclusions(json: &Value, group: &str, paths: &[&str], external: bool) {
    let certificate = runtime_group_certificate(json, group);
    let eligible = certificate["eligible_files"].as_u64().expect("eligible");
    let visited = certificate["visited_files"].as_u64().expect("visited");
    assert!(visited < eligible, "{group}: {json:#}");
    let exclusions = certificate["excluded_files_by_reason"]["incomplete_traversal"]
        .as_array()
        .expect("incomplete traversal exclusions");
    for path in paths {
        assert!(
            exclusions.iter().any(|candidate| candidate == path),
            "missing {path} exclusion for {group}: {json:#}"
        );
    }
    if external {
        assert_external_stops(json, group, paths);
    }
}

fn assert_external_stops(json: &Value, group: &str, paths: &[&str]) {
    let certificate = runtime_group_certificate(json, group);
    let stops = certificate["external_stops"]
        .as_array()
        .expect("external stops");
    for path in paths {
        assert!(
            stops.iter().any(|stop| stop["location"]["path"] == *path),
            "missing {path} external stop for {group}: {json:#}"
        );
    }
}
