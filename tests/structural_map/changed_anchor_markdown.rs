#[test]
fn changed_markdown_summarizes_large_export_lists() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"changed-anchor-compact-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    let exports = (0..12)
        .map(|index| format!("export function exported{index}() {{ return {index}; }}"))
        .collect::<Vec<_>>()
        .join("\n");
    write(&repo.path().join("src/big.ts"), &format!("{exports}\n"));
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "big export fixture"]);
    write(
        &repo.path().join("src/big.ts"),
        &format!("{exports}\nexport const changedValue = 1;\n"),
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--limit", "20"])
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8_lossy(&output.stdout);
    assert!(
        markdown.contains("exports=13"),
        "changed anchor summary should show export count: {markdown}"
    );
    assert!(
        markdown.contains("exports: changedValue, exported0, exported1, exported10, exported11, exported2 +7 hidden"),
        "changed anchor exports should be previewed with a hidden count, not dumped: {markdown}"
    );
    assert!(
        !markdown.contains("exported3, exported4, exported5, exported6, exported7"),
        "changed anchor markdown should not dump the full export list: {markdown}"
    );
}

#[test]
fn changed_markdown_prints_common_prefix_once() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"changed-prefix-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    for name in ["one", "two", "three"] {
        write(
            &repo.path().join(format!("src/domain/{name}.ts")),
            &format!("export const {name} = 1;\n"),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "changed prefix fixture"]);
    for name in ["one", "two", "three"] {
        write(
            &repo.path().join(format!("src/domain/{name}.ts")),
            &format!("export const {name} = 2;\n"),
        );
    }

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--limit", "20"])
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8_lossy(&output.stdout);
    assert!(
        markdown.matches("prefix: `src/domain/`").count() >= 2,
        "changed markdown should print common prefix for Git State, Changed Anchors, and Impact: {markdown}"
    );
    assert!(
        markdown.contains("- `one.ts` [modified; staged=false; unstaged=true]")
            && markdown.contains("- `one.ts` [source; javascript/typescript"),
        "changed markdown should print paths relative to the common prefix: {markdown}"
    );
    assert!(
        !markdown.contains("- `src/domain/one.ts`"),
        "changed markdown should not repeat the common prefix on every row: {markdown}"
    );
    assert!(
        markdown.contains("- `one.ts` [direct=") && !markdown.contains("- `changed:src/domain/one.ts`"),
        "changed impact summary should use the same compact coordinates: {markdown}"
    );
}
