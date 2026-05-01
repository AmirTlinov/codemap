#[test]
fn diff_map_limit_does_not_skip_selected_changed_files() {
    let (repo, cache) = fixture();
    for name in ["a", "b", "c"] {
        write(
            &repo
                .path()
                .join(format!("packages/replay/src/{name}-delta.ts")),
            "import { Timeline } from './timeline';\n\nexport const delta = new Timeline();\n",
        );
    }

    let changed = run_json(
        repo.path(),
        cache.path(),
        &["diff-map", "--changed", "--limit", "1", "--format", "json"],
    );
    assert_schema("schemas/diff-map.schema.json", &changed);
    assert_eq!(
        changed["changed"].as_array().expect("changed").len(),
        1,
        "limit should bound rendered changed summaries: {changed:#}"
    );
    assert!(
        changed["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "changed file summaries hidden by limit"
                && group["count"] == 2
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.starts_with("codemap diff-map --files packages/replay/src/a-delta.ts,packages/replay/src/b-delta.ts,packages/replay/src/c-delta.ts --limit ")
                        && !expand.contains("<larger-number>")
                })),
        "diff-map should expose hidden changed summaries with a concrete full selected-file snapshot: {changed:#}"
    );
    assert!(
        changed["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "added structural edges hidden by limit"
                && group["count"].as_u64().unwrap_or_default() >= 2),
        "diff-map must still inspect structural lines from changed files beyond the visible changed-summary limit: {changed:#}"
    );
    assert!(
        changed["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"] == "added export surfaces hidden by limit"
                && group["count"].as_u64().unwrap_or_default() >= 2),
        "diff-map must not silently drop export surfaces from changed files beyond the visible limit: {changed:#}"
    );
}
