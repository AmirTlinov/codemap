// Responsibility: runtime-cache-filesystem-node-truth-regressions
#[cfg(unix)]
#[test]
fn runtime_cache_untracked_regular_file_becoming_symlink_cannot_reuse_body_facts() {
    use std::os::unix::fs::symlink;

    let repo = TempDir::new().expect("untracked node repo");
    let cache = TempDir::new().expect("untracked node cache");
    let external = TempDir::new().expect("external node body");
    init_scan_boundary_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"name":"node-transition","private":true}"#,
    );
    git(repo.path(), &["add", "package.json"]);
    git(repo.path(), &["commit", "-qm", "tracked root"]);
    let worker = repo.path().join("workers/job.ts");
    let body = "app.get('/UNTRACKED_BODY_ROUTE', handler);\n";
    write(&worker, body);

    let regular = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(horizon(&regular["observations"], "routes")["count"]["observed"], 1);
    let regular_snapshot = runtime_group_certificate(&regular, "routes")["snapshot"].clone();

    write(&external.path().join("job.ts"), body);
    fs::remove_file(&worker).expect("remove regular worker");
    symlink(external.path().join("job.ts"), &worker).expect("replace worker with symlink");

    let cached = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let fresh = run_runtime_without_cache(repo.path(), cache.path());
    assert_eq!(cached, fresh, "regular-to-symlink cache drift");
    assert!(cached["routes"].as_array().expect("routes").is_empty(), "{cached:#}");
    assert!(
        cached["workers"]
            .as_array()
            .expect("workers")
            .iter()
            .any(|worker| worker["path"] == "workers/job.ts"),
        "symlink keeps only exact path evidence: {cached:#}"
    );
    assert_incomplete_exclusion(&cached, "routes", "workers/job.ts");
    assert_ne!(
        regular_snapshot,
        runtime_group_certificate(&cached, "routes")["snapshot"],
        "node-kind transition must change the certified snapshot"
    );
}

#[test]
fn runtime_cache_non_git_same_size_same_mtime_edit_is_hashed_before_cache_reuse() {
    let repo = TempDir::new().expect("non-git same-metadata repo");
    let cache = TempDir::new().expect("non-git same-metadata cache");
    let timestamp = TempDir::new().expect("saved timestamp");
    write(
        &repo.path().join("package.json"),
        r#"{"name":"non-git-content","private":true}"#,
    );
    let source = repo.path().join("src/app.ts");
    let before = "app.get('/alpha', handler);\n";
    let after = "app.get('/bravo', handler);\n";
    assert_eq!(before.len(), after.len());
    write(&source, before);
    let initial = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let initial_snapshot = runtime_group_certificate(&initial, "routes")["snapshot"].clone();

    let saved = timestamp.path().join("app.ts");
    assert!(Command::new("cp").args(["-p", source.to_str().unwrap(), saved.to_str().unwrap()]).status().expect("save mtime").success());
    write(&source, after);
    assert!(Command::new("touch").args(["-r", saved.to_str().unwrap(), source.to_str().unwrap()]).status().expect("restore mtime").success());
    assert_eq!(fs::metadata(&source).expect("source metadata").modified().unwrap(), fs::metadata(&saved).expect("saved metadata").modified().unwrap());

    let cached = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let fresh = run_runtime_without_cache(repo.path(), cache.path());
    assert_eq!(cached, fresh, "same-metadata non-git cache drift");
    let rendered = serde_json::to_string(&cached).expect("runtime json");
    assert!(rendered.contains("/bravo") && !rendered.contains("/alpha"), "{cached:#}");
    assert_ne!(initial_snapshot, runtime_group_certificate(&cached, "routes")["snapshot"]);
}

#[test]
fn runtime_cache_assume_unchanged_tracked_body_is_rechecked_even_when_status_is_clean() {
    let repo = TempDir::new().expect("assume-unchanged repo");
    let cache = TempDir::new().expect("assume-unchanged cache");
    init_scan_boundary_repo(repo.path());
    let source = repo.path().join("src/app.ts");
    write(&source, "app.get('/alpha', handler);\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "tracked route"]);
    git(repo.path(), &["update-index", "--assume-unchanged", "src/app.ts"]);
    let initial = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let initial_snapshot = runtime_group_certificate(&initial, "routes")["snapshot"].clone();

    write(&source, "app.get('/bravo', handler);\n");
    let status = Command::new("git").args(["status", "--porcelain"]).current_dir(repo.path()).output().expect("git status");
    assert!(status.status.success() && status.stdout.is_empty(), "assume-unchanged fixture must be status-clean: {status:?}");

    let cached = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let fresh = run_runtime_without_cache(repo.path(), cache.path());
    assert_eq!(cached, fresh, "assume-unchanged cache drift");
    let rendered = serde_json::to_string(&cached).expect("runtime json");
    assert!(rendered.contains("/bravo") && !rendered.contains("/alpha"), "{cached:#}");
    assert_ne!(initial_snapshot, runtime_group_certificate(&cached, "routes")["snapshot"]);
}

#[cfg(unix)]
#[test]
fn runtime_cache_tracked_readable_body_becoming_unreadable_invalidates_stat_fast_path_and_recovers() {
    use std::os::unix::fs::PermissionsExt;

    let repo = TempDir::new().expect("tracked unreadable repo");
    let cache = TempDir::new().expect("tracked unreadable cache");
    init_scan_boundary_repo(repo.path());
    let source = repo.path().join("src/app.ts");
    write(&source, "app.get('/readable', handler);\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "readable route"]);
    let healthy = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let healthy_snapshot = runtime_group_certificate(&healthy, "routes")["snapshot"].clone();

    fs::set_permissions(&source, fs::Permissions::from_mode(0o000)).expect("make source unreadable");
    assert!(fs::File::open(&source).is_err(), "fixture must be unreadable");
    let unreadable = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let unreadable_fresh = run_runtime_without_cache(repo.path(), cache.path());
    assert_eq!(unreadable, unreadable_fresh, "unreadable tracked cache drift");
    assert!(unreadable["routes"].as_array().expect("routes").is_empty(), "{unreadable:#}");
    assert_ne!(healthy_snapshot, runtime_group_certificate(&unreadable, "routes")["snapshot"]);

    fs::set_permissions(&source, fs::Permissions::from_mode(0o644)).expect("restore source readability");
    let recovered = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(recovered, run_runtime_without_cache(repo.path(), cache.path()));
    assert_eq!(horizon(&recovered["observations"], "routes")["count"]["observed"], 1, "{recovered:#}");
}

#[cfg(unix)]
#[test]
fn runtime_cache_discovers_and_recovers_path_boundaries_outside_the_delta_set() {
    use std::os::unix::fs::PermissionsExt;

    let repo = TempDir::new().expect("tracked parent boundary repo");
    let cache = TempDir::new().expect("tracked parent boundary cache");
    init_scan_boundary_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"name":"tracked-parent-boundary","private":true}"#,
    );
    write(&repo.path().join("README.md"), "reusable root fact\n");
    write(
        &repo.path().join("src/nested/app.ts"),
        "app.get('/nested', handler);\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "nested tracked route"]);

    let healthy = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let healthy_snapshot = runtime_group_certificate(&healthy, "routes")["snapshot"].clone();
    let nested = repo.path().join("src/nested");
    fs::set_permissions(&nested, fs::Permissions::from_mode(0o000))
        .expect("deny nested traversal");
    assert!(fs::read_dir(&nested).is_err(), "fixture must deny traversal");

    let doctor = run_json(
        repo.path(),
        cache.path(),
        &["doctor", "--format", "json"],
    );
    let degraded = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let degraded_fresh = run_runtime_without_cache(repo.path(), cache.path());
    fs::set_permissions(&nested, fs::Permissions::from_mode(0o755))
        .expect("restore nested traversal");

    assert_eq!(doctor["cache_strategy"], "partial_rescan", "{doctor:#}");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 0,
        "typed boundary discovery must not trigger a full body rescan: {doctor:#}"
    );
    assert!(doctor["files_reused"].as_u64().unwrap_or_default() >= 2, "{doctor:#}");
    assert_eq!(degraded, degraded_fresh, "path-boundary cache drift");
    let certificate = runtime_group_certificate(&degraded, "routes");
    assert_eq!(certificate["eligible_files"], 2, "{degraded:#}");
    assert_incomplete_exclusion(&degraded, "routes", "src/nested");
    assert_incomplete_exclusion(&degraded, "routes", "src/nested/app.ts");
    assert_scan_wide_stop(&degraded, "src/nested");
    assert_ne!(healthy_snapshot, certificate["snapshot"]);

    let recovery_doctor = run_json(
        repo.path(),
        cache.path(),
        &["doctor", "--format", "json"],
    );
    assert_eq!(recovery_doctor["cache_strategy"], "partial_rescan", "{recovery_doctor:#}");
    assert_eq!(recovery_doctor["scanner"]["files_scanned"], 1, "{recovery_doctor:#}");
    assert!(
        recovery_doctor["files_reused"].as_u64().unwrap_or_default() >= 2,
        "{recovery_doctor:#}"
    );
    let recovered = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(recovered, run_runtime_without_cache(repo.path(), cache.path()));
    assert_eq!(
        horizon(&recovered["observations"], "routes")["count"]["observed"],
        1,
        "{recovered:#}"
    );
    assert_eq!(runtime_group_certificate(&recovered, "routes")["eligible_files"], 1);
    assert_eq!(healthy_snapshot, runtime_group_certificate(&recovered, "routes")["snapshot"]);
}
