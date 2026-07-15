fn project_cache_path(cache_root: &Path) -> std::path::PathBuf {
    std::fs::read_dir(cache_root)
        .expect("cache root")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.join("status.json").exists())
        .expect("project cache path")
}

#[test]
fn cache_status_is_schema_backed_and_explains_external_privacy() {
    let (repo, cache) = fixture();
    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);

    let report = run_json(
        repo.path(),
        cache.path(),
        &["cache", "status", "--format", "json"],
    );
    assert_schema("schemas/cache.schema.json", &report);
    assert_eq!(report["kind"], "cache_report");
    assert_eq!(report["action"], "status");
    assert_eq!(report["outside_repository"], true);
    assert_eq!(report["private_file_permissions"], true);
    assert!(report["files"].as_u64().unwrap_or_default() > 5, "{report:#}");
    assert!(
        report["privacy"]
            .as_array()
            .expect("privacy")
            .iter()
            .any(|line| line.as_str().is_some_and(|line| line.contains("file text") || line.contains("text from indexed"))),
        "cache privacy must disclose snapshot source text: {report:#}"
    );
}

#[test]
fn cache_clear_requires_confirmation_and_never_mutates_inside_repo() {
    let (repo, cache) = fixture();
    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);

    let unconfirmed = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cache", "clear"])
        .output()
        .expect("unconfirmed clear");
    assert!(!unconfirmed.status.success());
    assert!(String::from_utf8_lossy(&unconfirmed.stderr).contains("requires --yes"));
    assert!(project_cache_path(cache.path()).exists());

    let inside = repo.path().join(".cache-owner");
    let refused = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", &inside)
        .args(["cache", "gc"])
        .output()
        .expect("inside repo gc");
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("inside repository"));
    assert!(!inside.exists(), "refused cache mutation must not create repo files");

    let cleared = run_json(
        repo.path(),
        cache.path(),
        &["cache", "clear", "--yes", "--format", "json"],
    );
    assert_schema("schemas/cache.schema.json", &cleared);
    assert_eq!(cleared["exists"], false);
    assert!(cleared["removed_files"].as_u64().unwrap_or_default() > 5);
}

#[test]
fn corrupt_inventory_is_quarantined_with_visible_receipts() {
    let (repo, cache) = fixture();
    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let project_cache = project_cache_path(cache.path());
    std::fs::write(project_cache.join("inventory.json"), "{broken\n")
        .expect("corrupt inventory");

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "full_scan");
    assert!(
        doctor["cache_diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|event| event["operation"] == "quarantine"
                && event["artifact"] == "inventory.json"
                && event["outcome"] == "moved"),
        "corruption fallback must be explicit: {doctor:#}"
    );
    let report = run_json(
        repo.path(),
        cache.path(),
        &["cache", "status", "--format", "json"],
    );
    assert!(report["quarantine_receipts"].as_u64().unwrap_or_default() >= 1);
}

#[test]
fn failed_atomic_refresh_keeps_last_good_artifact_and_records_failure() {
    let (repo, cache) = fixture();
    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let project_cache = project_cache_path(cache.path());
    let inventory = project_cache.join("inventory.json");
    let before = std::fs::read(&inventory).expect("last good inventory");
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "export const writeFailureProbe = true;\n",
    );

    let failed = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("CODEMAP_TEST_CACHE_WRITE_FAILURE", "inventory.json")
        .args(["ls", "packages/app/src/useReplay.ts", "--format", "json"])
        .output()
        .expect("injected write failure");
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("injected cache write failure"));
    assert_eq!(std::fs::read(&inventory).expect("inventory survives"), before);
    assert!(
        std::fs::read_dir(&project_cache)
            .expect("cache entries")
            .filter_map(|entry| entry.ok())
            .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp-")),
        "failed atomic write must clean temporary files"
    );

    let report = run_json(
        repo.path(),
        cache.path(),
        &["cache", "status", "--format", "json"],
    );
    assert!(
        report["diagnostic_events"]
            .as_array()
            .expect("events")
            .iter()
            .any(|event| event["operation"] == "write"
                && event["artifact"] == "inventory.json"
                && event["outcome"] == "failed"),
        "write failure must remain diagnosable: {report:#}"
    );
}

#[test]
fn partial_rescan_reports_only_affected_reverse_index_targets() {
    let (repo, cache) = fixture();
    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { internalValue } from '../../replay/src/internal';\nexport const now = internalValue;\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_eq!(doctor["cache_strategy"], "partial_rescan", "{doctor:#}");
    assert_eq!(doctor["cache_work"]["per_file_facts_rebuilt"], 1);
    assert!(doctor["cache_work"]["per_file_facts_reused"].as_u64().unwrap_or_default() > 1);
    assert_eq!(doctor["cache_work"]["reverse_import_strategy"], "affected");
    let rebuilt = doctor["cache_work"]["reverse_import_targets_rebuilt"]
        .as_u64()
        .unwrap_or_default();
    assert!(rebuilt > 0 && rebuilt < doctor["files_scanned"].as_u64().unwrap_or_default());
    assert!(doctor["timings"]["cache_probe_ms"].as_u64().is_some());
    assert!(doctor["timings"]["reverse_index_ms"].as_u64().is_some());
}

#[test]
fn partial_rescan_stays_incremental_with_tracked_non_index_files() {
    let (repo, cache) = fixture();
    write(&repo.path().join("LICENSE"), "fixture license\n");
    write(
        &repo.path().join("fixtures/go-workspace/go.work.fixture"),
        "go 1.24\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "tracked scanner exclusions"]);
    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { internalValue } from '../../replay/src/internal';\nexport const now = internalValue;\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_eq!(doctor["cache_strategy"], "partial_rescan", "{doctor:#}");
    assert_eq!(doctor["cache_work"]["per_file_facts_rebuilt"], 1);
    assert!(doctor["cache_work"]["per_file_facts_reused"].as_u64().unwrap_or_default() > 1);
    assert_eq!(doctor["cache_work"]["reverse_import_strategy"], "affected");
}

#[test]
fn reverse_index_integrity_mismatch_falls_back_to_full_truth_and_quarantine() {
    let (repo, cache) = fixture();
    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let project_cache = project_cache_path(cache.path());
    let path = project_cache.join("reverse-imports.json");
    let mut cached: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("reverse index"))
            .expect("reverse index json");
    cached["imports"]["packages/replay/src/internal.ts"] =
        serde_json::json!(["forged/stale-consumer.ts"]);
    std::fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&cached).expect("serialize corrupt index")
        ),
    )
    .expect("corrupt reverse index");
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { internalValue } from '../../replay/src/internal';\nexport const changed = internalValue;\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_eq!(doctor["cache_strategy"], "partial_rescan", "{doctor:#}");
    assert_eq!(doctor["cache_work"]["reverse_import_strategy"], "full");
    assert!(
        doctor["cache_diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|event| event["operation"] == "quarantine"
                && event["artifact"] == "reverse-imports.json"
                && event["detail"] == "reverse index integrity mismatch"),
        "integrity mismatch must be visible and cannot serve stale edges: {doctor:#}"
    );
}
