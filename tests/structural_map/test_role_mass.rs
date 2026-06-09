#[test]
fn rust_test_prefix_source_does_not_create_test_or_proof_mass() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"test-prefix-source\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(&repo.path().join("src/lib.rs"), "pub mod test_edges;\n");
    write(
        &repo.path().join("src/test_edges.rs"),
        "pub fn production_helper() -> bool { true }\n",
    );
    write(
        &repo.path().join("src/test_parser.py"),
        "def test_parser():\n    assert True\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "test prefix source fixture"]);

    let source = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/test_edges.rs", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &source);
    assert_eq!(
        source["anchor"]["kind"], "source",
        "Rust src/test_edges.rs should remain production source, not a test role: {source:#}"
    );
    let roles = source["anchor"]["roles"].as_array().expect("roles");
    assert!(
        !roles.iter().any(|role| role == "test"),
        "a Rust production helper must not gain test mass from the `test_` token: {source:#}"
    );
    assert!(
        !source["edges"].as_array().expect("edges").iter().any(|edge| {
            edge["type"] == "tests"
                && edge["from"] == "src/test_edges.rs"
                && edge["to"] == "src/test_edges.rs"
        }),
        "a source file must not create a self proof edge from its own name: {source:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "src/test_edges.rs", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        !proof["proofs"].as_array().expect("proofs").iter().any(|surface| {
            surface["path"] == "src/test_edges.rs" && surface["evidence"] == "test_name"
        }),
        "proof should not emit a self soft sensor for a production source file: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "direct_test_import_not_found"),
        "without a real direct sensor, proof must keep the direct proof gap visible: {proof:#}"
    );

    let python_test = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/test_parser.py", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &python_test);
    assert_eq!(
        python_test["anchor"]["kind"], "test",
        "Python test_*.py remains a language-native test convention: {python_test:#}"
    );
}
