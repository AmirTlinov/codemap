// Responsibility: runtime-root-nested-ignored-cache-identity
#[test]
fn nested_ignored_runtime_candidate_invalidates_proven_zero_root_cache() {
    let repo = TempDir::new().expect("nested ignored runtime repo");
    let cache = TempDir::new().expect("nested ignored runtime cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join(".git/info/exclude"), "*.hidden.ts\n");
    write(
        &repo.path().join("src/app.ts"),
        "export const app = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "tracked runtime root"]);

    let cold = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let cold_artifact = runtime_root_cache_json(cache.path());
    for group in ["routes", "workers", "ci"] {
        assert_eq!(
            horizon(&cold_artifact["report"]["observations"], group)["count"]["closure"],
            "closed",
            "the indexed fixture starts proven-zero for {group}: {cold_artifact:#}"
        );
    }

    let artifact_path = lens_artifact_path(cache.path(), "runtime-root.json");
    let mut primed = cold_artifact.clone();
    primed["warm_path_probe"] = serde_json::json!(true);
    fs::write(
        &artifact_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&primed).expect("nested cache probe")
        ),
    )
    .expect("prime nested ignored warm artifact");

    write(
        &repo.path().join("src/secret.hidden.ts"),
        "app.get('/secret', handler);\n",
    );
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo.path())
        .output()
        .expect("ignored nested status");
    assert!(status.status.success() && status.stdout.is_empty());

    let warm = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let repaired = runtime_root_cache_json(cache.path());
    assert_ne!(cold, warm, "the stale proven-zero output must not survive");
    assert!(
        repaired["warm_path_probe"].is_null(),
        "recursive physical identity must reject the primed artifact"
    );
    assert_eq!(
        repaired["fingerprint"], cold_artifact["fingerprint"],
        "Git/indexed project truth stays stable across the ignored candidate"
    );
    assert_ne!(
        one_runtime_report_snapshot(&repaired["report"]),
        one_runtime_report_snapshot(&cold_artifact["report"]),
        "the certificate snapshot must bind nested semantic candidate state"
    );
    for group in ["routes", "workers", "ci"] {
        let item = horizon(&repaired["report"]["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {repaired:#}");
        assert_eq!(item["count"]["closure"], "open", "{group}: {repaired:#}");
        assert!(
            runtime_group_certificate(&repaired["report"], group)["unresolved_stops"]
                .as_array()
                .expect("nested ignored stop")
                .iter()
                .any(|stop| stop["kind"] == "incomplete_traversal"),
            "the ignored candidate must be an explicit {group} boundary: {repaired:#}"
        );
    }

    let no_cache = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("CODEMAP_NO_CACHE", "1")
        .args(["runtime", "."])
        .output()
        .expect("nested ignored no-cache runtime");
    assert!(no_cache.status.success());
    assert_lens_markdown_eq(
        &warm,
        &String::from_utf8(no_cache.stdout).expect("nested ignored markdown"),
        "the repaired warm result must equal live recomputation",
    );
    assert_lens_markdown_eq(
        &warm,
        &run_markdown(repo.path(), cache.path(), &["runtime", "."]),
        "the repaired recursive identity must remain warm-stable",
    );
}
