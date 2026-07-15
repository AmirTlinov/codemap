#[test]
fn partial_rescan_rechecks_paths_that_leave_git_status_after_dirty_cache() {
    let (repo, cache) = fixture();
    let path = repo.path().join("packages/app/src/useReplay.ts");
    let original =
        "import { seek } from '@fixture/replay';\n\nexport const frame = seek(1).frame;\n";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &path,
        "import { seek } from '@fixture/replay';\n\nexport const dirtyFrame = seek(31).frame;\n",
    );
    let dirty = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/useReplay.ts", "--format", "json"],
    );
    assert!(
        dirty["anchor"]["symbols"]
            .as_array()
            .expect("dirty symbols")
            .iter()
            .any(|symbol| symbol["name"] == "dirtyFrame"),
        "dirty cache precondition should include changed symbol: {dirty:#}"
    );

    write(&path, original);
    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "a path that left git status after a dirty cache must be rechecked, not trusted as warm: {doctor:#}"
    );

    let clean = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/useReplay.ts", "--format", "json"],
    );
    assert!(
        clean["anchor"]["symbols"]
            .as_array()
            .expect("clean symbols")
            .iter()
            .any(|symbol| symbol["name"] == "frame"),
        "reverted file should expose clean symbol facts: {clean:#}"
    );
    assert!(
        !clean["anchor"]["symbols"]
            .as_array()
            .expect("clean symbols")
            .iter()
            .any(|symbol| symbol["name"] == "dirtyFrame"),
        "stale dirty symbol must not survive after status becomes clean: {clean:#}"
    );
}

#[test]
fn partial_rescan_rechecks_removed_paths_that_are_restored_after_dirty_cache() {
    let (repo, cache) = fixture();
    let path = repo.path().join("packages/app/src/useReplay.ts");
    let original =
        "import { seek } from '@fixture/replay';\n\nexport const frame = seek(1).frame;\n";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    std::fs::remove_file(&path).expect("delete tracked source");
    let missing = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/useReplay.ts", "--format", "json"],
    );
    assert_eq!(
        missing["mode"], "missing",
        "dirty deleted cache precondition should remove file facts: {missing:#}"
    );

    write(&path, original);
    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "a restored path from a dirty deleted cache must be scanned without a full repo scan: {doctor:#}"
    );

    let restored = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/useReplay.ts", "--format", "json"],
    );
    assert_eq!(
        restored["mode"], "file",
        "restored source file must be rebuilt into the cached project: {restored:#}"
    );
}

#[test]
fn conflict_cache_without_status_probe_falls_back_to_file_fingerprints() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";
    let path = repo.path().join(rel);

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    git(repo.path(), &["checkout", "-q", "-b", "other"]);
    write(
        &path,
        "import { seek } from '@fixture/replay';\n\nexport const otherFrame = seek(41).frame;\n",
    );
    git(repo.path(), &["add", rel]);
    git(repo.path(), &["commit", "-qm", "other frame"]);
    git(repo.path(), &["checkout", "-q", "main"]);
    write(
        &path,
        "import { seek } from '@fixture/replay';\n\nexport const mainFrame = seek(42).frame;\n",
    );
    git(repo.path(), &["add", rel]);
    git(repo.path(), &["commit", "-qm", "main frame"]);

    let merge = Command::new("git")
        .args(["merge", "other"])
        .current_dir(repo.path())
        .output()
        .expect("git merge should run");
    assert!(
        !merge.status.success(),
        "fixture should create a merge conflict"
    );
    let conflicted = run_json(repo.path(), cache.path(), &["ls", rel, "--format", "json"]);
    assert!(
        conflicted["anchor"]["symbols"]
            .as_array()
            .expect("conflicted symbols")
            .iter()
            .any(|symbol| symbol["name"] == "otherFrame"),
        "conflicted cache precondition should include conflict-only facts: {conflicted:#}"
    );

    git(repo.path(), &["merge", "--abort"]);
    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_ne!(
        doctor["cache_strategy"], "warm_load",
        "cache written without a valid git-status probe must not be trusted as a warm status hit: {doctor:#}"
    );
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "invalid status probe should fall back to fingerprint delta and scan only the mismatched file: {doctor:#}"
    );

    let clean = run_json(repo.path(), cache.path(), &["ls", rel, "--format", "json"]);
    let symbols = clean["anchor"]["symbols"].as_array().expect("clean symbols");
    assert!(
        symbols.iter().any(|symbol| symbol["name"] == "mainFrame"),
        "post-abort file should expose HEAD facts: {clean:#}"
    );
    assert!(
        !symbols.iter().any(|symbol| symbol["name"] == "otherFrame"),
        "conflict-only symbol must not survive status-probe fallback: {clean:#}"
    );
}

#[test]
fn active_conflict_cache_scans_only_fingerprint_mismatches() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";
    let path = repo.path().join(rel);
    let changed_rel = "packages/replay/src/internal.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    git(repo.path(), &["checkout", "-q", "-b", "other"]);
    write(
        &path,
        "import { seek } from '@fixture/replay';\n\nexport const otherFrame = seek(51).frame;\n",
    );
    git(repo.path(), &["add", rel]);
    git(repo.path(), &["commit", "-qm", "other frame"]);
    git(repo.path(), &["checkout", "-q", "main"]);
    write(
        &path,
        "import { seek } from '@fixture/replay';\n\nexport const mainFrame = seek(52).frame;\n",
    );
    git(repo.path(), &["add", rel]);
    git(repo.path(), &["commit", "-qm", "main frame"]);

    let merge = Command::new("git")
        .args(["merge", "other"])
        .current_dir(repo.path())
        .output()
        .expect("git merge should run");
    assert!(!merge.status.success(), "fixture should create a merge conflict");
    let _ = run_json(repo.path(), cache.path(), &["ls", rel, "--format", "json"]);

    write(
        &repo.path().join(changed_rel),
        "export const internalValue = 2;\n",
    );
    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["cache_strategy"], "partial_rescan");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "when git status is unavailable, cache fallback should rescan only fingerprint mismatches: {doctor:#}"
    );

    git(repo.path(), &["merge", "--abort"]);
}

#[test]
fn git_known_same_size_same_mtime_change_rebuilds_aliased_consumer_facts() {
    let repo = TempDir::new().expect("same-metadata repo");
    let warm_cache = TempDir::new().expect("warm cache");
    let fresh_cache = TempDir::new().expect("fresh cache");
    let timestamp_copy = TempDir::new().expect("timestamp copy");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"same-metadata-consumer","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    let consumer = repo.path().join("src/consumer.ts");
    let before = "import { target as alias } from './target';\n\
export function consumer() { return 0      ; }\n";
    let after = "import { target as alias } from './target';\n\
export function consumer() { return alias(); }\n";
    assert_eq!(before.len(), after.len(), "fixture must preserve file size");
    write(&consumer, before);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "initial consumer"]);

    let cold = run_json(
        repo.path(),
        warm_cache.path(),
        &["where", "target", "--format", "json"],
    );
    let cold_consumers = cold["definitions"][0]["consumers"]
        .as_array()
        .expect("cold consumers");
    assert!(
        cold_consumers
            .iter()
            .all(|edge| edge["from"] != "src/consumer.ts"),
        "fixture must start without an observed aliased consumer: {cold:#}"
    );

    let saved_timestamp = timestamp_copy.path().join("consumer.ts");
    let copied = Command::new("cp")
        .args(["-p", consumer.to_str().unwrap(), saved_timestamp.to_str().unwrap()])
        .status()
        .expect("copy preserved timestamp");
    assert!(copied.success(), "timestamp copy should succeed");
    write(&consumer, after);
    let restored = Command::new("touch")
        .args(["-r", saved_timestamp.to_str().unwrap(), consumer.to_str().unwrap()])
        .status()
        .expect("restore timestamp");
    assert!(restored.success(), "timestamp restore should succeed");
    let current_meta = std::fs::metadata(&consumer).expect("consumer metadata");
    let saved_meta = std::fs::metadata(&saved_timestamp).expect("saved metadata");
    assert_eq!(current_meta.len(), saved_meta.len(), "size must be unchanged");
    assert_eq!(
        current_meta.modified().expect("current mtime"),
        saved_meta.modified().expect("saved mtime"),
        "mtime must be restored exactly"
    );

    let doctor = run_json(
        repo.path(),
        warm_cache.path(),
        &["doctor", "--format", "json"],
    );
    assert_eq!(doctor["cache_strategy"], "partial_rescan", "{doctor:#}");
    assert_eq!(
        doctor["scanner"]["files_scanned"], 1,
        "git-known content changes cannot be accepted from size and mtime: {doctor:#}"
    );
    let warm = run_json(
        repo.path(),
        warm_cache.path(),
        &["where", "target", "--format", "json"],
    );
    let fresh = run_json(
        repo.path(),
        fresh_cache.path(),
        &["where", "target", "--format", "json"],
    );
    let consumer_paths = |report: &Value| {
        report["definitions"][0]["consumers"]
            .as_array()
            .expect("consumers")
            .iter()
            .map(|edge| edge["from"].as_str().expect("consumer path").to_string())
            .collect::<std::collections::BTreeSet<_>>()
    };
    let warm_paths = consumer_paths(&warm);
    let fresh_paths = consumer_paths(&fresh);
    assert_eq!(
        warm_paths, fresh_paths,
        "warm cache must reproduce a fresh scan after the metadata collision"
    );
    assert!(
        warm_paths.contains("src/consumer.ts"),
        "the aliased consumer added by the changed content must be observed: {warm:#}"
    );
    assert_eq!(
        warm["definitions"][0]["consumers_total"], fresh["definitions"][0]["consumers_total"],
        "warm and fresh consumer evidence must agree"
    );
}
