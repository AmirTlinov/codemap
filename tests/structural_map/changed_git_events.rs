#[test]
fn changed_reports_deleted_and_renamed_structural_events() {
    let (repo, cache) = fixture();
    git(
        repo.path(),
        &[
            "mv",
            "packages/replay/src/timeline.ts",
            "packages/replay/src/timeline-clock.ts",
        ],
    );
    std::fs::remove_file(repo.path().join("packages/replay/src/internal.ts"))
        .expect("delete fixture file");

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert!(
        changed["git_state"]
            .as_array()
            .expect("git state")
            .iter()
            .any(|entry| entry["status"] == "renamed"
                && entry["old_path"] == "packages/replay/src/timeline.ts"
                && entry["path"] == "packages/replay/src/timeline-clock.ts"),
        "changed should preserve old/new paths for git renames: {changed:#}"
    );
    assert!(
        changed["structural_events"]
            .as_array()
            .expect("structural events")
            .iter()
            .any(|event| event["kind"] == "renamed_anchor"
                && event["old_path"] == "packages/replay/src/timeline.ts"
                && event["path"] == "packages/replay/src/timeline-clock.ts"
                && event["evidence"] == "git_status"
                && event["locations"][0]["kind"] == "git_renamed"),
        "changed should turn renames into explicit structural events: {changed:#}"
    );
    assert!(
        changed["structural_events"]
            .as_array()
            .expect("structural events")
            .iter()
            .any(|event| event["kind"] == "removed_anchor"
                && event["path"] == "packages/replay/src/internal.ts"
                && event["old_path"].is_null()
                && event["evidence"] == "git_status"
                && event["locations"][0]["kind"] == "git_deleted"
                && event["expand"] == "codemap diff-map --changed"),
        "changed should turn deletions into explicit structural events: {changed:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .arg("changed")
        .output()
        .expect("changed markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Structural Events")
            && markdown.contains("[removed_anchor; evidence=git_status]")
            && markdown.contains("[renamed_anchor; evidence=git_status]"),
        "changed markdown should expose structural deletion/rename events compactly: {markdown}"
    );
    assert!(
        !markdown.contains("safe") && !markdown.contains("probably unused"),
        "changed deletion events must not claim deletion safety: {markdown}"
    );
}

#[test]
fn changed_rename_structural_event_preserves_paths_with_spaces() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("src/with space/old name.ts"),
        "export const value = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "path with spaces"]);
    git(
        repo.path(),
        &["mv", "src/with space/old name.ts", "src/with space/new name.ts"],
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert!(
        changed["structural_events"]
            .as_array()
            .expect("structural events")
            .iter()
            .any(|event| event["kind"] == "renamed_anchor"
                && event["old_path"] == "src/with space/old name.ts"
                && event["path"] == "src/with space/new name.ts"
                && event["expand"] == "codemap cone 'src/with space/new name.ts'"),
        "renamed paths with spaces must not keep porcelain quotes in structural events: {changed:#}"
    );
}

#[test]
fn changed_rename_into_ignored_dir_surfaces_old_path_removal() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/live.ts"),
        "export const live = true;\n",
    );
    git(repo.path(), &["add", "packages/app/src/live.ts"]);
    git(repo.path(), &["commit", "-qm", "add live source"]);
    std::fs::create_dir_all(repo.path().join("dist")).expect("create ignored dist");
    git(
        repo.path(),
        &["mv", "packages/app/src/live.ts", "dist/live.ts"],
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert!(
        changed["git_state"]
            .as_array()
            .expect("git state")
            .iter()
            .any(|entry| entry["status"] == "deleted"
                && entry["path"] == "packages/app/src/live.ts"),
        "rename into ignored dir should keep the old source path as a deleted git-state fact: {changed:#}"
    );
    assert!(
        changed["structural_events"]
            .as_array()
            .expect("structural events")
            .iter()
            .any(|event| event["kind"] == "removed_anchor"
                && event["path"] == "packages/app/src/live.ts"
                && event["expand"] == "codemap diff-map --changed"),
        "rename into ignored dir should surface a removed anchor instead of a clean map: {changed:#}"
    );
    assert_eq!(
        changed["map_delta"]["removed_exports"].as_u64(),
        Some(1),
        "old source removal should still feed diff-map structural deltas: {changed:#}"
    );
}

#[test]
fn changed_staged_rename_into_ignored_dir_surfaces_old_path_removal() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/staged-live.ts"),
        "export const stagedLive = true;\n",
    );
    git(repo.path(), &["add", "packages/app/src/staged-live.ts"]);
    git(repo.path(), &["commit", "-qm", "add staged live source"]);
    std::fs::create_dir_all(repo.path().join("dist")).expect("create ignored dist");
    git(
        repo.path(),
        &["mv", "packages/app/src/staged-live.ts", "dist/staged-live.ts"],
    );

    let changed = run_json(
        repo.path(),
        cache.path(),
        &["changed", "--staged", "--format", "json"],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    assert!(
        changed["git_state"]
            .as_array()
            .expect("git state")
            .iter()
            .any(|entry| entry["status"] == "deleted"
                && entry["path"] == "packages/app/src/staged-live.ts"),
        "changed --staged should keep the old source path as a deleted git-state fact when a rename target is ignored: {changed:#}"
    );
    assert!(
        changed["structural_events"]
            .as_array()
            .expect("structural events")
            .iter()
            .any(|event| event["kind"] == "removed_anchor"
                && event["path"] == "packages/app/src/staged-live.ts"
                && event["expand"] == "codemap diff-map --staged"),
        "changed --staged should surface the removed old anchor instead of reporting a clean map: {changed:#}"
    );
    assert_eq!(
        changed["map_delta"]["removed_exports"].as_u64(),
        Some(1),
        "staged old source removal should feed diff-map structural deltas: {changed:#}"
    );
}
