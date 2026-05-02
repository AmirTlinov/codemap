#[test]
fn cone_markdown_groups_edges_by_source_without_edge_table_spam() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "packages/replay/src/types.ts", "--depth", "1"])
        .output()
        .expect("cone markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Links\n") && markdown.contains("incoming:"),
        "cone markdown should keep relation sections with grouped lists: {markdown}"
    );
    assert!(
        markdown.contains("[reverse_import; high]") || markdown.contains("[test_import; high]"),
        "cone markdown should keep evidence and strength in grouped relation rows: {markdown}"
    );
    assert!(
        markdown.contains("`packages/replay/src/session.ts:2`")
            || markdown.contains("`packages/replay/src/index.ts:3`"),
        "cone markdown should keep source locations in grouped relation rows: {markdown}"
    );
    assert!(
        !markdown.contains("| From | Type | To | Evidence | Strength | Where |"),
        "cone markdown should not return to repeated edge table spam: {markdown}"
    );
    assert!(
        !markdown.contains("| Field | Value |") && markdown.contains("\n## Observed\n"),
        "cone markdown should render anchor metadata as compact map bullets: {markdown}"
    );
    assert!(
        markdown.lines().count() < 100,
        "focused cone markdown should stay compact: {markdown}"
    );
}
