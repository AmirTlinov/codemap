#[test]
fn ls_and_cone_treat_stylesheets_as_first_class_non_code_anchors() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/styles.css"),
        ".button {\n  color: red;\n}\n",
    );
    write(
        &repo.path().join("packages/app/src/view.ts"),
        "import './styles.css';\n\nexport function view() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "style fixture"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/styles.css", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["mode"], "file");
    assert_eq!(ls["anchor"]["kind"], "style");
    assert_eq!(ls["anchor"]["language"], "style");
    assert_eq!(ls["anchor"]["lines"], 3);
    assert!(
        ls["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/view.ts"
                && edge["to"] == "packages/app/src/styles.css"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"),
        "stylesheet anchor should preserve deterministic TS import references without fake symbols: {ls:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "packages/app/src/styles.css", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/view.ts"
                && edge["to"] == "packages/app/src/styles.css"
                && edge["type"] == "imported_by"),
        "stylesheet cone should show deterministic import users: {cone:#}"
    );
}
