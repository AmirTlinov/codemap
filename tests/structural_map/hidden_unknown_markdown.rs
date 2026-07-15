#[test]
fn hidden_markdown_uses_compact_expand_blocks() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", ".", "--limit", "1"])
        .output()
        .expect("ls markdown should run");
    assert_eq!(output.status.code(), Some(0));
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Hidden\n") && markdown.contains("expand: `codemap ls ."),
        "hidden markdown should keep exact expand commands in compact blocks: {markdown}"
    );
    assert!(
        !markdown.contains("| Reason | Count | Expand |"),
        "hidden markdown should not return to table spam: {markdown}"
    );
}

#[test]
fn unknown_markdown_groups_by_kind_without_table_spam() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "packages/replay/src/not-real.ts"])
        .output()
        .expect("cone markdown should run");
    assert_eq!(output.status.code(), Some(20));
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Unknown\n") && markdown.contains("- `unindexed_anchor`"),
        "unknown markdown should group blind spots by typed kind: {markdown}"
    );
    assert!(
        markdown.contains("where: `packages/replay/src/not-real.ts`")
            && markdown.contains("expand: `codemap ls packages/replay/src`"),
        "unknown markdown should keep where and expand provenance: {markdown}"
    );
    assert!(
        !markdown.contains("| Kind | Where | Reason | Effect | Expand |"),
        "unknown markdown should not return to table spam: {markdown}"
    );
}
