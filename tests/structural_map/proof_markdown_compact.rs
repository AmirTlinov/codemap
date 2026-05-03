#[test]
fn proof_changed_markdown_summarizes_sensors_by_command() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"proof-compact-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/session.ts"),
        "export function sessionValue() {\n  return 1;\n}\n",
    );
    for index in 1..=8 {
        write(
            &repo.path().join(format!("tests/session-{index}.test.ts")),
            "import { sessionValue } from '../src/session';\n\ntest('session value', () => {\n  expect(sessionValue()).toBe(1);\n});\n",
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof compact fixture"]);
    write(
        &repo.path().join("src/session.ts"),
        "export function sessionValue() {\n  return 2;\n}\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed", "--limit", "20"])
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8_lossy(&output.stdout);
    assert!(
        markdown.contains("- sensors: `8`"),
        "proof markdown should summarize command sensor count: {markdown}"
    );
    assert!(
        markdown.contains("- evidence: `test_import: 8`"),
        "proof markdown should show evidence distribution: {markdown}"
    );
    assert!(
        markdown.contains("- hidden details: `3` sensors"),
        "proof markdown should hide excess per-command detail: {markdown}"
    );
    assert!(
        markdown.contains("codemap proof-map --changed --raw-sensors --limit 8"),
        "proof markdown should expose raw-sensor expansion: {markdown}"
    );
    assert_eq!(
        markdown.matches("[test_import; high]").count(),
        5,
        "proof markdown should sample, not dump every direct sensor: {markdown}"
    );

    let changed_output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--limit", "20"])
        .output()
        .expect("codemap should run");
    assert!(
        changed_output.status.success(),
        "codemap changed failed: {}",
        String::from_utf8_lossy(&changed_output.stderr)
    );
    let changed_markdown = String::from_utf8_lossy(&changed_output.stdout);
    assert!(
        changed_markdown.contains("- sensors: `8`"),
        "changed markdown should summarize proof sensor count: {changed_markdown}"
    );
    assert!(
        changed_markdown.contains("- hidden details: `3` sensors"),
        "changed markdown should hide excess proof detail: {changed_markdown}"
    );
    assert_eq!(
        changed_markdown.matches("[test_import; high]").count(),
        5,
        "changed proof section should sample, not dump every direct sensor: {changed_markdown}"
    );
}

#[test]
fn proof_changed_section_filter_works_on_clean_fast_path() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed", "--section", "unknown"])
        .output()
        .expect("proof changed unknown section should run");
    assert!(
        output.status.success(),
        "proof changed --section unknown should run through the clean fast path: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("# Proof Plan") && markdown.contains("## Unknown"),
        "proof changed --section unknown should render a stable Unknown layer: {markdown}"
    );
    assert!(
        !markdown.contains("unexpected argument") && !markdown.contains("## Proof"),
        "proof changed --section unknown should not fall through to CLI errors or proof sections: {markdown}"
    );
}

#[test]
fn proof_changed_unknown_stays_fail_open_for_changed_source_without_direct_test() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"proof-unknown-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/runtime.ts"),
        "export function runtimeValue() {\n  return 1;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof unknown fixture"]);
    write(
        &repo.path().join("src/runtime.ts"),
        "export function runtimeValue() {\n  return 2;\n}\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed", "--section", "unknown"])
        .output()
        .expect("proof changed unknown should run");
    assert!(
        output.status.success(),
        "proof changed unknown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("direct_test_import_not_found")
            && markdown.contains("src/runtime.ts")
            && !markdown.contains("No Unknown entries were emitted"),
        "proof changed must not hide missing direct proof just because script proof exists: {markdown}"
    );
}

#[test]
fn proof_section_filter_is_display_only_and_refuses_run() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed", "--section", "unknown", "--run"])
        .output()
        .expect("proof section run guard should execute");
    assert!(
        !output.status.success(),
        "proof --run must not execute with a display-only section filter"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("--section is display-only"),
        "guard should explain the conflict without running commands: {stderr}"
    );
}
