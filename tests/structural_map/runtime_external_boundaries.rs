// Responsibility: runtime-indexed-boundary-accounting
#[cfg(unix)]
#[test]
fn runtime_symlink_directories_are_external_stops_not_path_facts() {
    use std::os::unix::fs::symlink;

    let (repo, cache) = runtime_candidate_fixture();
    let external = TempDir::new().expect("external runtime tree");
    write(
        &external.path().join("workflows/ci.yml"),
        "jobs:\n  leaked:\n    runs-on: ubuntu-latest\n",
    );
    write(
        &external.path().join("workers/job.ts"),
        "process.env.EXTERNAL_WORKER_SECRET;\n",
    );
    write(
        &external.path().join("src/app.ts"),
        "app.get('/external', handler);\nprocess.env.EXTERNAL_SOURCE_SECRET;\n",
    );
    fs::create_dir_all(repo.path().join(".github")).expect("github directory");
    symlink(
        external.path().join("workflows"),
        repo.path().join(".github/workflows"),
    )
    .expect("workflow directory symlink");
    symlink(external.path().join("workers"), repo.path().join("workers"))
        .expect("worker directory symlink");
    symlink(external.path().join("src"), repo.path().join("src"))
        .expect("source directory symlink");
    commit_runtime_candidate_fixture(&repo, "external runtime directories");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    for group in ["routes", "workers", "ci"] {
        assert!(
            json[group].as_array().expect("runtime facts").is_empty(),
            "{group}: {json:#}"
        );
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
        assert!(
            !runtime_group_certificate(&json, group)["external_stops"]
                .as_array()
                .expect("external stops")
                .is_empty(),
            "{group} must retain the non-followed directory boundary: {json:#}"
        );
    }
    assert_runtime_external_exclusion(&json, "workers", "workers");
    assert_runtime_external_exclusion(&json, "ci", ".github/workflows");
    assert!(
        json["env"].as_array().expect("env facts").is_empty(),
        "{json:#}"
    );
}

#[cfg(unix)]
#[test]
fn runtime_dangling_tree_symlinks_remain_conservative_external_stops() {
    use std::os::unix::fs::symlink;

    let (repo, cache) = runtime_candidate_fixture();
    fs::create_dir_all(repo.path().join(".github")).expect("github directory");
    symlink(
        repo.path().join("missing-workflows"),
        repo.path().join(".github/workflows"),
    )
    .expect("dangling workflow tree symlink");
    symlink(
        repo.path().join("missing-workers"),
        repo.path().join("workers"),
    )
    .expect("dangling worker tree symlink");
    commit_runtime_candidate_fixture(&repo, "dangling runtime trees");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    for group in ["workers", "ci"] {
        assert!(
            json[group].as_array().expect("runtime facts").is_empty(),
            "{group}: {json:#}"
        );
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
        assert!(
            !runtime_group_certificate(&json, group)["external_stops"]
                .as_array()
                .expect("external stops")
                .is_empty(),
            "unknown symlink target kind must remain an external boundary: {json:#}"
        );
    }
    assert_runtime_external_exclusion(&json, "workers", "workers");
    assert_runtime_external_exclusion(&json, "ci", ".github/workflows");
}

#[cfg(unix)]
#[test]
fn runtime_symlink_files_keep_path_facts_without_reading_external_body() {
    use std::os::unix::fs::symlink;

    let (repo, cache) = runtime_candidate_fixture();
    let external = TempDir::new().expect("external runtime files");
    write(
        &external.path().join("ci.yml"),
        "jobs:\n  leaked:\n    env:\n      EXTERNAL_CI_SECRET: leaked\n",
    );
    write(
        &external.path().join("job.ts"),
        "process.env.EXTERNAL_WORKER_SECRET;\napp.get('/leaked', handler);\n",
    );
    fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflow directory");
    fs::create_dir_all(repo.path().join("workers")).expect("worker directory");
    symlink(
        external.path().join("ci.yml"),
        repo.path().join(".github/workflows/ci.yml"),
    )
    .expect("workflow file symlink");
    symlink(
        external.path().join("job.ts"),
        repo.path().join("workers/job.ts"),
    )
    .expect("worker file symlink");
    commit_runtime_candidate_fixture(&repo, "external runtime files");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert!(
        json["ci"]
            .as_array()
            .expect("ci facts")
            .iter()
            .any(|surface| {
                surface["path"] == ".github/workflows/ci.yml"
                    && surface["evidence"] == "role:build_ci"
            }),
        "the symlink file keeps exact CI path evidence: {json:#}"
    );
    assert!(
        json["workers"]
            .as_array()
            .expect("worker facts")
            .iter()
            .any(|surface| surface["path"] == "workers/job.ts"),
        "the symlink file keeps exact worker path evidence: {json:#}"
    );
    assert!(
        json["env"].as_array().expect("env facts").is_empty(),
        "{json:#}"
    );
    assert!(
        json["routes"].as_array().expect("route facts").is_empty(),
        "{json:#}"
    );
    for (group, path) in [
        ("workers", "workers/job.ts"),
        ("ci", ".github/workflows/ci.yml"),
    ] {
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["observed"], 1, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
        assert_runtime_external_exclusion(&json, group, path);
    }
    for group in ["routes", "env"] {
        assert_eq!(
            horizon(&json["observations"], group)["count"]["closure"],
            "open",
            "content-derived {group} remains open without reading symlink body: {json:#}"
        );
    }
}

#[cfg(unix)]
#[test]
fn runtime_root_cache_is_independent_of_external_symlink_target_kind() {
    use std::os::unix::fs::symlink;

    let (repo, cache) = runtime_candidate_fixture();
    let external = TempDir::new().expect("external target owner");
    let target = external.path().join("ci-target");
    write(&target, "jobs:\n  check:\n    runs-on: ubuntu-latest\n");
    fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflow directory");
    symlink(&target, repo.path().join(".github/workflows/ci.yml")).expect("workflow file symlink");
    commit_runtime_candidate_fixture(&repo, "cached symlink boundary");

    let cold = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let artifact_path = lens_artifact_path(cache.path(), "runtime-root.json");
    let mut primed_artifact: Value = serde_json::from_str(
        &fs::read_to_string(&artifact_path).expect("primed runtime root cache"),
    )
    .expect("runtime root cache json");
    primed_artifact["warm_path_probe"] = serde_json::json!(true);
    fs::write(
        &artifact_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&primed_artifact).expect("runtime cache probe")
        ),
    )
    .expect("write warm-path cache probe");
    fs::remove_file(&target).expect("replace external file target");
    fs::create_dir(&target).expect("external target becomes a directory");

    let warm = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    assert_eq!(
        runtime_root_cache_json(cache.path())["warm_path_probe"],
        true,
        "the target-kind mutation must exercise the unchanged warm artifact"
    );
    let no_cache = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("CODEMAP_NO_CACHE", "1")
        .args(["runtime", "."])
        .output()
        .expect("no-cache runtime recomputation");
    assert!(
        no_cache.status.success(),
        "no-cache runtime failed: {}",
        String::from_utf8_lossy(&no_cache.stderr)
    );
    let recomputed = String::from_utf8(no_cache.stdout).expect("runtime markdown");
    assert_lens_markdown_eq(
        &cold,
        &warm,
        "external target type cannot change cached symlink-boundary semantics",
    );
    assert_lens_markdown_eq(
        &warm,
        &recomputed,
        "warm runtime must equal a no-cache recomputation after file-to-directory target drift",
    );

    let artifact = runtime_root_cache_json(cache.path());
    let report = &artifact["report"];
    let ci = horizon(&report["observations"], "ci");
    assert_eq!(ci["count"]["observed"], 1, "{artifact:#}");
    assert_eq!(ci["count"]["closure"], "open", "{artifact:#}");
    assert_runtime_external_exclusion(report, "ci", ".github/workflows/ci.yml");
}

#[cfg(unix)]
fn assert_runtime_external_exclusion(json: &Value, group: &str, path: &str) {
    let certificate = runtime_group_certificate(json, group);
    let eligible = certificate["eligible_files"]
        .as_u64()
        .expect("eligible files");
    let visited = certificate["visited_files"]
        .as_u64()
        .expect("visited files");
    assert!(visited < eligible, "{group}: {json:#}");
    assert!(
        certificate["excluded_files_by_reason"]["incomplete_traversal"]
            .as_array()
            .expect("external traversal exclusions")
            .iter()
            .any(|candidate| candidate == path),
        "{path} must be an explicit {group} traversal exclusion: {json:#}"
    );
    assert!(
        !certificate["external_stops"]
            .as_array()
            .expect("external stops")
            .is_empty(),
        "{group} must retain a wrapper external stop: {json:#}"
    );
}

#[test]
fn runtime_extensionless_worker_path_is_an_indexed_fact() {
    let (repo, cache) = runtime_candidate_fixture();
    write(
        &repo.path().join("workers/job"),
        "#!/bin/sh\nprintf 'work\\n'\n",
    );
    commit_runtime_candidate_fixture(&repo, "extensionless worker path");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert!(
        json["workers"]
            .as_array()
            .expect("worker facts")
            .iter()
            .any(|surface| surface["path"] == "workers/job"),
        "extensionless path convention must survive scan filtering: {json:#}"
    );
    let item = horizon(&json["observations"], "workers");
    assert_eq!(item["count"]["observed"], 1, "{json:#}");
    assert_eq!(item["count"]["closure"], "closed", "{json:#}");
}

#[test]
fn runtime_group_oversized_manifest_remains_an_explicit_unread_candidate() {
    let (repo, cache) = runtime_candidate_fixture();
    let body = format!(
        "{{\"bin\":{{\"tool\":\"missing.js\"}},\"padding\":\"{}\"}}\n",
        "x".repeat(910_000)
    );
    write(&repo.path().join("package.json"), &body);
    commit_runtime_candidate_fixture(&repo, "oversized manifest boundary");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    for group in ["entrypoints", "scripts"] {
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
        assert_unsupported_file(item, "package.json", &json);
    }
}

#[test]
fn runtime_group_binary_lock_path_still_selects_the_indexed_package_manager() {
    let (repo, cache) = runtime_candidate_fixture();
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"node test.js"}}"#,
    );
    fs::write(repo.path().join("bun.lockb"), [0xff, 0x00, 0x01]).expect("binary bun lock fixture");
    commit_runtime_candidate_fixture(&repo, "binary package manager lock");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_eq!(
        json["scripts"][0]["examples"][0], "test: bun test",
        "package-manager identity is indexed path evidence, not readable lock content: {json:#}"
    );
}
