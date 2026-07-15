// Responsibility: runtime-scan-completeness-boundary-regressions
#[test]
fn tracked_common_ignore_files_remain_bounded_path_facts_with_open_horizons() {
    let repo = TempDir::new().expect("ignored tracked repo");
    let cache = TempDir::new().expect("ignored tracked cache");
    init_scan_boundary_repo(repo.path());
    write(&repo.path().join("README.md"), "tracked ignored fixture\n");
    write(
        &repo.path().join("node_modules/workers/job.ts"),
        "app.get('/IGNORED_BODY_ROUTE', handler);\nprocess.env.IGNORED_BODY_ENV;\n",
    );
    write(
        &repo.path().join("vendor/.github/workflows/ci.yml"),
        "jobs:\n  IGNORED_BODY_CI:\n    runs-on: ubuntu-latest\n",
    );
    git(repo.path(), &["add", "README.md"]);
    git(
        repo.path(),
        &[
            "add",
            "-f",
            "node_modules/workers/job.ts",
            "vendor/.github/workflows/ci.yml",
        ],
    );
    git(repo.path(), &["commit", "-qm", "force tracked ignored paths"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let rendered = serde_json::to_string(&json).expect("runtime json");
    for leaked in ["IGNORED_BODY_ROUTE", "IGNORED_BODY_ENV", "IGNORED_BODY_CI"] {
        assert!(!rendered.contains(leaked), "ignored body leaked: {json:#}");
    }
    for (group, path) in [
        ("workers", "node_modules/workers/job.ts"),
        ("ci", "vendor/.github/workflows/ci.yml"),
    ] {
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["observed"], 1, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
        assert_incomplete_exclusion(&json, group, path);
    }
    for group in ["routes", "env", "proof", "unknowns"] {
        assert_eq!(
            horizon(&json["observations"], group)["count"]["closure"],
            "open",
            "{group}: {json:#}"
        );
    }
    let warm = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(json, warm, "ignored tracked path cache drift");
}

#[cfg(unix)]
#[test]
fn non_git_unreadable_tree_is_scan_wide_open_and_recovers_without_stale_cache() {
    use std::os::unix::fs::PermissionsExt;

    let repo = TempDir::new().expect("non-git traversal repo");
    let cache = TempDir::new().expect("non-git traversal cache");
    write(
        &repo.path().join("package.json"),
        r#"{"name":"traversal-boundary","private":true}"#,
    );
    write(
        &repo.path().join("blocked/src/app.ts"),
        "app.get('/recovered', handler);\n",
    );
    write(
        &repo.path().join("blocked/workers/job.ts"),
        "export const worker = true;\n",
    );
    write(
        &repo.path().join("blocked/.github/workflows/ci.yml"),
        "jobs:\n  check:\n    runs-on: ubuntu-latest\n",
    );
    let blocked = repo.path().join("blocked");
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000)).expect("block traversal");

    let cold = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let warm = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let fresh = run_runtime_without_cache(repo.path(), cache.path());
    fs::set_permissions(&blocked, fs::Permissions::from_mode(0o755)).expect("restore traversal");

    assert_eq!(cold, warm, "unreadable tree warm drift");
    assert_eq!(warm, fresh, "unreadable tree cache differs from no-cache");
    assert_scan_wide_stop(&cold, "blocked");
    for group in ["routes", "workers", "ci", "proof"] {
        assert!(cold[group].as_array().expect("runtime group").is_empty());
    }

    let recovered = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(
        recovered,
        run_runtime_without_cache(repo.path(), cache.path()),
        "recovered traversal cache differs from no-cache"
    );
    assert_eq!(horizon(&recovered["observations"], "routes")["count"]["observed"], 1);
    assert_eq!(horizon(&recovered["observations"], "workers")["count"]["observed"], 1);
    assert_eq!(horizon(&recovered["observations"], "ci")["count"]["observed"], 1);
}

#[test]
fn corrupt_git_index_is_scan_wide_open_and_cache_recovers() {
    let repo = TempDir::new().expect("corrupt index repo");
    let cache = TempDir::new().expect("corrupt index cache");
    init_scan_boundary_repo(repo.path());
    write(&repo.path().join("README.md"), "index fixture\n");
    write(&repo.path().join("src/app.ts"), "app.get('/healthy', handler);\n");
    write(&repo.path().join("workers/job.ts"), "export const worker = true;\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "jobs:\n  check:\n    runs-on: ubuntu-latest\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "indexed runtime tree"]);

    let healthy = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let healthy_snapshot = runtime_group_certificate(&healthy, "routes")["snapshot"].clone();
    let index_path = repo.path().join(".git/index");
    let saved_index = fs::read(&index_path).expect("saved git index");
    fs::write(&index_path, b"corrupt index\n").expect("corrupt index");
    fs::remove_dir_all(repo.path().join("src")).expect("hide tracked routes");
    fs::remove_dir_all(repo.path().join("workers")).expect("hide tracked workers");
    fs::remove_dir_all(repo.path().join(".github")).expect("hide tracked ci");

    let degraded = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let degraded_warm = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let degraded_fresh = run_runtime_without_cache(repo.path(), cache.path());
    let degraded_readable = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let degraded_readable_fresh = run_runtime_markdown_without_cache(repo.path(), cache.path());

    assert_eq!(degraded, degraded_warm, "corrupt-index warm drift");
    assert_eq!(degraded, degraded_fresh, "corrupt-index cache differs from no-cache");
    assert_lens_markdown_eq(
        &degraded_readable,
        &degraded_readable_fresh,
        "corrupt-index readable cache differs from no-cache",
    );
    assert_scan_wide_stop(&degraded, ".git/index");
    let degraded_status = run_json(
        repo.path(),
        cache.path(),
        &["status", "--format", "json"],
    );
    assert!(
        degraded_status["scanner"].get("inventory_boundaries").is_none(),
        "internal scan boundaries must not silently expand the stable status schema"
    );
    assert_schema("schemas/status.schema.json", &degraded_status);
    assert_ne!(
        healthy_snapshot,
        runtime_group_certificate(&degraded, "routes")["snapshot"],
        "scan completeness changes must change the certified snapshot"
    );

    fs::write(&index_path, saved_index).expect("restore git index");
    git(repo.path(), &["reset", "--hard", "-q", "HEAD"]);
    let recovered = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(healthy, recovered, "index recovery left stale cache truth");
}

fn init_scan_boundary_repo(path: &Path) {
    git(path, &["init", "-q"]);
    git(path, &["config", "user.email", "a@example.com"]);
    git(path, &["config", "user.name", "a"]);
}

fn assert_incomplete_exclusion(json: &Value, group: &str, path: &str) {
    let certificate = runtime_group_certificate(json, group);
    assert!(
        certificate["excluded_files_by_reason"]["incomplete_traversal"]
            .as_array()
            .expect("incomplete exclusions")
            .iter()
            .any(|candidate| candidate == path),
        "missing {path} exclusion for {group}: {json:#}"
    );
}

fn assert_scan_wide_stop(json: &Value, path: &str) {
    for group in [
        "entrypoints",
        "routes",
        "scripts",
        "env",
        "workers",
        "ci",
        "proof",
        "unknowns",
    ] {
        assert_eq!(
            horizon(&json["observations"], group)["count"]["closure"],
            "open",
            "{group}: {json:#}"
        );
        let certificate = runtime_group_certificate(json, group);
        assert!(
            certificate["unresolved_stops"]
                .as_array()
                .expect("unresolved stops")
                .iter()
                .any(|stop| stop["kind"] == "incomplete_traversal"
                    && stop["location"]["path"] == path),
            "missing scan-wide {path} stop for {group}: {json:#}"
        );
    }
}
