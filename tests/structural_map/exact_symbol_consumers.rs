#[test]
fn rust_grouped_alias_module_reexport_and_include_consumers_are_exact() {
    let repo = TempDir::new().expect("Rust consumer repo");
    let cache = TempDir::new().expect("Rust consumer cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"rust-consumer-truth\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &repo.path().join("src/lib.rs"),
        "mod consumer;\nmod include_host;\nmod inner;\nmod not_module_alias;\npub use inner::Thing;\n",
    );
    write(
        &repo.path().join("src/inner.rs"),
        "pub struct Thing;\npub fn helper() -> usize { 1 }\n",
    );
    write(
        &repo.path().join("src/consumer.rs"),
        "use crate::{inner as owner, Thing as Alias};\n\npub fn build() -> Alias { let _ = owner::helper(); Alias }\n",
    );
    write(
        &repo.path().join("src/include_host.rs"),
        "include!(\"included.rs\");\npub fn call_included() -> usize { included_helper() }\n",
    );
    write(
        &repo.path().join("src/included.rs"),
        "pub fn included_helper() -> usize { 2 }\n",
    );
    write(
        &repo.path().join("src/unrelated.rs"),
        "pub fn unrelated() { let Thing = 3; let _ = Thing; }\n",
    );
    write(
        &repo.path().join("src/not_module_alias.rs"),
        "use crate::inner::Thing as owner;\npub fn false_qualified() { owner::helper(); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "Rust exact consumer fixture"]);

    let thing = run_json(repo.path(), cache.path(), &["where", "Thing", "--format", "json"]);
    assert_schema("schemas/where.schema.json", &thing);
    assert_consumer(
        &thing,
        "src/consumer.rs",
        "reexported_symbol_reference",
        3,
    );
    assert!(
        thing["definitions"][0]["consumers"]
            .as_array()
            .expect("Thing consumers")
            .iter()
            .all(|edge| edge["from"] != "src/unrelated.rs"),
        "same-name local without an import path is not a consumer: {thing:#}"
    );

    let helper = run_json(repo.path(), cache.path(), &["where", "helper", "--format", "json"]);
    assert_consumer(
        &helper,
        "src/consumer.rs",
        "imported_symbol_reference",
        3,
    );
    assert!(
        helper["definitions"][0]["consumers"]
            .as_array()
            .expect("helper consumers")
            .iter()
            .all(|edge| edge["from"] != "src/not_module_alias.rs"),
        "a symbol alias must not be treated as a module alias: {helper:#}"
    );

    let included = run_json(
        repo.path(),
        cache.path(),
        &["where", "included_helper", "--format", "json"],
    );
    assert_consumer(
        &included,
        "src/include_host.rs",
        "included_symbol_reference",
        2,
    );
    assert_eq!(included["definitions"][0]["consumers_total"]["observed"], 1);
    assert_eq!(included["definitions"][0]["consumers_total"]["closure"], "open");
    assert!(
        included["definitions"][0]["consumers_total"]["reasons"]
            .as_array()
            .expect("include reasons")
            .iter()
            .any(|reason| reason == "rust_include_flow"),
        "static lower bound keeps the unclosed include horizon: {included:#}"
    );
}

#[test]
fn python_and_go_static_import_consumers_keep_alias_locations() {
    let python_repo = TempDir::new().expect("Python consumer repo");
    let python_cache = TempDir::new().expect("Python consumer cache");
    git(python_repo.path(), &["init", "-q"]);
    git(python_repo.path(), &["config", "user.email", "a@example.com"]);
    git(python_repo.path(), &["config", "user.name", "a"]);
    write(
        &python_repo.path().join("pyproject.toml"),
        "[project]\nname = \"python-consumer-truth\"\nversion = \"0.1.0\"\n",
    );
    write(
        &python_repo.path().join("src/owner.py"),
        "def shared_value():\n    return 1\n",
    );
    write(
        &python_repo.path().join("src/consumer.py"),
        "from owner import shared_value as use_value\n\ndef run():\n    return use_value()\n",
    );
    git(python_repo.path(), &["add", "."]);
    git(python_repo.path(), &["commit", "-qm", "Python exact consumer fixture"]);
    let python = run_json(
        python_repo.path(),
        python_cache.path(),
        &["where", "shared_value", "--format", "json"],
    );
    assert_consumer(
        &python,
        "src/consumer.py",
        "imported_symbol_reference",
        4,
    );

    let go_repo = TempDir::new().expect("Go consumer repo");
    let go_cache = TempDir::new().expect("Go consumer cache");
    git(go_repo.path(), &["init", "-q"]);
    git(go_repo.path(), &["config", "user.email", "a@example.com"]);
    git(go_repo.path(), &["config", "user.name", "a"]);
    write(&go_repo.path().join("go.mod"), "module example.com/consumer\n\ngo 1.24\n");
    write(
        &go_repo.path().join("owner/owner.go"),
        "package owner\n\nfunc SharedValue() int { return 1 }\n",
    );
    write(
        &go_repo.path().join("cmd/app/main.go"),
        "package main\n\nimport value \"example.com/consumer/owner\"\n\nfunc main() { _ = value.SharedValue() }\n",
    );
    git(go_repo.path(), &["add", "."]);
    git(go_repo.path(), &["commit", "-qm", "Go exact consumer fixture"]);
    let go = run_json(
        go_repo.path(),
        go_cache.path(),
        &["where", "SharedValue", "--format", "json"],
    );
    assert_consumer(
        &go,
        "cmd/app/main.go",
        "imported_symbol_reference",
        5,
    );
}

#[test]
fn javascript_namespace_import_is_an_exact_static_consumer() {
    let repo = TempDir::new().expect("JS namespace repo");
    let cache = TempDir::new().expect("JS namespace cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("package.json"), "{\"name\":\"namespace-consumer\"}\n");
    write(&repo.path().join("src/owner.ts"), "export function sharedValue() { return 1; }\n");
    write(
        &repo.path().join("src/consumer.ts"),
        "import * as owner from './owner';\nexport const result = owner.sharedValue();\n",
    );
    write(
        &repo.path().join("src/shadowed.ts"),
        "import * as owner from './owner';\nexport function fake(owner: { sharedValue(): number }) { return owner.sharedValue(); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "JS namespace consumer fixture"]);
    let report = run_json(
        repo.path(),
        cache.path(),
        &["where", "sharedValue", "--format", "json"],
    );
    assert_consumer(
        &report,
        "src/consumer.ts",
        "imported_symbol_reference",
        2,
    );
    assert!(
        report["definitions"][0]["consumers"]
            .as_array()
            .expect("namespace consumers")
            .iter()
            .all(|edge| edge["from"] != "src/shadowed.ts"),
        "a shadowed namespace alias must not create a consumer: {report:#}"
    );
}

fn assert_consumer(report: &Value, path: &str, evidence: &str, line: u64) {
    let edge = report["definitions"][0]["consumers"]
        .as_array()
        .expect("consumer edges")
        .iter()
        .find(|edge| edge["from"] == path && edge["evidence"] == evidence)
        .unwrap_or_else(|| panic!("missing {evidence} from {path}: {report:#}"));
    assert_eq!(edge["locations"][0]["line_start"], line, "{report:#}");
}
