#[test]
fn diff_map_file_summaries_render_as_compact_blocks() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/summary-delta.ts"),
        "export const summaryDelta = true;\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["diff-map", "--files", "packages/replay/src/summary-delta.ts"])
        .output()
        .expect("diff-map markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("- `packages/replay/src/summary-delta.ts` [source; javascript/typescript;"),
        "file summaries should render as compact path metadata blocks: {markdown}"
    );
    assert!(
        markdown.contains("package=@fixture/replay; lines=1"),
        "file summaries should preserve package and line-count metadata: {markdown}"
    );
    assert!(
        markdown.contains("exports: summaryDelta"),
        "file summaries should keep useful exact-anchor exports in compact markdown: {markdown}"
    );
    assert!(
        !markdown.contains("| Path | Kind | Package | Language |"),
        "file summaries should not return to table spam: {markdown}"
    );
}
