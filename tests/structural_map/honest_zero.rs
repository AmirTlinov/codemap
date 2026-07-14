#[test]
fn rust_barrel_reexport_makes_symbol_consumer_count_unknown() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"barrel-honest-zero-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("src/lib.rs"),
        "mod consumer;\nmod inner;\n\npub use inner::*;\n",
    );
    write(
        &repo.path().join("src/inner.rs"),
        "pub struct Thing {\n    pub id: u32,\n}\n",
    );
    write(
        &repo.path().join("src/consumer.rs"),
        "pub fn make() -> crate::Thing {\n    crate::Thing { id: 1 }\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "Thing", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    assert_eq!(json["total_matches"], 1);
    let definition = &json["definitions"][0];
    assert_eq!(definition["anchor"]["path"], "src/inner.rs#Thing");
    assert_eq!(
        definition["consumers_total"]["status"], "unknown",
        "a `pub use inner::*` barrel hides who consumes Thing: {json:#}"
    );
    assert!(
        definition["consumers_total"]["reason"]
            .as_str()
            .expect("unknown reason")
            .contains("re-export flow"),
        "the unknown must name the barrel blind spot: {json:#}"
    );
}

#[test]
fn rust_include_flow_makes_symbol_consumer_count_unknown() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"include-honest-zero-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("src/main.rs"),
        "include!(\"part.rs\");\n\nfn main() {\n    println!(\"{}\", helper());\n}\n",
    );
    write(
        &repo.path().join("src/part.rs"),
        "pub fn helper() -> &'static str {\n    \"ok\"\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "helper", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    assert_eq!(json["total_matches"], 1);
    let definition = &json["definitions"][0];
    assert_eq!(definition["anchor"]["path"], "src/part.rs#helper");
    assert_eq!(
        definition["consumers_total"]["status"], "unknown",
        "include! splices part.rs into main.rs, so helper consumers are not countable: {json:#}"
    );
    assert!(
        definition["consumers_total"]["reason"]
            .as_str()
            .expect("unknown reason")
            .contains("include! flow"),
        "the unknown must name the include! blind spot: {json:#}"
    );
}

#[test]
fn js_dynamic_import_makes_symbol_consumer_count_unknown() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        "{\n  \"name\": \"dynamic-import-honest-zero-fixture\",\n  \"private\": true\n}\n",
    );
    write(
        &repo.path().join("src/a.js"),
        "export async function load() {\n  const mod = await import('./b.js');\n  return mod.greet();\n}\n",
    );
    write(
        &repo.path().join("src/b.js"),
        "export function greet() {\n  return 'hello';\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "greet", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    assert_eq!(json["total_matches"], 1);
    let definition = &json["definitions"][0];
    assert_eq!(definition["anchor"]["path"], "src/b.js#greet");
    for fact in [
        &definition["anchor"]["imported_by"],
        &definition["consumers_total"],
    ] {
        assert_eq!(
            fact["status"], "unknown",
            "which symbols flow through `import('./b.js')` is not countable: {json:#}"
        );
        assert!(
            fact["reason"]
                .as_str()
                .expect("unknown reason")
                .contains("dynamic import"),
            "the unknown must name the dynamic-import blind spot: {json:#}"
        );
    }

    // The literal dynamic specifier is still a resolvable file edge: the
    // file-level cone keeps the counted incoming edge instead of blurring it.
    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/b.js", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "src/a.js" && edge["to"] == "src/b.js"),
        "a literal import('./b.js') stays a countable file edge: {cone:#}"
    );
}

#[test]
fn fully_supported_ts_repo_keeps_proven_zero_and_counted() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        "{\n  \"name\": \"honest-zero-negative-fixture\",\n  \"private\": true\n}\n",
    );
    write(
        &repo.path().join("src/isolated.ts"),
        "export function orphanHelper(): string {\n  return 'unused';\n}\n",
    );
    write(
        &repo.path().join("src/used.ts"),
        "export function sharedHelper(): string {\n  return 'used';\n}\n",
    );
    write(
        &repo.path().join("src/app.ts"),
        "import { sharedHelper } from './used';\n\nexport function run(): string {\n  return sharedHelper();\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    // Zero consumers in a fully supported import flow is a proven fact, not an unknown.
    let isolated = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/isolated.ts", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &isolated);
    assert_eq!(
        isolated["anchor"]["imported_by"]["status"], "proven_zero",
        "no barrel, no include!, no dynamic import: zero must stay proven: {isolated:#}"
    );
    assert_eq!(isolated["anchor"]["imported_by"]["value"], 0);

    let orphan = run_json(
        repo.path(),
        cache.path(),
        &["where", "orphanHelper", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &orphan);
    assert_eq!(
        orphan["definitions"][0]["consumers_total"]["status"],
        "proven_zero"
    );
    assert_eq!(orphan["definitions"][0]["consumers_total"]["value"], 0);

    // Observed consumers stay counted with a real value.
    let used = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/used.ts", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &used);
    assert_eq!(used["anchor"]["imported_by"]["status"], "counted");
    assert!(
        used["anchor"]["imported_by"]["value"]
            .as_u64()
            .expect("counted value")
            >= 1,
        "an observed import edge must keep its count: {used:#}"
    );
}

#[test]
fn self_map_where_cone_report_is_unknown_via_reexport_flow() {
    // Dogfood: this repository re-exports report types through `pub use` barrels
    // in src/model.rs, so symbol consumer counts there must be honest unknowns.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cache = TempDir::new().expect("cache tempdir");
    let json = run_json(root, cache.path(), &["where", "ConeReport", "--format", "json"]);
    assert_schema("schemas/where.schema.json", &json);
    assert!(
        json["total_matches"].as_u64().expect("total") >= 1,
        "ConeReport is defined in this repository: {json:#}"
    );
    let definition = &json["definitions"][0];
    assert_eq!(
        definition["anchor"]["path"],
        "src/model/cone_reports.rs#ConeReport"
    );
    assert_eq!(
        definition["consumers_total"]["status"], "unknown",
        "consumers of ConeReport flow through the model barrel: {json:#}"
    );
    assert!(
        definition["consumers_total"]["reason"]
            .as_str()
            .expect("unknown reason")
            .contains("re-export flow"),
        "the unknown must name the re-export blind spot: {json:#}"
    );
}
