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

#[test]
fn ls_and_cone_treat_imported_assets_as_first_class_non_code_anchors() {
    let (repo, cache) = fixture();
    std::fs::write(
        repo.path().join("packages/app/src/logo.png"),
        [0x89, b'P', b'N', b'G', 0, 1, 2],
    )
    .expect("write png");
    write(
        &repo.path().join("packages/app/src/view.ts"),
        "import logo from './logo.png';\n\nexport function view() { return logo; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "asset fixture"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/src/logo.png", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["mode"], "file");
    assert_eq!(ls["anchor"]["kind"], "asset");
    assert_eq!(ls["anchor"]["language"], "asset");
    assert_eq!(ls["anchor"]["lines"], 0);
    assert!(ls["anchor"]["symbols"].as_array().expect("symbols").is_empty());
    assert!(
        ls["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/view.ts"
                && edge["to"] == "packages/app/src/logo.png"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"),
        "asset anchor should preserve deterministic import references without fake symbols: {ls:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "packages/app/src/logo.png", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(cone["anchor"]["kind"], "asset");
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/view.ts"
                && edge["to"] == "packages/app/src/logo.png"
                && edge["type"] == "imported_by"),
        "asset cone should show deterministic import users: {cone:#}"
    );
}

#[test]
fn ls_indexes_large_assets_as_metadata_without_fake_text_content() {
    let (repo, cache) = fixture();
    std::fs::create_dir_all(repo.path().join("packages/app/public")).expect("public dir");
    std::fs::write(
        repo.path().join("packages/app/public/hero.png"),
        vec![0x89; 1_000_001],
    )
    .expect("write large png");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "large asset fixture"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/public/hero.png", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["mode"], "file");
    assert_eq!(ls["anchor"]["kind"], "asset");
    assert_eq!(ls["anchor"]["language"], "asset");
    assert_eq!(ls["anchor"]["lines"], 0);
    assert!(ls["anchor"]["symbols"].as_array().expect("symbols").is_empty());
}

#[test]
fn ls_treats_snapshots_as_first_class_non_code_anchors() {
    let (repo, cache) = fixture();
    write(
        &repo.path()
            .join("packages/app/src/__snapshots__/view.test.ts.snap"),
        "exports[`view renders 1`] = `<button>Save</button>`;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "snapshot fixture"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "packages/app/src/__snapshots__/view.test.ts.snap",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["mode"], "file");
    assert_eq!(ls["anchor"]["kind"], "snapshot");
    assert_eq!(ls["anchor"]["language"], "snapshot");
    assert_eq!(ls["anchor"]["lines"], 1);
    assert!(
        ls["anchor"]["roles"]
            .as_array()
            .expect("roles")
            .iter()
            .any(|role| role == "snapshot"),
        "snapshot role should be explicit: {ls:#}"
    );
}
