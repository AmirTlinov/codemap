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

#[test]
fn ls_file_anchor_renders_metadata_and_symbols_without_tables() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "packages/replay/src/session.ts"])
        .output()
        .expect("ls file markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## File\n") && markdown.contains("- kind: `runtime_state`"),
        "ls file markdown should render file metadata as compact bullets: {markdown}"
    );
    assert!(
        markdown.contains("\n## Symbols\n") && markdown.contains("["),
        "ls file markdown should keep a compact symbol outline: {markdown}"
    );
    assert!(
        markdown.contains("exported=true") && markdown.contains("lines=4-6"),
        "ls file symbol outline should preserve exported status and line range: {markdown}"
    );
    assert!(
        !markdown.contains("| Field | Value |")
            && !markdown.contains("| Name | Kind | Exported | Lines |"),
        "ls file markdown should not use metadata or symbol tables: {markdown}"
    );
}

#[test]
fn ls_directory_surfaces_render_as_compact_blocks() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "packages/replay"])
        .output()
        .expect("ls directory markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Surfaces\n") && markdown.contains("[role="),
        "ls directory markdown should render surfaces as compact blocks: {markdown}"
    );
    assert!(
        markdown.contains("count=")
            && markdown.contains("file_role_or_extension")
            && markdown.contains("medium"),
        "ls directory surface blocks should preserve count, evidence, and strength: {markdown}"
    );
    assert!(
        markdown.contains("examples: `"),
        "ls directory markdown should preserve visible surface examples: {markdown}"
    );
    assert!(
        !markdown.contains("| Kind | Role | Count | Evidence | Strength | Examples |"),
        "ls directory markdown should not use the old surface table: {markdown}"
    );
}
