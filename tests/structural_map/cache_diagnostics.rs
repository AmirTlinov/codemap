#[test]
fn doctor_uses_warm_index_when_cached_fingerprints_match() {
    let (repo, cache) = fixture();

    let ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["kind"], "status_report");
    assert_eq!(doctor["schema_version"], "4");
    assert_eq!(doctor["cache_state"], "warm");
    assert_eq!(doctor["cache_strategy"], "warm_load");
    assert!(
        doctor["files_reused"].as_u64().unwrap_or(0) > 0,
        "warm doctor should reuse cached file facts: {doctor:#}"
    );
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
    assert!(text.contains("warm_load"));
    assert!(text.contains("Files reused"));
    assert!(text.contains("## Project Timings"));
}

#[test]
fn doctor_incrementally_rescans_only_changed_files() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { seek } from '@fixture/replay';\n\nexport const changedFrame = seek(11).frame;\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert!(
        doctor["files_reused"].as_u64().unwrap_or(0) > 0,
        "incremental doctor should reuse unchanged cached file facts: {doctor:#}"
    );
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "only the changed source file should be rescanned: {doctor:#}"
    );
}

#[test]
fn doctor_incrementally_scans_only_added_files() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join("packages/app/src/newWidget.ts"),
        "export function newWidget() {\n  return true;\n}\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert!(
        doctor["files_reused"].as_u64().unwrap_or(0) > 0,
        "incremental doctor should reuse unchanged cached file facts: {doctor:#}"
    );
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "only the added source file should be scanned: {doctor:#}"
    );

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/newWidget.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["mode"], "file");
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "newWidget"),
        "added file should be present after partial rescan: {ls:#}"
    );
}

#[test]
fn doctor_incrementally_removes_deleted_files_without_rescanning_unchanged_files() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    std::fs::remove_file(repo.path().join("packages/app/src/useReplay.ts"))
        .expect("delete cached source file");

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert!(
        doctor["files_reused"].as_u64().unwrap_or(0) > 0,
        "incremental doctor should reuse unchanged cached file facts: {doctor:#}"
    );
    assert_eq!(
        doctor["scanner"]["files_scanned"], 0,
        "deleting a file should remove cached facts without a full rescan: {doctor:#}"
    );

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/useReplay.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(
        ls["mode"], "missing",
        "deleted file must not survive from cached file facts: {ls:#}"
    );
}

#[test]
fn partial_rescan_rebuilds_reverse_imports_from_changed_files() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { internalValue } from '../../replay/src/internal';\n\nexport const changedFrame = internalValue;\n",
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "packages/replay/src/internal.ts", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/useReplay.ts"
                && edge["to"] == "packages/replay/src/internal.ts"
                && edge["type"] == "imported_by"),
        "partial rescan must rebuild reverse imports over cached + rescanned files: {cone:#}"
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_eq!(doctor["cache_strategy"], "warm_load");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 0,
        "cone should write refreshed fingerprints after partial rescan: {doctor:#}"
    );
}
