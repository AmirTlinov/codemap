#[test]
fn rust_qualified_module_calls_reach_reexported_owner_and_next_runtime_hop() {
    let repo = TempDir::new().expect("Rust qualified cone repo");
    let cache = TempDir::new().expect("Rust qualified cone cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"qualified-cone\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &repo.path().join("src/lib.rs"),
        "mod consumer;\nmod facade;\nmod runtime;\n",
    );
    write(
        &repo.path().join("src/facade.rs"),
        "mod owner;\npub(crate) use owner::{target, Thing};\n",
    );
    write(
        &repo.path().join("src/facade/owner.rs"),
        "use crate::runtime;\npub(crate) struct Thing;\nimpl Thing { pub(crate) fn target() {} }\npub(crate) fn target() { runtime::execute(); }\n",
    );
    write(
        &repo.path().join("src/runtime.rs"),
        "pub(crate) fn execute() {}\n",
    );
    write(
        &repo.path().join("src/consumer.rs"),
        "use crate::facade;\nuse crate::facade::Thing as facade_alias;\npub(crate) fn run() { facade::target(); }\npub(crate) fn associated() { facade_alias::target(); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "qualified module cone fixture"]);

    let direct = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/consumer.rs#run",
            "--depth",
            "1",
            "--all",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &direct);
    assert!(direct["outgoing"]
        .as_array()
        .expect("direct outgoing")
        .iter()
        .any(|edge| {
            edge["to"] == "src/facade/owner.rs#target"
                && edge["evidence"] == "reexported_module_symbol_in_symbol_body"
                && edge["locations"][0]["line_start"] == 3
        }), "qualified facade call should reach its reexported owner: {direct:#}");
    assert!(direct["outgoing"]
        .as_array()
        .expect("direct outgoing")
        .iter()
        .all(|edge| edge["to"] != "src/runtime.rs#execute"));

    let transitive = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/consumer.rs#run",
            "--depth",
            "2",
            "--all",
            "--format",
            "json",
        ],
    );
    assert!(transitive["outgoing"]
        .as_array()
        .expect("transitive outgoing")
        .iter()
        .any(|edge| {
            edge["from"] == "src/facade/owner.rs#target"
                && edge["to"] == "src/runtime.rs#execute"
        }), "depth two should cross the runtime owner edge: {transitive:#}");

    let associated = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/consumer.rs#associated",
            "--all",
            "--format",
            "json",
        ],
    );
    assert!(associated["outgoing"]
        .as_array()
        .expect("associated outgoing")
        .iter()
        .all(|edge| edge["to"] != "src/facade/owner.rs#target"),
        "a type alias must not become a module edge: {associated:#}");
}
