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
