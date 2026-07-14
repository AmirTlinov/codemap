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
    assert!(
        !ls["anchor"]["roles"]
            .as_array()
            .expect("roles")
            .iter()
            .any(|role| role == "map_surface"),
        "Rust text renderers must not become map_surface from generic `changed` wording: {ls:#}"
    );
}

#[test]
fn nested_agent_bootstrap_does_not_turn_source_dirs_into_docs_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("src/cli/AGENTS.md"), "# CLI Bootstrap\n");
    write(&repo.path().join("src/render/AGENTS.md"), "# Render Bootstrap\n");
    write(
        &repo.path().join("src/cli/args.rs"),
        "pub struct CliArgs;\n",
    );
    write(
        &repo.path().join("src/render/changed.rs"),
        "pub fn render_changed_text() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "nested agent docs"]);

    let ls = run_json(repo.path(), cache.path(), &["ls", "src", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    let directory = ls["directory"].as_array().expect("directory");
    assert!(
        !directory.iter().any(|surface| {
            surface["kind"] == "docs"
                && surface["examples"]
                    .as_array()
                    .expect("examples")
                    .iter()
                    .any(|example| example == "src/cli/" || example == "src/render/")
        }),
        "nested AGENTS.md files should remain instruction/doc files, not make source dirs docs surfaces: {ls:#}"
    );
}

#[test]
fn typescript_preview_helpers_do_not_become_renderer_ui_from_substrings() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo
            .path()
            .join("apps/control-center/lib/prosteq/local-preview.ts"),
        "export function localPreview() { return { mode: 'preview' }; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "preview helper"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "apps/control-center/lib/prosteq/local-preview.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        !ls["anchor"]["roles"]
            .as_array()
            .expect("roles")
            .iter()
            .any(|role| role == "renderer_ui"),
        "`preview` contains `view`, but that substring is not UI evidence: {ls:#}"
    );
}

#[test]
fn doctor_source_only_hints_are_not_semantic_verdicts() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/plain.rs"),
        "pub fn plain_source() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "plain source"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["doctor"])
        .output()
        .expect("doctor should run");
    assert!(
        output.status.success(),
        "doctor failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Source Files With Only Generic Hints (1)")
            && markdown.contains("not an intent, ownership, or correctness verdict")
            && markdown.contains("src/plain.rs"),
        "doctor should frame source-only files as map visibility, not semantic truth: {markdown}"
    );
    assert!(
        !markdown.contains("Unclassified Source Files"),
        "doctor should not use legacy unclassified wording in agent-facing output: {markdown}"
    );
}

#[test]
fn changed_inline_changed_anchors_use_hint_wording() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/orders.service.ts"),
        "export function listOrders() { return []; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "service source"]);
    write(
        &repo.path().join("src/orders.service.ts"),
        "export function listOrders() { return ['changed']; }\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed"])
        .output()
        .expect("changed should run");
    assert!(
        output.status.success(),
        "changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        (markdown.contains("hints=") || markdown.contains("surface hints:"))
            && !markdown.contains("roles="),
        "changed output should expose hints without role-verdict wording: {markdown}"
    );
}

#[test]
fn current_public_docs_do_not_restore_role_or_coverage_verdict_wording() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut public_docs = String::new();
    for rel in ["README.md", "docs/PRODUCT.md", "docs/IMPLEMENTATION.md"] {
        public_docs.push_str(rel);
        public_docs.push('\n');
        public_docs.push_str(
            &fs::read_to_string(root.join(rel)).unwrap_or_else(|error| {
                panic!("failed to read public doc {rel}: {error}");
            }),
        );
        public_docs.push('\n');
    }
    for forbidden in [
        "mutation roles",
        "proof coverage surfaces",
        "hard proof",
        "missing direct proof",
        "proof surfaces it can justify",
        "roles prove",
        "Role patterns",
        "Unclassified Source Files",
    ] {
        assert!(
            !public_docs.contains(forbidden),
            "public docs should not restore legacy trust-boundary wording `{forbidden}`"
        );
    }
    assert!(
        public_docs.contains("Surface Hints"),
        "public docs should teach the compatibility section as Surface Hints"
    );
    assert!(
        public_docs.contains("verification surface map"),
        "public docs should define proof/proof-map as verification surface output, not proof verdicts"
    );
}
