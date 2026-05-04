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
        markdown.contains("\n## Observed\n") && markdown.contains("- kind: `runtime_state`"),
        "ls file markdown should render file metadata as compact bullets: {markdown}"
    );
    assert!(
        markdown.contains("\n## Observed Symbols\n") && markdown.contains("["),
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
fn ls_section_filters_use_stable_rfc_layers() {
    let (repo, cache) = fixture();

    let roles = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "package.json", "--section", "roles"])
        .output()
        .expect("ls roles section should run");
    assert!(
        roles.status.success(),
        "ls --section roles failed: {}",
        String::from_utf8_lossy(&roles.stderr)
    );
    let roles_markdown = String::from_utf8(roles.stdout).expect("roles markdown utf8");
    assert!(
        roles_markdown.contains("\n## Surface Hints\n") && roles_markdown.contains("manifest"),
        "ls --section roles should render only the surface-hint layer for manifests: {roles_markdown}"
    );
    assert!(
        !roles_markdown.contains("\n## Observed\n") && !roles_markdown.contains("\n## Links\n"),
        "ls --section roles should not dump observed or links layers: {roles_markdown}"
    );

    let links = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "packages/replay/src/session.ts", "--section", "links"])
        .output()
        .expect("ls links section should run");
    assert!(
        links.status.success(),
        "ls --section links failed: {}",
        String::from_utf8_lossy(&links.stderr)
    );
    let links_markdown = String::from_utf8(links.stdout).expect("links markdown utf8");
    assert!(
        links_markdown.contains("\n## Exports\n") && links_markdown.contains("\n## Imports\n"),
        "ls --section links should keep file link facts: {links_markdown}"
    );
    assert!(
        !links_markdown.contains("\n## Observed\n") && !links_markdown.contains("\n## Surface Hints\n"),
        "ls --section links should not dump observed or role layers: {links_markdown}"
    );

    let hidden = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "package.json", "--section", "hidden"])
        .output()
        .expect("ls hidden section should run");
    assert!(hidden.status.success());
    let hidden_markdown = String::from_utf8(hidden.stdout).expect("hidden markdown utf8");
    assert!(
        hidden_markdown.contains("\n## Hidden\n")
            && !hidden_markdown.contains("\n## Observed\n"),
        "ls --section hidden should render a stable hidden layer even when empty: {hidden_markdown}"
    );
}

#[test]
fn python_private_helpers_are_symbols_not_exports() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("tools/runner.py"),
        "def _candidate():\n    return 1\n\n\ndef _digest():\n    return 2\n\n\ndef public_helper():\n    return _candidate() + _digest()\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "python symbol fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", "tools/runner.py", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &json);
    let anchor = &json["anchor"];
    assert!(
        anchor["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "_candidate"
                && symbol["kind"] == "function"
                && symbol["exported"] == false),
        "Python private helpers should stay visible as symbols with exported=false: {json:#}"
    );
    assert!(
        anchor["exports"].as_array().expect("exports").is_empty(),
        "Python helpers should not be called exports without public export evidence: {json:#}"
    );

    let links = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "tools/runner.py", "--section", "links"])
        .output()
        .expect("ls links should run");
    assert!(links.status.success());
    let links_markdown = String::from_utf8(links.stdout).expect("links markdown utf8");
    assert!(
        !links_markdown.contains("\n## Exports\n"),
        "Python private helpers should not render under Exports in the links layer: {links_markdown}"
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
        markdown.contains("\n## Observed\n") && markdown.contains("[hint="),
        "ls directory markdown should render surfaces as compact blocks: {markdown}"
    );
    assert!(
        markdown.contains("count=")
            && markdown.contains("file_role_or_extension")
            && markdown.contains("medium"),
        "ls directory surface blocks should preserve count, evidence, and strength: {markdown}"
    );
    assert!(
        markdown.contains("[hint=container;")
            && markdown.contains("directory_inventory")
            && !markdown.contains("[hint=none;"),
        "ls directory containers should expose deterministic container hints, not none: {markdown}"
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
