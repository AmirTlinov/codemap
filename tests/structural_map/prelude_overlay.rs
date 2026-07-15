#[test]
fn prelude_json_reports_fresh_local_repo_truth_without_network_claims() {
    let (repo, cache) = fixture();
    git(
        repo.path(),
        &[
            "remote",
            "add",
            "origin",
            "https://token:secret@github.com/example/private-repo.git",
        ],
    );
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\n\nexport function seek(cursor: number) {\n  return { frame: new Timeline().frameAt(cursor + 10) };\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/staged.ts"),
        "export const staged = true;\n",
    );
    git(repo.path(), &["add", "packages/replay/src/staged.ts"]);
    write(&repo.path().join("notes scratch.txt"), "untracked\n");

    for args in [
        vec!["ls", ".", "--format", "json"],
        vec!["cone", "packages/replay/src/session.ts", "--format", "json"],
        vec!["changed", "--format", "json"],
        vec!["proof", "changed", "--format", "json"],
    ] {
        let report = run_json(repo.path(), cache.path(), &args);
        let identity = &report["build_identity"];
        assert_eq!(identity["semver"], env!("CARGO_PKG_VERSION"));
        assert_eq!(identity["binary_sha256"], Value::Null);
        assert_eq!(identity["binary_sha256_state"], "not_requested");
        assert!(
            identity["executable_path"]
                .as_str()
                .is_some_and(|path| Path::new(path).is_absolute()),
            "daily report should identify the running executable: {report:#}"
        );
        let prelude = &report["prelude"];
        assert!(
            prelude.is_object(),
            "primary JSON report should include fresh prelude: {report:#}"
        );
        assert_eq!(prelude["vcs"], "git");
        assert_eq!(prelude["freshness"]["network_used"], false);
        assert_eq!(prelude["freshness"]["remote_refs"], "local_only");
        assert_eq!(prelude["freshness"]["remote_currentness"], "unknown");
        assert_eq!(prelude["worktree"]["clean"], false);
        assert_eq!(prelude["worktree"]["staged"], 1);
        assert_eq!(prelude["worktree"]["unstaged"], 1);
        assert_eq!(prelude["worktree"]["untracked"], 1);
        assert_eq!(prelude["remote"]["display"], "github.com/example/private-repo");
        assert!(
            !prelude["remote"]["sanitized_url"]
                .as_str()
                .unwrap_or_default()
                .contains("secret"),
            "prelude must not leak remote credentials: {prelude:#}"
        );
    }
}

#[test]
fn prelude_markdown_replaces_stale_snapshot_for_primary_daily_commands() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { seek } from '@fixture/replay';\n\nexport const frame = seek(100).frame;\n",
    );
    let ls = run_lens_stdout(repo.path(), cache.path(), &["ls", "."]);
    assert!(ls.contains("Repo: root=`"), "ls should render compact repo prelude: {ls}");
    assert!(
        ls.contains("worktree=`dirty; staged=0 unstaged=1 untracked=0 conflicts=0`"),
        "ls prelude should use fresh dirty worktree counts: {ls}"
    );
    assert!(
        ls.contains("Map Snapshot: root=`"),
        "primary daily markdown should keep structural snapshot provenance beside Repo prelude: {ls}"
    );

    let changed = run_lens_stdout(repo.path(), cache.path(), &["changed"]);
    assert!(
        changed.contains("Repo:\n  root: `"),
        "changed should render multiline repo prelude: {changed}"
    );
    assert!(
        changed.contains("  worktree: `dirty; staged=0 unstaged=1 untracked=0 conflicts=0`"),
        "changed prelude should show fresh worktree counts: {changed}"
    );
    assert!(
        changed.contains("  remote_refs: `local_only; currentness unknown; no network used`"),
        "changed prelude should name local-only remote currentness: {changed}"
    );
}

#[test]
fn prelude_is_public_overlay_not_cached_lens_body() {
    let (repo, cache) = fixture();
    let ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        ls.get("prelude").is_some(),
        "public JSON should include prelude overlay: {ls:#}"
    );
    let artifact = cached_lens_artifact_json(cache.path(), "ls-current.json");
    assert!(
        artifact.get("prelude").is_none(),
        "lens cache artifact must store structural body only, not live repo prelude: {artifact:#}"
    );
    assert!(
        artifact.get("build_identity").is_none(),
        "lens cache artifact must not freeze live binary identity: {artifact:#}"
    );
}

#[test]
fn prelude_handles_non_git_directory_without_scary_error() {
    let repo = TempDir::new().expect("non-git tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    write(&repo.path().join("package.json"), r#"{"name":"no-git"}"#);

    let report = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &report);
    let prelude = &report["prelude"];
    assert_eq!(prelude["vcs"], serde_json::Value::Null);
    assert_eq!(prelude["git_root"], serde_json::Value::Null);
    assert_eq!(prelude["worktree"]["clean"], true);
    assert_eq!(prelude["freshness"]["network_used"], false);
    assert_eq!(prelude["freshness"]["remote_refs"], "not_applicable");
}

#[test]
fn prelude_rename_old_path_is_not_counted_as_status_record() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("1 old.ts"), "export const oldName = true;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "rename fixture"]);
    git(repo.path(), &["mv", "1 old.ts", "2 new.ts"]);

    let report = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &report);
    let worktree = &report["prelude"]["worktree"];
    assert_eq!(worktree["renamed"], 1);
    assert_eq!(worktree["staged"], 1);
    assert_eq!(worktree["unstaged"], 0);
    assert_eq!(worktree["untracked"], 0);
}

#[test]
fn prelude_handles_unborn_and_detached_git_states_from_local_status_only() {
    let unborn = TempDir::new().expect("unborn repo tempdir");
    let unborn_cache = TempDir::new().expect("unborn cache tempdir");
    git(unborn.path(), &["init", "-q"]);
    write(&unborn.path().join("README.md"), "# unborn\n");

    let unborn_report = run_json(unborn.path(), unborn_cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &unborn_report);
    assert_eq!(unborn_report["prelude"]["head"]["unborn"], true);
    assert_eq!(unborn_report["prelude"]["head"]["oid"], serde_json::Value::Null);
    assert_eq!(unborn_report["prelude"]["worktree"]["untracked"], 1);
    assert_eq!(unborn_report["prelude"]["freshness"]["network_used"], false);

    let detached = TempDir::new().expect("detached repo tempdir");
    let detached_cache = TempDir::new().expect("detached cache tempdir");
    git(detached.path(), &["init", "-q"]);
    git(detached.path(), &["config", "user.email", "a@example.com"]);
    git(detached.path(), &["config", "user.name", "a"]);
    write(&detached.path().join("README.md"), "# detached\n");
    git(detached.path(), &["add", "."]);
    git(detached.path(), &["commit", "-qm", "detached fixture"]);
    git(detached.path(), &["checkout", "--detach", "HEAD"]);

    let detached_report = run_json(
        detached.path(),
        detached_cache.path(),
        &["changed", "--format", "json"],
    );
    assert_schema("schemas/changed.schema.json", &detached_report);
    assert_eq!(detached_report["prelude"]["head"]["detached"], true);
    assert_eq!(detached_report["prelude"]["branch"]["name"], serde_json::Value::Null);
    assert_eq!(detached_report["prelude"]["freshness"]["network_used"], false);
}

#[test]
fn empty_non_git_root_stays_directory_anchor() {
    let repo = TempDir::new().expect("empty non-git tempdir");
    let cache = TempDir::new().expect("cache tempdir");

    let report = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &report);
    assert_eq!(report["mode"], "directory");
    assert_eq!(report["prelude"]["vcs"], serde_json::Value::Null);
    assert_eq!(report["prelude"]["freshness"]["remote_refs"], "not_applicable");
    assert!(
        report["next"]
            .as_array()
            .expect("next")
            .iter()
            .all(|command| command != "codemap ls ."),
        "empty root ls must not suggest expanding with the same command: {report:#}"
    );
}

#[test]
fn missing_or_file_root_does_not_become_directory_anchor() {
    let cwd = TempDir::new().expect("cwd tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let missing_root = cwd.path().join("missing-root");
    let file_root = cwd.path().join("root-file.txt");
    write(&file_root, "not a directory\n");

    for root in [missing_root, file_root] {
        let output = codemap()
            .current_dir(cwd.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args([
                "--root",
                root.to_str().expect("utf8 path"),
                "ls",
                ".",
                "--format",
                "json",
            ])
            .output()
            .expect("codemap should run");
        assert!(
            output.status.success(),
            "codemap missing/file root failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("valid json");
        assert_schema("schemas/ls.schema.json", &report);
        assert_eq!(report["mode"], "missing");
        assert!(
            report["directory"]
                .as_array()
                .expect("directory")
                .is_empty(),
            "missing/file root must not emit synthetic directory surfaces: {report:#}"
        );
        assert!(
            report["next"].as_array().expect("next").is_empty(),
            "missing/file root must not suggest self-expanding `ls .`: {report:#}"
        );
    }
}
