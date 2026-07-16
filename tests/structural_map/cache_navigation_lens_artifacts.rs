#[test]
fn navigation_lens_artifacts_roundtrip_without_output_drift() {
    let (repo, cache) = fixture();
    let rel = "packages/app/src/useReplay.ts";

    let _ = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);

    let ls_first = run_lens_stdout(repo.path(), cache.path(), &["ls", rel]);
    let ls_second = run_lens_stdout(repo.path(), cache.path(), &["ls", rel]);
    assert_lens_markdown_eq(
        &ls_first,
        &ls_second,
        "cached ls artifact must preserve markdown output"
    );

    let cone_first = run_lens_stdout(repo.path(), cache.path(), &["cone", rel]);
    let cone_second = run_lens_stdout(repo.path(), cache.path(), &["cone", rel]);
    assert_lens_markdown_eq(
        &cone_first,
        &cone_second,
        "cached cone artifact must preserve markdown output"
    );

    let where_first = run_lens_stdout(repo.path(), cache.path(), &["where", "seek"]);
    let where_second = run_lens_stdout(repo.path(), cache.path(), &["where", "seek"]);
    assert_eq!(
        &where_first,
        &where_second,
        "cached where artifact must preserve markdown output"
    );
    let where_json_first = run_json(
        repo.path(),
        cache.path(),
        &["where", "seek", "--format", "json"],
    );
    let where_json_second = run_json(
        repo.path(),
        cache.path(),
        &["where", "seek", "--format", "json"],
    );
    assert_eq!(
        where_json_first, where_json_second,
        "cached where artifact must preserve JSON output"
    );
    let different_where = run_json(
        repo.path(),
        cache.path(),
        &["where", "Timeline", "--format", "json"],
    );
    assert_eq!(
        different_where["query"], "Timeline",
        "where cache must miss when the exact query changes"
    );

    for (name, command) in [
        ("ls-current.json", "ls"),
        ("cone-current.json", "cone"),
        ("where-current.json", "where"),
    ] {
        assert!(
            cached_lens_artifact_exists(cache.path(), name),
            "{command} command should write an external lens artifact"
        );
    }
}
