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
fn css_imports_create_deterministic_style_edges() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/landing-shell.css"),
        "/* @import url('./commented.css'); */\n/* @import './landing-hero.css'; */\n@import url('./landing-hero.css');\n@import './landing-footer.css';\n",
    );
    write(
        &repo.path().join("packages/app/src/landing-hero.css"),
        ".hero {\n  color: red;\n}\n",
    );
    write(
        &repo.path().join("packages/app/src/landing-footer.css"),
        ".footer {\n  color: blue;\n}\n",
    );
    write(
        &repo.path().join("packages/app/src/commented.css"),
        ".commented {\n  color: black;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "css import fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/landing-hero.css",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let incoming = cone["incoming"].as_array().expect("incoming");
    let shell_edge = incoming
        .iter()
        .find(|edge| {
            edge["from"] == "packages/app/src/landing-shell.css"
                && edge["to"] == "packages/app/src/landing-hero.css"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"
        })
        .unwrap_or_else(|| {
            panic!("CSS @import should create an exact style reverse edge: {cone:#}")
        });
    let location_lines = shell_edge["locations"]
        .as_array()
        .expect("locations")
        .iter()
        .map(|location| location["line_start"].as_u64())
        .collect::<Vec<_>>();
    assert_eq!(
        location_lines,
        vec![Some(3)],
        "CSS @import line evidence must ignore commented imports for the same target: {cone:#}"
    );

    let commented = run_json(
        repo.path(),
        cache.path(),
        &["cone", "packages/app/src/commented.css", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &commented);
    assert!(
        commented["incoming"]
            .as_array()
            .expect("incoming")
            .is_empty(),
        "commented CSS @import must not become a hard style edge: {commented:#}"
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
