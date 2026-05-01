#[test]
fn rust_include_and_file_module_edges_are_structural() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"rust-include-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("src/map.rs"),
        "mod graph_lens;\ninclude!(\"map/status.rs\");\n\npub fn entry() -> &'static str {\n    status()\n}\n",
    );
    write(
        &repo.path().join("src/commented.rs"),
        "// include!(\"map/comment_only.rs\");\n/* outer\n   /* inner include!(\"map/comment_only.rs\"); */\n*/\npub const DOC: &str = \"include!(\\\"map/comment_only.rs\\\")\";\npub const RAW: &str = r#\"include!(\"map/comment_only.rs\")\"#;\n",
    );
    write(
        &repo.path().join("src/lifetime.rs"),
        "pub fn hold<'a>(value: &'a str) -> &'a str {\n    value\n}\ninclude!(\"map/lifetime_include.rs\");\n",
    );
    write(
        &repo.path().join("src/raw_include.rs"),
        "include!(r#\"map/raw_include.rs\"#);\n",
    );
    write(
        &repo.path().join("src/map/status.rs"),
        "pub fn status() -> &'static str {\n    \"ok\"\n}\n",
    );
    write(
        &repo.path().join("src/map/comment_only.rs"),
        "pub fn comment_only() -> &'static str {\n    \"not imported\"\n}\n",
    );
    write(
        &repo.path().join("src/map/lifetime_include.rs"),
        "pub fn lifetime_include() -> &'static str {\n    \"included\"\n}\n",
    );
    write(
        &repo.path().join("src/map/raw_include.rs"),
        "pub fn raw_include() -> &'static str {\n    \"included\"\n}\n",
    );
    write(
        &repo.path().join("src/map/graph_lens.rs"),
        "pub fn graph_lens() -> &'static str {\n    \"graph\"\n}\n",
    );
    write(
        &repo.path().join("src/graph_lens.rs"),
        "pub fn wrong_sibling() -> &'static str {\n    \"wrong\"\n}\n",
    );
    write(
        &repo.path().join("graph_lens.rs"),
        "pub fn wrong_root() -> &'static str {\n    \"wrong\"\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let include_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/map/status.rs", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &include_cone);
    assert!(
        include_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "src/map.rs"
                && edge["to"] == "src/map/status.rs"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"),
        "Rust include! targets should be structural incoming edges: {include_cone:#}"
    );

    let mod_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/map/graph_lens.rs", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &mod_cone);
    assert!(
        mod_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "src/map.rs"
                && edge["to"] == "src/map/graph_lens.rs"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"),
        "Rust file-module targets should be structural incoming edges: {mod_cone:#}"
    );
    assert!(
        mod_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["to"] != "src/graph_lens.rs"),
        "file-backed Rust modules should prefer child module files over root siblings: {mod_cone:#}"
    );
    let root_shadow_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "graph_lens.rs", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &root_shadow_cone);
    assert!(
        root_shadow_cone["incoming"]
            .as_array()
            .expect("root shadow incoming")
            .iter()
            .all(|edge| edge["from"] != "src/map.rs"),
        "file-backed Rust modules must not resolve to repo-root shadow files: {root_shadow_cone:#}"
    );

    let commented_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/map/comment_only.rs", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &commented_cone);
    assert!(
        commented_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"] != "src/commented.rs"),
        "Rust include! text inside comments or strings must not create structural edges: {commented_cone:#}"
    );

    let lifetime_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/map/lifetime_include.rs", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &lifetime_cone);
    assert!(
        lifetime_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "src/lifetime.rs"
                && edge["to"] == "src/map/lifetime_include.rs"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"),
        "Rust lifetimes before include! must not hide real structural include edges: {lifetime_cone:#}"
    );

    let raw_include_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/map/raw_include.rs", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &raw_include_cone);
    assert!(
        raw_include_cone["incoming"]
            .as_array()
            .expect("raw include incoming")
            .iter()
            .any(|edge| edge["from"] == "src/raw_include.rs"
                && edge["to"] == "src/map/raw_include.rs"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"),
        "Rust raw-string include! should create structural include edges: {raw_include_cone:#}"
    );
}
