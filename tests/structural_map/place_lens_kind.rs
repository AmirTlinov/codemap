#[test]
fn place_lens_kind_maps_existing_lens_files_without_semantic_search() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"place-lens-kind\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("src/map/lenses/runtime.rs"),
        "pub fn runtime_lens() {}\n",
    );
    write(
        &repo.path().join("src/map/lenses/contract.rs"),
        "pub fn contract_lens() {}\n",
    );
    write(
        &repo.path().join("src/map/not_a_lens.rs"),
        "pub fn helper() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "place lens fixture"]);

    let place = run_json(
        repo.path(),
        cache.path(),
        &["place", "src/map", "--kind", "lens", "--format", "json"],
    );
    assert_schema("schemas/place.schema.json", &place);
    let examples = place["existing_surfaces"]
        .as_array()
        .expect("existing surfaces")
        .first()
        .and_then(|surface| surface["examples"].as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!examples.is_empty(), "place --kind lens should expose existing lens files: {place:#}");
    assert!(
        examples.iter().any(|path| path == "src/map/lenses/runtime.rs")
            && examples.iter().any(|path| path == "src/map/lenses/contract.rs")
            && examples.iter().all(|path| path != "src/map/not_a_lens.rs"),
        "place --kind lens should expose local lens files by path convention only: {place:#}"
    );
    assert_eq!(place["requested_kind"], "lens");
    assert!(
        place["expand"]
            .as_array()
            .expect("expand")
            .iter()
            .any(|command| command == "codemap place src/map --kind lens --all"),
        "place expand should preserve the required --kind argument: {place:#}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["place", "src/map", "--kind", "lens"])
        .output()
        .expect("place markdown should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("place markdown utf8");
    assert!(
        markdown.contains("- lens surfaces already exist under `src/map`")
            && !markdown.contains("- `lens surfaces already exist under `src/map`"),
        "place local conventions should not be wrapped as one nested code literal: {markdown}"
    );
}
