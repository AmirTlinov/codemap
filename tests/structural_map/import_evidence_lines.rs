// Responsibility: import-evidence line truth (spec-text fallback and explicit line-unknown rendering)

#[test]
fn side_effect_import_evidence_finds_the_spec_line() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"evidence-line-fixture","private":true}"#,
    );
    write(
        &repo.path().join("src/app.ts"),
        "import './widgets';\nexport const app = 1;\n",
    );
    write(
        &repo.path().join("src/widgets/index.ts"),
        "export const widget = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "evidence line fixture"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/app.ts", "--format", "json"],
    );
    let edge = ls["edges"]
        .as_array()
        .expect("edges")
        .iter()
        .find(|edge| edge["type"] == "imports" && edge["to"] == "src/widgets/index.ts")
        .expect("side-effect import edge");
    assert_eq!(
        edge["locations"][0]["line_start"], 1,
        "a side-effect import must locate the line carrying its spec text instead of a lineless file fact: {ls:#}"
    );
}

#[test]
fn edge_without_line_renders_explicit_line_unknown() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"line-unknown-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("src/session_report.rs"),
        "pub fn session_report() -> bool { true }\n",
    );
    write(
        &repo.path().join("tests/session_report_check.rs"),
        "#[test]\nfn session_report_stays_true() { assert!(true); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "line unknown fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "src/session_report.rs", "--depth", "1"])
        .output()
        .expect("cone should run");
    assert!(
        output.status.success(),
        "cone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("`tests/session_report_check.rs` (line unknown)"),
        "an edge location without a line must say so instead of posing as a located fact: {markdown}"
    );
}
