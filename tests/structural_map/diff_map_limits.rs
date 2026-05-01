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

#[test]
fn diff_map_ignores_added_structural_text_inside_runtime_comments() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/diff-runtime-noise.ts"),
        "export const base = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "diff runtime base"]);

    write(
        &repo.path().join("packages/app/src/diff-runtime-noise.ts"),
        "export const base = true;\n/*\nimport { Fake } from './fake';\nexport const fake = true;\nprocess.env[commentKey];\n*/\nconst docs = \"process.env[stringKey] import(dynamicTarget)\";\nconst key = 'REAL_DYNAMIC';\nconst token = process.env[key];\nvoid docs;\nvoid token;\n",
    );

    let diff = run_json(
        repo.path(),
        cache.path(),
        &["diff-map", "--changed", "--format", "json"],
    );
    assert_schema("schemas/diff-map.schema.json", &diff);
    assert!(
        diff["added_edges"]
            .as_array()
            .expect("added edges")
            .is_empty(),
        "diff-map must not create structural edges from import-looking text inside block comments: {diff:#}"
    );
    assert!(
        diff["added_exports"]
            .as_array()
            .expect("added exports")
            .is_empty(),
        "diff-map must not create export surfaces from export-looking text inside block comments: {diff:#}"
    );
    assert!(
        diff["changed"][0]["exports"]
            .as_array()
            .expect("file exports")
            .iter()
            .all(|export| export != "fake"),
        "file summaries inside diff-map must not surface exports from block comments: {diff:#}"
    );
    assert!(
        diff["changed_symbols"]
            .as_array()
            .expect("changed symbols")
            .iter()
            .all(|symbol| symbol["name"] != "fake"),
        "diff-map changed symbols must not include symbol declarations from block comments: {diff:#}"
    );
    let unknowns = diff["new_unknowns"].as_array().expect("new unknowns");
    assert_eq!(
        unknowns
            .iter()
            .filter(|unknown| unknown["kind"] == "env_dynamic_lookup")
            .count(),
        1,
        "only the real dynamic env lookup should be reported from added lines: {diff:#}"
    );
    assert!(
        unknowns
            .iter()
            .all(|unknown| unknown["kind"] != "dynamic_import"),
        "dynamic-import-looking text inside a string literal must not become a diff-map unknown: {diff:#}"
    );
}
