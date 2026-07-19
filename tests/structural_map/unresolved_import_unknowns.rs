#[test]
fn cone_reports_unresolved_local_imports_as_typed_unknowns() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"unresolved-import-fixture","private":true}"#,
    );
    write(
        &repo.path().join("src/view.tsx"),
        "import React from 'react';\nimport { MissingPanel } from './missing-panel';\n\nexport function View() {\n  return <MissingPanel />;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/view.tsx", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "unresolved_import"
                && unknown["path"] == "src/view.tsx"
                && unknown["line_start"] == 2
                && unknown["reason"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("./missing-panel")),
        "local unresolved import should be a typed unknown with line provenance: {cone:#}"
    );
    assert!(
        cone["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .all(|unknown| !unknown["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("react")),
        "external packages should not become unresolved local-import unknowns: {cone:#}"
    );
}

#[test]
fn python_bare_import_resolves_to_the_sibling_module() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("pyproject.toml"),
        "[project]\nname = 'python-sibling-fixture'\n",
    );
    write(&repo.path().join("package/__init__.py"), "");
    write(&repo.path().join("package/sibling.py"), "VALUE = 1\n");
    write(
        &repo.path().join("package/consumer.py"),
        "import sibling\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "package/consumer.py", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["outgoing"]
            .as_array()
            .expect("outgoing")
            .iter()
            .any(|edge| {
                edge["to"] == "package/sibling.py"
                    && edge["type"] == "imports"
                    && edge["evidence"] == "resolved_import"
            }),
        "bare Python import should bind to its sibling module: {cone:#}"
    );
}
