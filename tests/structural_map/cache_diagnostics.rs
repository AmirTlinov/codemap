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
fn doctor_invalidates_warm_index_when_fingerprint_format_changes() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let fingerprint_path = std::fs::read_dir(cache.path())
        .expect("cache dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("fingerprints.json"))
        .find(|path| path.exists())
        .expect("fingerprints json path");
    let mut fingerprints: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&fingerprint_path).expect("fingerprints json"),
    )
    .expect("fingerprints value");
    fingerprints["format_version"] = serde_json::json!(4);
    std::fs::write(
        &fingerprint_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&fingerprints).expect("fingerprints serialize")
        ),
    )
    .expect("write downgraded fingerprints");

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(
        doctor["cache_strategy"], "full_scan",
        "stale parser/index cache format must not be reused after extractor changes: {doctor:#}"
    );
    assert_eq!(
        doctor["files_reused"], 0,
        "format mismatch should prevent cached FileInfo reuse: {doctor:#}"
    );
    assert!(
        doctor["scanner"]["files_scanned"].as_u64().unwrap_or_default() > 1,
        "format mismatch should force a real scan instead of status-only reuse: {doctor:#}"
    );
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
fn doctor_uses_git_status_mismatch_set_for_committed_repos() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { seek } from '@fixture/replay';\n\nexport const changedFrame = seek(21).frame;\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert!(
        doctor["files_reused"].as_u64().unwrap_or(0) > 0,
        "git status fast path should reuse cached file facts: {doctor:#}"
    );
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "git status fast path should scan only the mismatched tracked file: {doctor:#}"
    );
}

#[test]
fn doctor_uses_git_status_mismatch_set_for_untracked_paths_with_spaces() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join("packages/app/src/local draft.ts"),
        "export const localDraft = 'untracked spaced path';\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "NUL status parsing should scan only the new untracked file with a space: {doctor:#}"
    );

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/local draft.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(
        ls["mode"], "file",
        "untracked path with spaces must not be hidden by quoted porcelain parsing: {ls:#}"
    );
}

#[test]
fn doctor_uses_cached_untracked_probe_for_deleted_untracked_files() {
    let (repo, cache) = fixture();
    let untracked = repo.path().join("packages/app/src/localDraft.ts");
    write(
        &untracked,
        "export const localDraft = 'untracked cache candidate';\n",
    );

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    std::fs::remove_file(&untracked).expect("delete cached untracked source file");

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 0,
        "deleted cached untracked files should be removed without a full candidate scan: {doctor:#}"
    );

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/localDraft.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["mode"], "missing");
}

#[test]
fn doctor_removes_cached_untracked_file_that_becomes_git_ignored() {
    let (repo, cache) = fixture();
    let untracked = repo.path().join("packages/app/src/localDraft.ts");
    write(
        &untracked,
        "export const localDraft = 'untracked cache candidate';\n",
    );

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(&repo.path().join(".gitignore"), "packages/app/src/localDraft.ts\n");

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 0,
        "cached untracked files that become ignored should disappear without parser rescan: {doctor:#}"
    );

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/localDraft.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(
        ls["mode"], "missing",
        "ignored local file must not survive from cached untracked facts: {ls:#}"
    );
}

#[test]
fn doctor_uses_cached_untracked_probe_for_modified_untracked_files() {
    let (repo, cache) = fixture();
    let untracked = repo.path().join("packages/app/src/localDraft.ts");
    write(
        &untracked,
        "export const localDraft = 'first untracked body';\n",
    );

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &untracked,
        "export function localDraft() {\n  return 'changed untracked body';\n}\n",
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "modified cached untracked files should be the only parser rescan: {doctor:#}"
    );

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/localDraft.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "localDraft"),
        "changed cached untracked file should be rescanned with new symbol facts: {ls:#}"
    );
}

#[test]
fn doctor_removes_cached_source_renamed_into_ignored_directory() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/live.ts"),
        "export const live = true;\n",
    );
    git(repo.path(), &["add", "packages/app/src/live.ts"]);
    git(repo.path(), &["commit", "-qm", "add live source"]);

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    std::fs::create_dir_all(repo.path().join("dist")).expect("create ignored dist");
    git(
        repo.path(),
        &["mv", "packages/app/src/live.ts", "dist/live.ts"],
    );

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 0,
        "rename into ignored dir should remove old cached facts without scanning ignored target: {doctor:#}"
    );

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/live.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(
        ls["mode"], "missing",
        "old source path must not survive after rename into ignored dir: {ls:#}"
    );
}

#[test]
fn doctor_skips_rescan_when_only_metadata_changed_for_same_content() {
    let (repo, cache) = fixture();

    let body = "import { seek } from '@fixture/replay';\n\nexport const frame = seek(1).frame;\n";
    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    std::thread::sleep(std::time::Duration::from_millis(5));
    write(&repo.path().join("packages/app/src/useReplay.ts"), body);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(
        doctor["cache_strategy"], "warm_load",
        "same-content rewrites should keep cached file facts warm: {doctor:#}"
    );
    assert_eq!(
        doctor["scanner"]["files_scanned"], 0,
        "same-content metadata changes should not trigger parser rescan: {doctor:#}"
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
