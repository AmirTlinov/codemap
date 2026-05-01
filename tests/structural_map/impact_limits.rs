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
