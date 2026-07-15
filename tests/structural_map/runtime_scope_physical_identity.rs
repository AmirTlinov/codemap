// Responsibility: runtime-scope-physical-identity-and-replacement-boundaries
#[test]
fn runtime_exact_replacement_keeps_indexed_descendant_as_an_incomplete_candidate() {
    let repo = TempDir::new().expect("runtime replacement repo");
    let cache = TempDir::new().expect("runtime replacement cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("scope.ts/workers/job.ts"),
        "export async function run() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "indexed runtime descendant"],
    );

    fs::remove_dir_all(repo.path().join("scope.ts")).expect("remove tracked scope directory");
    write(
        &repo.path().join("scope.ts"),
        "export const replacement = true;\n",
    );

    let exact = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "scope.ts", "--format", "json"],
    );
    let root = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    for report in [&exact, &root] {
        let workers = horizon(&report["observations"], "workers");
        assert_eq!(workers["count"]["observed"], 0, "{report:#}");
        assert_eq!(workers["count"]["closure"], "open", "{report:#}");
        let certificate = runtime_group_certificate(report, "workers");
        assert!(
            certificate["visited_files"].as_u64() < certificate["eligible_files"].as_u64(),
            "the unavailable descendant cannot count as visited: {report:#}"
        );
        assert!(
            certificate["excluded_files_by_reason"]["incomplete_traversal"]
                .as_array()
                .expect("replacement exclusions")
                .iter()
                .any(|path| path == "scope.ts/workers/job.ts"),
            "the exact scope must retain its indexed descendant: {report:#}"
        );
    }
}

#[test]
fn runtime_exact_file_replaced_by_directory_keeps_both_truths_without_proven_zero() {
    let repo = TempDir::new().expect("runtime inverse replacement repo");
    let cache = TempDir::new().expect("runtime inverse replacement cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("scope.ts"),
        "export const original = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "indexed runtime file"]);

    fs::remove_file(repo.path().join("scope.ts")).expect("remove tracked runtime file");
    write(
        &repo.path().join("scope.ts/workers/job.ts"),
        "export async function run() { return true; }\n",
    );

    let exact = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "scope.ts", "--format", "json"],
    );
    let root = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    for report in [&exact, &root] {
        let workers = horizon(&report["observations"], "workers");
        assert_eq!(workers["count"]["observed"], 1, "{report:#}");
        assert_eq!(workers["count"]["closure"], "open", "{report:#}");
        let certificate = runtime_group_certificate(report, "workers");
        assert!(
            certificate["visited_files"].as_u64() < certificate["eligible_files"].as_u64(),
            "the replacement directory must not make the stale exact file readable: {report:#}"
        );
        assert!(
            certificate["excluded_files_by_reason"]["incomplete_traversal"]
                .as_array()
                .expect("inverse replacement exclusions")
                .iter()
                .any(|path| path == "scope.ts"),
            "the exact stale file boundary must remain explicit beside its descendant: {report:#}"
        );
    }
}

#[test]
fn runtime_scope_snapshot_tracks_missing_empty_and_ignored_nonempty_state() {
    let repo = TempDir::new().expect("runtime scope identity repo");
    let cache = TempDir::new().expect("runtime scope identity cache");
    let scope = "stateful-runtime-scope";
    initialize_runtime_coverage_repo(&repo);
    write(&repo.path().join(".gitignore"), &format!("{scope}/\n"));
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "runtime scope identity fixture"],
    );

    let missing = run_json(
        repo.path(),
        cache.path(),
        &["runtime", scope, "--format", "json"],
    );
    fs::create_dir_all(repo.path().join(scope)).expect("empty runtime scope");
    let empty = run_json(
        repo.path(),
        cache.path(),
        &["runtime", scope, "--format", "json"],
    );
    write(
        &repo.path().join(scope).join("ignored.rs"),
        "fn main() {}\n",
    );
    let ignored_nonempty = run_json(
        repo.path(),
        cache.path(),
        &["runtime", scope, "--format", "json"],
    );

    for (report, expected_closure) in [
        (&missing, "unavailable"),
        (&empty, "closed"),
        (&ignored_nonempty, "unavailable"),
    ] {
        for group in S03C_RUNTIME_GROUPS {
            assert_eq!(
                horizon(&report["observations"], group)["count"]["closure"],
                expected_closure,
                "{group}: {report:#}"
            );
        }
    }

    let snapshots = [
        one_runtime_report_snapshot(&missing),
        one_runtime_report_snapshot(&empty),
        one_runtime_report_snapshot(&ignored_nonempty),
    ];
    assert_ne!(snapshots[0], snapshots[1], "missing and empty differ");
    assert_ne!(snapshots[1], snapshots[2], "empty and nonempty differ");
    assert_ne!(snapshots[0], snapshots[2], "missing and nonempty differ");
}

#[test]
fn empty_git_root_and_disjoint_scanner_directories_close_on_cold_and_warm_paths() {
    let repo = TempDir::new().expect("empty Git runtime root");
    let cache = TempDir::new().expect("empty Git runtime cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("target/debug/probe"), "cache output\n");
    write(
        &repo.path().join("node_modules/pkg/index.js"),
        "module.exports = true;\n",
    );
    write(&repo.path().join(".codemap/state.json"), "{}\n");

    let cold = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let cold_artifact = runtime_root_cache_json(cache.path());
    for group in S03C_RUNTIME_GROUPS {
        let item = horizon(&cold_artifact["report"]["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {cold_artifact:#}");
        assert_eq!(
            item["count"]["closure"], "closed",
            "{group}: {cold_artifact:#}"
        );
    }

    let artifact_path = lens_artifact_path(cache.path(), "runtime-root.json");
    let mut primed = cold_artifact;
    primed["warm_path_probe"] = serde_json::json!(true);
    fs::write(
        &artifact_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&primed).expect("empty root cache probe")
        ),
    )
    .expect("prime empty root warm cache");

    let warm = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    assert_lens_markdown_eq(
        &cold,
        &warm,
        "repository metadata and scanner-disjoint directories cannot turn a logical empty root open",
    );
    assert_eq!(
        runtime_root_cache_json(cache.path())["warm_path_probe"],
        true,
        "the second read must use the unchanged warm artifact"
    );
}

#[test]
fn recursively_empty_runtime_scope_is_a_valid_proven_zero() {
    let repo = TempDir::new().expect("recursive empty runtime scope");
    let cache = TempDir::new().expect("recursive empty runtime cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    fs::create_dir_all(repo.path().join("empty-tree/a/b/c"))
        .expect("recursively empty runtime directories");
    write(
        &repo.path().join("empty-tree/a/node_modules/pkg/index.js"),
        "module.exports = true;\n",
    );

    let report = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "empty-tree", "--format", "json"],
    );
    for group in S03C_RUNTIME_GROUPS {
        let item = horizon(&report["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {report:#}");
        assert_eq!(item["count"]["closure"], "closed", "{group}: {report:#}");
    }
}

#[test]
fn ignored_nonempty_root_invalidates_warm_cache_and_opens_every_runtime_group() {
    let repo = TempDir::new().expect("ignored runtime root");
    let cache = TempDir::new().expect("ignored runtime root cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".git/info/exclude"),
        "custom-ignored-runtime/\n",
    );

    run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let cold_artifact = runtime_root_cache_json(cache.path());
    let cold_snapshot = one_runtime_report_snapshot(&cold_artifact["report"]);
    let artifact_path = lens_artifact_path(cache.path(), "runtime-root.json");
    let mut primed = cold_artifact.clone();
    primed["warm_path_probe"] = serde_json::json!(true);
    fs::write(
        &artifact_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&primed).expect("ignored root cache probe")
        ),
    )
    .expect("prime ignored root warm cache");

    write(
        &repo.path().join("custom-ignored-runtime/secret.ts"),
        "export const hiddenRuntime = true;\n",
    );
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo.path())
        .output()
        .expect("ignored root git status");
    assert!(status.status.success(), "git status must succeed");
    assert!(
        status.stdout.is_empty(),
        "the transition must be invisible to the Git status fingerprint"
    );

    let warm = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let repaired = runtime_root_cache_json(cache.path());
    assert!(
        repaired["warm_path_probe"].is_null(),
        "physical root identity must invalidate and replace the previously valid artifact"
    );
    assert_eq!(
        repaired["fingerprint"], cold_artifact["fingerprint"],
        "the project/status snapshot intentionally stays stable across an ignored entry"
    );
    assert_ne!(
        one_runtime_report_snapshot(&repaired["report"]),
        cold_snapshot,
        "runtime certificate identity must include semantic physical root state"
    );
    for group in S03C_RUNTIME_GROUPS {
        let item = horizon(&repaired["report"]["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {repaired:#}");
        assert_eq!(item["count"]["closure"], "open", "{group}: {repaired:#}");
        let certificate = runtime_group_certificate(&repaired["report"], group);
        assert!(
            certificate["unresolved_stops"]
                .as_array()
                .expect("root unresolved stops")
                .iter()
                .any(|stop| stop["kind"] == "incomplete_traversal"),
            "ignored physical content must remain an explicit {group} stop: {repaired:#}"
        );
    }

    let no_cache = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .env("CODEMAP_NO_CACHE", "1")
        .args(["runtime", "."])
        .output()
        .expect("no-cache ignored root runtime");
    assert!(
        no_cache.status.success(),
        "no-cache runtime failed: {}",
        String::from_utf8_lossy(&no_cache.stderr)
    );
    assert_lens_markdown_eq(
        &warm,
        &String::from_utf8(no_cache.stdout).expect("ignored root markdown"),
        "repaired warm output must equal live ignored-root truth",
    );
}

fn one_runtime_report_snapshot(report: &Value) -> String {
    let ledger = &report["observations"];
    let horizons = ledger["horizons"].as_array().expect("runtime horizons");
    assert_eq!(horizons.len(), S03C_RUNTIME_GROUPS.len(), "{report:#}");
    let mut snapshots = horizons
        .iter()
        .map(|horizon| {
            let id = horizon["count"]["certificate_id"]
                .as_str()
                .expect("runtime certificate id");
            ledger["certificates"][id]["snapshot"]
                .as_str()
                .expect("runtime certificate snapshot")
                .to_string()
        })
        .collect::<Vec<_>>();
    snapshots.sort();
    snapshots.dedup();
    assert_eq!(
        snapshots.len(),
        1,
        "one runtime report must have one observation snapshot: {report:#}"
    );
    snapshots.pop().expect("one runtime snapshot")
}
