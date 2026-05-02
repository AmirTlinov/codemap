#[test]
fn runtime_markdown_renders_routes_and_env_as_compact_blocks() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/.env.example"),
        "RUNTIME_MARKDOWN_TOKEN=\n",
    );
    write(
        &repo.path().join("packages/app/src/runtime-markdown.ts"),
        "const token = process.env.RUNTIME_MARKDOWN_TOKEN;\nrouter.get('/runtime-markdown', runtimeMarkdownHandler);\nexport function runtimeMarkdownHandler() {\n  return token;\n}\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["runtime", "packages/app/src/runtime-markdown.ts"])
        .output()
        .expect("runtime markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("`GET /runtime-markdown` -> `packages/app/src/runtime-markdown.ts`"),
        "runtime route markdown should keep method/path/file in a compact block: {markdown}"
    );
    assert!(
        markdown.contains("`RUNTIME_MARKDOWN_TOKEN` used by `packages/app/src/runtime-markdown.ts`")
            && markdown.contains("declaration: `packages/app/.env.example`"),
        "runtime env markdown should keep used_by and declaration provenance: {markdown}"
    );
    assert!(
        markdown.contains("`packages/app/src/runtime-markdown.ts:2`")
            && markdown.contains("`packages/app/src/runtime-markdown.ts:1`"),
        "runtime route/env markdown should keep line locations: {markdown}"
    );
    assert!(
        !markdown.contains("| Method | Path | File | Evidence | Strength |")
            && !markdown.contains("| Name | Used By | Declaration | Evidence | Strength | Where |"),
        "runtime markdown should not return to route/env table spam: {markdown}"
    );
}
