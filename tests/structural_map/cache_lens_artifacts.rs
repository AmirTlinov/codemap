#[test]
fn dirty_daily_lens_artifacts_roundtrip_without_output_drift() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join(rel),
        "import { seek } from '@fixture/replay';\n\nexport const changedFrame = seek(71).frame;\n",
    );

    let changed_first = run_lens_stdout(repo.path(), cache.path(), &["changed"]);
    let changed_second = run_lens_stdout(repo.path(), cache.path(), &["changed"]);
    assert_eq!(
        changed_first, changed_second,
        "cached changed artifact must preserve markdown output"
    );

    let proof_first = run_lens_stdout(repo.path(), cache.path(), &["proof", "--changed"]);
    let proof_second = run_lens_stdout(repo.path(), cache.path(), &["proof", "--changed"]);
    assert_eq!(
        proof_first, proof_second,
        "cached proof artifact must preserve markdown output"
    );

    assert!(
        cached_lens_artifact_exists(cache.path(), "changed-current.json"),
        "changed command should write an external lens artifact"
    );
    assert!(
        cached_lens_artifact_exists(cache.path(), "proof-changed.json"),
        "proof --changed should write an external lens artifact"
    );
}

#[test]
fn navigation_lens_artifacts_roundtrip_without_output_drift() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);

    let ls_first = run_lens_stdout(repo.path(), cache.path(), &["ls", rel]);
    let ls_second = run_lens_stdout(repo.path(), cache.path(), &["ls", rel]);
    assert_eq!(
        ls_first, ls_second,
        "cached ls artifact must preserve markdown output"
    );

    let cone_first = run_lens_stdout(repo.path(), cache.path(), &["cone", rel]);
    let cone_second = run_lens_stdout(repo.path(), cache.path(), &["cone", rel]);
    assert_eq!(
        cone_first, cone_second,
        "cached cone artifact must preserve markdown output"
    );

    assert!(
        cached_lens_artifact_exists(cache.path(), "ls-current.json"),
        "ls command should write an external lens artifact"
    );
    assert!(
        cached_lens_artifact_exists(cache.path(), "cone-current.json"),
        "cone command should write an external lens artifact"
    );
}

#[test]
fn proof_map_lens_artifact_roundtrips_without_output_drift() {
    let (repo, cache) = fixture();

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);

    let proof_map_first = run_lens_stdout(repo.path(), cache.path(), &["proof-map", "."]);
    let proof_map_second = run_lens_stdout(repo.path(), cache.path(), &["proof-map", "."]);
    assert_eq!(
        proof_map_first, proof_map_second,
        "cached proof-map artifact must preserve markdown output"
    );

    assert!(
        cached_lens_artifact_exists(cache.path(), "proof-map-current.json"),
        "proof-map command should write an external lens artifact"
    );
}

#[test]
fn proof_map_staged_cache_rechecks_selector_changed_set() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    write(
        &repo.path().join(rel),
        "import { seek } from '@fixture/replay';\n\nexport const stagedFrame = seek(9).frame;\n",
    );
    git(repo.path(), &["add", rel]);

    let staged_first = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["proof-map", "--staged", "--format", "json"],
    );
    let staged_first: Value =
        serde_json::from_str(&staged_first).expect("first proof-map json");
    assert_eq!(
        staged_first["changed"].as_array().expect("changed files"),
        &[Value::String(rel.to_string())],
        "proof-map --staged should capture the staged file before caching"
    );

    git(repo.path(), &["reset", "-q", "HEAD", "--", rel]);
    let staged_second = run_lens_stdout(
        repo.path(),
        cache.path(),
        &["proof-map", "--staged", "--format", "json"],
    );
    let staged_second: Value =
        serde_json::from_str(&staged_second).expect("second proof-map json");
    assert!(
        staged_second["changed"]
            .as_array()
            .expect("changed files")
            .is_empty(),
        "proof-map cache must not reuse a staged artifact after the staged set changes: {staged_second:#}"
    );
}

fn run_lens_stdout(repo: &Path, cache: &Path, args: &[&str]) -> String {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

fn cached_lens_artifact_exists(cache_root: &Path, name: &str) -> bool {
    fs::read_dir(cache_root)
        .expect("cache dir")
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.path().join(name).exists())
}
