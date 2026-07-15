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
        "use crate::inner::Thing;\n\npub fn make() -> Thing {\n    Thing { id: 1 }\n}\n",
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
    assert_open_count(definition, 0, "reexport_flow", &json);
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
    assert_open_count(definition, 0, "rust_include_flow", &json);
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
    let legacy_anchor_count = &definition["anchor"]["imported_by"];
    assert_eq!(legacy_anchor_count["status"], "unknown");
    assert!(
        legacy_anchor_count["reason"]
            .as_str()
            .expect("unknown reason")
            .contains("dynamic import"),
        "the legacy file count must retain its dynamic-import boundary: {json:#}"
    );
    assert_open_count(definition, 0, "dynamic_import_flow", &json);

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
    let orphan_count = &orphan["definitions"][0]["consumers_total"];
    assert_eq!(orphan_count["closure"], "closed");
    assert_eq!(orphan_count["observed"], 0);
    assert_count_certificate_resolves(&orphan["definitions"][0], &orphan);

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
    assert_open_count(definition, 0, "reexport_flow", &json);
}

fn assert_open_count(definition: &Value, observed: u64, reason: &str, report: &Value) {
    let count = &definition["consumers_total"];
    assert_eq!(count["observed"], observed, "lower bound drifted: {report:#}");
    assert_eq!(count["closure"], "open", "count must stay open: {report:#}");
    assert!(
        count["reasons"]
            .as_array()
            .expect("typed reasons")
            .iter()
            .any(|value| value == reason),
        "count must name {reason}: {report:#}"
    );
    assert_count_certificate_resolves(definition, report);
}

fn assert_count_certificate_resolves(definition: &Value, report: &Value) {
    let id = definition["consumers_total"]["certificate_id"]
        .as_str()
        .expect("certificate id");
    let certificate = &definition["observations"]["certificates"][id];
    assert_eq!(certificate["id"], id, "dangling certificate id: {report:#}");
    assert_eq!(
        certificate["closure"], definition["consumers_total"]["closure"],
        "count and certificate closure must agree: {report:#}"
    );
}
