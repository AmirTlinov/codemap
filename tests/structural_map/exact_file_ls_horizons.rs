// Responsibility: exact-file-ls-relationship-horizon-contract
const EXACT_FILE_LS_ANCHOR: &str = "src/owner.ts";

#[test]
fn exact_file_ls_readable_and_json_share_relationship_horizons() {
    let repo = exact_file_ls_fixture();
    let readable_cache = TempDir::new().expect("file ls readable cache");
    let json_cache = TempDir::new().expect("file ls json cache");
    let readable = run_markdown(
        repo.path(),
        readable_cache.path(),
        &["ls", EXACT_FILE_LS_ANCHOR, "--limit", "1"],
    );
    let json = run_json(
        repo.path(),
        json_cache.path(),
        &[
            "ls",
            EXACT_FILE_LS_ANCHOR,
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &json);
    assert_eq!(json["schema_version"], "15", "{json:#}");
    assert_eq!(json["edges"].as_array().expect("file edges").len(), 4);
    assert_eq!(
        json["edges"]
            .as_array()
            .expect("file edges")
            .iter()
            .map(|edge| edge["type"].as_str().expect("edge type"))
            .collect::<std::collections::BTreeSet<_>>(),
        ["imported_by", "imports", "tests"].into_iter().collect(),
        "test imports must live only in verification: {json:#}"
    );
    assert!(
        json["hidden"].as_array().expect("hidden").is_empty(),
        "full JSON must not carry detached edge accounting: {json:#}"
    );

    let ledger = &json["observations"];
    assert_eq!(ledger["horizons"].as_array().expect("horizons").len(), 4);
    for (group, observed, closure) in [
        ("imports", 1, "closed"),
        ("consumers", 2, "open"),
        ("verification", 1, "open"),
        ("symbols", 1, "closed"),
    ] {
        let item = horizon(ledger, group);
        assert_eq!(item["count"]["observed"], observed, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], closure, "{group}: {json:#}");
        assert_eq!(item["shown"], observed, "{group}: {json:#}");
        assert_eq!(item["hidden"], 0, "{group}: {json:#}");
        assert_horizon_certificate_resolves(ledger, item);
        assert!(
            readable
                .lines()
                .any(|line| line.starts_with(&format!("- {group}:"))),
            "readable output must expose the {group} horizon: {readable}"
        );
    }
    let rows = readable
        .lines()
        .filter(|line| {
            line.contains("shown=")
                && ["- imports:", "- consumers:", "- verification:", "- symbols:"]
                .iter()
                .any(|prefix| line.starts_with(prefix))
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 4, "{readable}");
    assert_eq!(
        rows.iter()
            .filter(|line| !line.starts_with("- symbols:") && line.contains("shown=1"))
            .count(),
        3,
        "each populated relationship group gets its own limit-one representation: {readable}"
    );
    assert_eq!(
        rows.iter().filter(|line| line.contains("hidden=1")).count(),
        1,
        "only the saturated relationship group owns a hidden remainder: {readable}"
    );
    assert!(!readable.contains("edges hidden by limit"), "{readable}");
}

#[test]
fn dynamic_and_unresolved_file_imports_keep_the_import_horizon_open() {
    let repo = exact_file_ls_fixture();
    let cache = TempDir::new().expect("dynamic file ls cache");
    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/dynamic.ts", "--format", "json"],
    );
    let imports = horizon(&json["observations"], "imports");
    assert_eq!(imports["count"]["observed"], 0, "{json:#}");
    assert_eq!(imports["count"]["closure"], "open", "{json:#}");
    let reasons = imports["count"]["reasons"]
        .as_array()
        .expect("import reasons");
    assert!(reasons.iter().any(|reason| reason == "dynamic_import_flow"));
    assert!(reasons.iter().any(|reason| reason == "incomplete_traversal"));
    let id = imports["count"]["certificate_id"].as_str().expect("id");
    let certificate = &json["observations"]["certificates"][id];
    assert!(!certificate["dynamic_stops"].as_array().unwrap().is_empty());
    assert!(!certificate["unresolved_stops"].as_array().unwrap().is_empty());
}

#[test]
fn dynamic_rust_include_keeps_the_file_import_horizon_open() {
    let repo = TempDir::new().expect("dynamic include file ls repo");
    let cache = TempDir::new().expect("dynamic include file ls cache");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname='dynamic-include'\nversion='0.1.0'\n",
    );
    write(
        &repo.path().join("src/lib.rs"),
        "const PART: &str = \"part.rs\";\ninclude!(PART);\n",
    );
    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/lib.rs", "--format", "json"],
    );
    let imports = horizon(&json["observations"], "imports");
    assert_eq!(imports["count"]["closure"], "open", "{json:#}");
    assert!(
        imports["count"]["reasons"]
            .as_array()
            .expect("include reasons")
            .iter()
            .any(|reason| reason == "rust_include_flow"),
        "{json:#}"
    );
}

#[test]
fn unavailable_exact_file_keeps_every_relationship_group_unavailable() {
    let repo = exact_file_ls_fixture();
    write(&repo.path().join("src/unavailable.ts"), &"x".repeat(901_000));
    let cache = TempDir::new().expect("unavailable file ls cache");
    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/unavailable.ts", "--format", "json"],
    );
    assert_eq!(json["mode"], "file", "the indexed path remains an anchor: {json:#}");
    assert!(json["edges"].as_array().expect("edges").is_empty());
    for group in ["imports", "consumers", "verification", "symbols"] {
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {json:#}");
        assert_eq!(
            item["count"]["closure"], "unavailable",
            "{group}: {json:#}"
        );
        assert_eq!(
            item["count"]["reasons"],
            serde_json::json!(["unsupported_construct"]),
            "{group}: {json:#}"
        );
        assert_horizon_certificate_resolves(&json["observations"], item);
    }
}

#[test]
fn exact_file_ls_cache_preserves_the_complete_projection() {
    let repo = exact_file_ls_fixture();
    let cache = TempDir::new().expect("file ls warm cache");
    let args = [
        "ls",
        EXACT_FILE_LS_ANCHOR,
        "--limit",
        "1",
        "--format",
        "json",
    ];
    let cold = run_json(repo.path(), cache.path(), &args);
    let warm = run_json(repo.path(), cache.path(), &args);
    assert_eq!(warm, cold, "warm exact-file LS must preserve the full ledger");
    let artifact: Value = serde_json::from_str(
        &fs::read_to_string(lens_artifact_path(cache.path(), "ls-current.json"))
            .expect("cached exact-file ls artifact"),
    )
    .expect("exact-file ls artifact json");
    assert_eq!(
        artifact["report"]["observations"]["horizons"]
            .as_array()
            .expect("cached horizons")
            .len(),
        4,
        "{artifact:#}"
    );
}

#[test]
fn exact_file_symbol_catalog_is_bounded_in_readable_and_complete_in_json() {
    let repo = exact_file_ls_fixture();
    let readable = run_markdown(
        repo.path(),
        TempDir::new().expect("catalog readable cache").path(),
        &["ls", "src/catalog.rs", "--limit", "2"],
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("catalog json cache").path(),
        &["ls", "src/catalog.rs", "--limit", "2", "--format", "json"],
    );
    let symbols = horizon(&json["observations"], "symbols");
    assert_eq!(symbols["count"]["observed"], 4, "{json:#}");
    assert_eq!(symbols["count"]["closure"], "closed", "{json:#}");
    assert_eq!(symbols["shown"], 4, "{json:#}");
    assert_eq!(
        json["anchor"]["symbols"].as_array().expect("symbols").len(),
        4,
        "machine projection must contain the complete catalog: {json:#}"
    );
    assert!(json["hidden"].as_array().expect("hidden").is_empty(), "{json:#}");
    let row = readable
        .lines()
        .find(|line| line.starts_with("- symbols:") && line.contains("shown="))
        .expect("readable symbols horizon");
    assert!(row.contains("counted(4)"), "{readable}");
    assert!(row.contains("shown=2 hidden=2"), "{readable}");
    assert!(!readable.contains("symbols hidden by limit"), "{readable}");
}

#[test]
fn supported_empty_file_proves_an_empty_symbol_catalog() {
    let repo = exact_file_ls_fixture();
    let json = run_json(
        repo.path(),
        TempDir::new().expect("empty symbol cache").path(),
        &["ls", "src/empty.ts", "--format", "json"],
    );
    let symbols = horizon(&json["observations"], "symbols");
    assert_eq!(symbols["count"]["observed"], 0, "{json:#}");
    assert_eq!(symbols["count"]["closure"], "closed", "{json:#}");
    assert!(json["anchor"]["symbols"].as_array().expect("symbols").is_empty());
}

#[test]
fn unsupported_file_keeps_the_symbol_catalog_unavailable() {
    let repo = exact_file_ls_fixture();
    let json = run_json(
        repo.path(),
        TempDir::new().expect("unsupported symbol cache").path(),
        &["ls", "README.md", "--format", "json"],
    );
    let symbols = horizon(&json["observations"], "symbols");
    assert_eq!(symbols["count"]["observed"], 0, "{json:#}");
    assert_eq!(symbols["count"]["closure"], "unavailable", "{json:#}");
    assert_eq!(
        symbols["count"]["reasons"],
        serde_json::json!(["unsupported_language"]),
        "{json:#}"
    );
}

fn exact_file_ls_fixture() -> TempDir {
    let repo = TempDir::new().expect("exact file ls repo");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"exact-file-ls","private":true}"#,
    );
    write(&repo.path().join("src/dependency.ts"), "export const value = 1;\n");
    write(
        &repo.path().join("src/owner.ts"),
        "import { value } from './dependency';\nexport function owner() { return value; }\n",
    );
    write(
        &repo.path().join("src/consumer.ts"),
        "import { owner } from './owner';\nexport const consumed = owner();\n",
    );
    write(
        &repo.path().join("src/namespace.ts"),
        "import * as api from './owner';\nexport const mediated = api.owner();\n",
    );
    write(
        &repo.path().join("src/dynamic.ts"),
        "import { missing } from './absent';\nconst path = './dependency';\nexport const load = () => import(path);\nvoid missing;\n",
    );
    write(
        &repo.path().join("src/catalog.rs"),
        "pub fn alpha() {}\npub fn beta() {}\npub fn gamma() {}\npub fn delta() {}\n",
    );
    write(&repo.path().join("src/empty.ts"), "// deliberately empty\n");
    write(&repo.path().join("README.md"), "# exact file fixture\n");
    write(
        &repo.path().join("tests/owner.test.ts"),
        "import { owner } from '../src/owner';\ntest('owner', () => owner());\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "exact file ls fixture"]);
    repo
}
