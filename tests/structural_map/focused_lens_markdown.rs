#[test]
fn diff_map_changed_symbols_render_as_compact_blocks() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/symbol-delta.ts"),
        "export const symbolDelta = true;\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["diff-map", "--files", "packages/replay/src/symbol-delta.ts"])
        .output()
        .expect("diff-map markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("- `symbolDelta` in `packages/replay/src/symbol-delta.ts:1`"),
        "changed symbol markdown should keep symbol and line provenance: {markdown}"
    );
    assert!(
        !markdown.contains("| Path | Name | Change | Line |"),
        "changed symbol markdown should not return to table spam: {markdown}"
    );
}

#[test]
fn boundary_map_package_edges_render_as_compact_blocks() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["boundary-map", "."])
        .output()
        .expect("boundary-map markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("`packages/app/package.json` --@fixture/replay--> `packages/replay/package.json`"),
        "package edge markdown should keep from/dependency/to evidence: {markdown}"
    );
    assert!(
        !markdown.contains("| From | Dependency | To | Evidence |"),
        "package edge markdown should not return to table spam: {markdown}"
    );
}

#[test]
fn boundary_map_package_edges_keep_workspace_manifest_in_markdown() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"workspace-edge-root\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("fixtures/Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"lib\"]\n\n[workspace.dependencies]\nfixture-lib = { path = \"lib\" }\n",
    );
    write(
        &repo.path().join("fixtures/app/Cargo.toml"),
        "[package]\nname = \"fixture-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nfixture-lib = { workspace = true }\n",
    );
    write(
        &repo.path().join("fixtures/lib/Cargo.toml"),
        "[package]\nname = \"fixture-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(&repo.path().join("fixtures/app/src/main.rs"), "fn main() {}\n");
    write(
        &repo.path().join("fixtures/lib/src/lib.rs"),
        "pub fn value() -> bool { true }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "workspace package edge fixture"]);
    write(
        &repo.path().join("fixtures/Cargo.toml"),
        "[workspace]\nmembers = [\"app\", \"lib\"]\n\n[workspace.dependencies]\nfixture-lib = { path = \"lib\" }\n# touched\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["boundary-map", ".", "--changed"])
        .output()
        .expect("boundary-map markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("workspace: `fixtures/Cargo.toml`"),
        "package edge markdown should preserve workspace manifest provenance: {markdown}"
    );
}

#[test]
fn flow_steps_render_as_compact_blocks() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/server.ts"),
        "import { seek } from '@fixture/replay';\n\nexport function server() {\n  return seek(1).frame;\n}\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["flow", "packages/app/src/server.ts"])
        .output()
        .expect("flow markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("- 0. `packages/app/src/server.ts` [file_anchor; exact_file_anchor]"),
        "flow step markdown should keep ordered anchor, kind, evidence, and where: {markdown}"
    );
    assert!(
        !markdown.contains("| # | Anchor | Kind | Evidence | Where |"),
        "flow steps should not return to table spam: {markdown}"
    );
}
