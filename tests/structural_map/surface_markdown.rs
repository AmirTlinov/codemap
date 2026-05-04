#[test]
fn focused_lens_surfaces_render_as_compact_blocks() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["runtime", "."])
        .output()
        .expect("runtime markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("\n## Scripts\n") && markdown.contains("[script; package.json script:"),
        "surface markdown should keep kind/evidence/strength in compact blocks without role-verdict columns: {markdown}"
    );
    assert!(
        markdown.contains("examples: `test:")
            || markdown.contains("examples: `typecheck:"),
        "surface markdown should keep examples without repeating a table row per field: {markdown}"
    );
    assert!(
        !markdown.contains("| Kind | Role | Path | Evidence | Strength | Examples |"),
        "surface markdown should not return to repeated table spam: {markdown}"
    );
}

#[test]
fn surface_markdown_does_not_hide_report_visible_examples() {
    let (repo, cache) = fixture();
    for index in 0..8 {
        write(
            &repo
                .path()
                .join(format!("packages/replay/tests/many-{index}.test.ts")),
            "test('many', () => expect(true).toBe(true));\n",
        );
    }

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["place", "packages/replay", "--kind", "test", "--limit", "6"])
        .output()
        .expect("place markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("packages/replay/tests/many-4.test.ts"),
        "surface markdown should render every example already selected by the report: {markdown}"
    );
    assert!(
        markdown.contains("additional examples:") && !markdown.contains("hidden examples:"),
        "surface markdown should label per-surface samples as additional examples, not hidden map material: {markdown}"
    );
}
