#[test]
fn doctor_distinguishes_warm_artifacts_from_full_scan_strategy() {
    let (repo, cache) = fixture();

    let ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["kind"], "status_report");
    assert_eq!(doctor["schema_version"], "4");
    assert_eq!(doctor["cache_state"], "warm");
    assert_eq!(doctor["cache_strategy"], "full_scan");
    assert_eq!(doctor["files_reused"], 0);
    assert!(
        doctor["timings"]["scan_ms"].as_u64().is_some(),
        "doctor should expose scan timing without implying cache reuse: {doctor:#}"
    );
    assert!(
        doctor["timings"]["cache_artifact_ms"].as_u64().is_some(),
        "doctor should expose cache artifact timing: {doctor:#}"
    );
    assert!(
        doctor["timings"]["total_ms"].as_u64().is_some(),
        "doctor should expose total project load timing: {doctor:#}"
    );

    let markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["doctor"])
        .output()
        .expect("doctor markdown should run");
    assert!(markdown.status.success());
    let text = String::from_utf8_lossy(&markdown.stdout);
    assert!(text.contains("Cache strategy"));
    assert!(text.contains("full_scan"));
    assert!(text.contains("Files reused"));
    assert!(text.contains("## Project Timings"));
}
