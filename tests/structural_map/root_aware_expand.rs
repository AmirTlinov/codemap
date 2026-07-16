#[test]
fn explicit_root_is_preserved_in_json_and_markdown_expand_commands() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("crates/worker/Cargo.toml"),
        "[package]\nname = \"fixture-worker\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"fixture-worker\"\npath = \"src/bin/worker.rs\"\n",
    );
    write(
        &repo.path().join("crates/worker/src/bin/worker.rs"),
        "fn main() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "root expand fixture"]);

    let output = codemap()
        .current_dir(repo.path().parent().expect("repo parent"))
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "--root",
            repo.path().to_str().expect("repo path utf8"),
            "runtime",
            "crates/worker",
            "--format",
            "json",
        ])
        .output()
        .expect("codemap json should run");
    assert!(
        output.status.success(),
        "codemap --root json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("json report");
    let agent_expands = report["agent"]["expands"].as_array().expect("agent expands");
    assert!(
        agent_expands
            .iter()
            .all(|command| command.as_array().is_some_and(|argv| {
                argv.first() == Some(&serde_json::json!("codemap"))
                    && argv.get(1) == Some(&serde_json::json!("--root"))
                    && argv.get(2) == Some(&serde_json::json!(repo.path()))
            })),
        "json expand commands should preserve explicit --root: {report:#}"
    );

    let markdown = codemap()
        .current_dir(repo.path().parent().expect("repo parent"))
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "--root",
            repo.path().to_str().expect("repo path utf8"),
            "runtime",
            "crates/worker",
        ])
        .output()
        .expect("codemap markdown should run");
    assert!(
        markdown.status.success(),
        "codemap --root markdown failed: {}",
        String::from_utf8_lossy(&markdown.stderr)
    );
    let stdout = String::from_utf8(markdown.stdout).expect("markdown utf8");
    let rendered_root = format!("'{}'", repo.path().display());
    let rendered_root = if cfg!(windows) {
        rendered_root
    } else {
        repo.path().display().to_string()
    };
    assert!(
        stdout.contains(&format!("`codemap --root {rendered_root} cone crates/worker`"))
            && stdout.contains(&format!(
                "`codemap --root {rendered_root} flow crates/worker/src/bin/worker.rs`"
            )),
        "markdown expand commands should preserve explicit --root: {stdout}"
    );
}

#[test]
fn explicit_root_with_spaces_is_shell_quoted_in_expand_commands() {
    let workspace = TempDir::new().expect("workspace tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let repo = workspace.path().join("repo with spaces");
    std::fs::create_dir_all(&repo).expect("repo dir");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "a@example.com"]);
    git(&repo, &["config", "user.name", "a"]);
    write(
        &repo.join("Cargo.toml"),
        "[package]\nname = \"space-root\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"space-root\"\npath = \"src/main.rs\"\n",
    );
    write(&repo.join("src/main.rs"), "fn main() {}\n");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-qm", "space root fixture"]);

    let markdown = codemap()
        .current_dir(workspace.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["--root", repo.to_str().expect("repo path utf8"), "runtime", "."])
        .output()
        .expect("codemap markdown should run");
    assert!(
        markdown.status.success(),
        "codemap --root markdown failed: {}",
        String::from_utf8_lossy(&markdown.stderr)
    );
    let stdout = String::from_utf8(markdown.stdout).expect("markdown utf8");
    assert!(
        stdout.contains(&format!("`codemap --root '{}' cone .`", repo.display())),
        "root path with spaces should be shell-quoted in expand commands: {stdout}"
    );
}
