#[test]
fn impact_limit_scans_all_selected_anchors_before_truncating_output() {
    let (repo, cache) = fixture();

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/session.ts,packages/replay/src/missing.ts",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    assert_eq!(impact["changed"].as_array().expect("changed").len(), 1);
    assert_eq!(impact["clusters"].as_array().expect("clusters").len(), 1);
    assert!(
        impact["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["path"] == "packages/replay/src/missing.ts"),
        "impact should inspect all selected anchors before truncating rendered sections: {impact:#}"
    );
    let hidden = impact["hidden"].as_array().expect("hidden");
    assert!(
        hidden.iter().any(|group| group["reason"] == "impact clusters hidden by limit"
            && group["count"] == 1
            && group["expand"]
                == "codemap impact --files packages/replay/src/session.ts,packages/replay/src/missing.ts --depth 1 --limit 2"),
        "impact should report hidden clusters with a concrete selector-preserving expand: {impact:#}"
    );
}

#[test]
fn impact_markdown_groups_relations_without_repeated_edge_tables() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["impact", "--files", "packages/replay/src/types.ts"])
        .output()
        .expect("impact markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Impact\n") && markdown.contains("direct consumers:"),
        "impact markdown should have one summary and grouped relation lists: {markdown}"
    );
    assert!(
        markdown.contains("[reverse_import; high]") || markdown.contains("[test_import; high]"),
        "impact markdown should retain structural evidence and strength: {markdown}"
    );
    assert!(
        !markdown.contains("| Field | Value |")
            && !markdown.contains("| Cluster | Risk | Reasons | Edges |")
            && !markdown.contains("| From | Type | To | Evidence | Strength | Where |"),
        "impact markdown should not repeat per-cluster field/edge tables: {markdown}"
    );
    assert!(
        markdown.lines().count() < 80,
        "focused impact markdown should stay compact: {markdown}"
    );
}
