#[test]
fn doctor_reports_map_quality_warnings_for_incomplete_owner_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"map-quality-fixture","private":true}"#,
    );
    write(
        &repo.path().join(".env.example"),
        "DATABASE_URL=\nUNUSED_RUNTIME_KEY=\n",
    );
    write(
        &repo.path().join("prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\nmodel User { id String @id }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "incomplete owner surfaces"]);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert_eq!(doctor["schema_version"], "5");
    let warnings = doctor["map_quality"].as_array().expect("map_quality");
    for kind in [
        "manifest_without_deterministic_proof",
        "schema_without_deterministic_proof",
        "env_config_without_consumers",
    ] {
        assert!(
            warnings.iter().any(|warning| warning["kind"] == kind),
            "doctor should report map-quality warning `{kind}` without claiming a fix: {doctor:#}"
        );
    }

    let markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["doctor"])
        .output()
        .expect("doctor markdown should run");
    assert!(
        markdown.status.success(),
        "doctor markdown failed: {}",
        String::from_utf8_lossy(&markdown.stderr)
    );
    let markdown = String::from_utf8(markdown.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Map Quality Warnings")
            && markdown.contains("`manifest_without_deterministic_proof`")
            && markdown.contains("codemap proof package.json"),
        "doctor markdown should show compact map-quality diagnostics with expand commands: {markdown}"
    );
}

#[test]
fn doctor_schema_proof_warning_skips_schema_contracts_without_owner_detector() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"schema-noise-fixture","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("schemas/report.schema.json"),
        r#"{"type":"object","properties":{"ok":{"type":"boolean"}}}"#,
    );
    write(
        &repo.path().join("packages/shared/src/types/api.ts"),
        "export interface ApiDto { id: string }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "schema contract noise fixture"]);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    let warnings = doctor["map_quality"].as_array().expect("map_quality");
    assert!(
        warnings
            .iter()
            .all(|warning| warning["kind"] != "schema_without_deterministic_proof"),
        "doctor should not ask DB-schema proof sensors from JSON schemas or TS type contracts: {doctor:#}"
    );
}

#[test]
fn doctor_schema_proof_warning_keeps_root_migrations_visible() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"migration-quality-fixture","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("migrations/001_init.ts"),
        "export const up = 'create table users(id text primary key)';\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "root migration fixture"]);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    let warnings = doctor["map_quality"].as_array().expect("map_quality");
    let schema_warning = warnings
        .iter()
        .find(|warning| warning["kind"] == "schema_without_deterministic_proof")
        .unwrap_or_else(|| {
            panic!("root migration owner should keep schema proof warning visible: {doctor:#}")
        });
    assert!(
        schema_warning["examples"]
            .as_array()
            .expect("examples")
            .iter()
            .any(|example| example == "migrations/001_init.ts"),
        "schema proof warning should point at root migration owner: {doctor:#}"
    );
}
