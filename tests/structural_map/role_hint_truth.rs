#[test]
fn json_review_artifacts_do_not_become_renderer_ui_from_ui_tokens() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("experiments/reviews/ui-review.json"),
        r#"{"ui":"reviewed","status":"pass"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "review artifact"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "experiments/reviews/ui-review.json", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        !ls["anchor"]["roles"]
            .as_array()
            .expect("roles")
            .iter()
            .any(|role| role == "renderer_ui"),
        "JSON review artifacts must not become renderer_ui from ui tokens: {ls:#}"
    );
}

#[test]
fn rust_text_renderers_do_not_become_renderer_ui_from_render_path() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/render/changed.rs"),
        "pub fn render_changed_text() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "text renderer"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/render/changed.rs", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        !ls["anchor"]["roles"]
            .as_array()
            .expect("roles")
            .iter()
            .any(|role| role == "renderer_ui"),
        "Rust text renderers must not become renderer_ui from render path: {ls:#}"
    );
}
