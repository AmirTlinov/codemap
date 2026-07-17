#[test]
fn rust_symbol_cone_follows_typed_methods_and_named_reexports_without_guessing() {
    let repo = tempfile::TempDir::new().expect("Rust typed method repo");
    let cache = tempfile::TempDir::new().expect("Rust typed method cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"typed-method\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &repo.path().join("src/lib.rs"),
        "mod entry;\nmod facade;\nmod owner;\nmod record;\n",
    );
    write(
        &repo.path().join("src/facade.rs"),
        "pub(crate) use crate::owner::{execute, Runtime};\n",
    );
    write(
        &repo.path().join("src/owner.rs"),
        "use crate::record;\npub(crate) struct Runtime;\nimpl Runtime {\n    pub(crate) fn run(&mut self) { record::persist(); }\n}\npub(crate) fn execute() { record::persist(); }\n",
    );
    write(
        &repo.path().join("src/record.rs"),
        "pub(crate) fn persist() {}\n",
    );
    write(
        &repo.path().join("src/entry.rs"),
        "use crate::facade::{execute, Runtime};\npub(crate) fn start(mut runtime: Runtime, unknown: impl Runnable) {\n    runtime.run();\n    unknown.run();\n    execute();\n}\ntrait Runnable { fn run(&self); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "typed method fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/entry.rs#start",
            "--depth",
            "2",
            "--all",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let outgoing = cone["outgoing"].as_array().expect("outgoing");
    assert!(outgoing.iter().any(|edge| {
        edge["from"] == "src/entry.rs#start"
            && edge["to"] == "src/owner.rs#run"
            && edge["evidence"] == "typed_receiver_method_in_symbol_body"
            && edge["locations"][0]["line_start"] == 3
    }), "typed receiver should resolve to its impl method: {cone:#}");
    assert!(outgoing.iter().any(|edge| {
        edge["from"] == "src/entry.rs#start"
            && edge["to"] == "src/owner.rs#execute"
            && edge["evidence"] == "reexported_symbol_in_symbol_body"
    }), "named barrel export should resolve to its owner: {cone:#}");
    assert!(outgoing.iter().any(|edge| {
        edge["from"] == "src/owner.rs#run" && edge["to"] == "src/record.rs#persist"
    }), "depth two should continue from the resolved method: {cone:#}");
    assert!(outgoing.iter().all(|edge| {
        !(edge["from"] == "src/entry.rs#start" && edge["to"] == "src/entry.rs#run")
    }), "opaque impl-trait receiver must not be guessed: {cone:#}");
}

#[test]
fn cli_symbol_cone_exposes_static_dogfood_consumer_without_comment_matches() {
    let repo = tempfile::TempDir::new().expect("CLI consumer repo");
    let cache = tempfile::TempDir::new().expect("CLI consumer cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"cli-consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("src/main.rs"), "mod cli;\nfn main() {}\n");
    write(
        &repo.path().join("src/cli.rs"),
        "pub(crate) fn proof() {}\n",
    );
    write(
        &repo.path().join("scripts/dogfood-cli.sh"),
        "#!/usr/bin/env bash\n# codemap proof changed is documentation only\nrun_probe_command target proof_changed proof changed\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "CLI consumer fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/cli.rs#proof", "--all", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(cone["proof"].as_array().expect("proof").iter().any(|edge| {
        edge["from"] == "scripts/dogfood-cli.sh"
            && edge["to"] == "src/cli.rs#proof"
            && edge["type"] == "invokes_cli_command"
            && edge["evidence"] == "static_cli_command_consumer"
            && edge["locations"][0]["line_start"] == 3
    }), "static dogfood invocation should remain visible: {cone:#}");
}
