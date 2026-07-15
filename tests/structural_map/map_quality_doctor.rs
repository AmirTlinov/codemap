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
    assert_eq!(doctor["schema_version"], "6");
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
fn doctor_env_quality_counts_missing_static_readers_without_per_env_cone_scan() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".env.example"),
        "SHARED_KEY=\nROOT_ONLY_KEY=\n",
    );
    write(
        &repo.path().join("apps/api/.env.example"),
        "SHARED_KEY=\nAPI_ONLY_KEY=\n",
    );
    write(
        &repo.path().join("apps/api/src/main.ts"),
        "export const shared = process.env.SHARED_KEY;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env quality"]);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    let warning = doctor["map_quality"]
        .as_array()
        .expect("map_quality")
        .iter()
        .find(|warning| warning["kind"] == "env_config_without_consumers")
        .unwrap_or_else(|| panic!("env map-quality warning should be present: {doctor:#}"));
    let examples = warning["examples"].as_array().expect("examples");
    for expected in [
        ".env.example (1 keys without static readers)",
        "apps/api/.env.example (1 keys without static readers)",
    ] {
        assert!(
            examples.iter().any(|example| example.as_str() == Some(expected)),
            "doctor should count missing env readers per env owner without requiring a full cone proof scan: {doctor:#}"
        );
    }
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
fn doctor_schema_role_warning_does_not_flag_contract_tests() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"contract-test-fixture","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("packages/contracts/tests/http/envelope.contract.test.ts"),
        "import { describe, it } from 'vitest';\ndescribe('contract', () => { it('checks envelope', () => {}); });\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "contract test"]);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &doctor);
    assert!(
        !doctor["map_quality"]
            .as_array()
            .expect("map_quality")
            .iter()
            .any(|warning| warning["kind"] == "schema_role_missing"),
        "contract tests are proof surfaces, not schema owners missing schema roles: {doctor:#}"
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

#[test]
fn doctor_manifest_quality_uses_builtin_cargo_workspace_and_swift_package_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = []\n",
    );
    write(
        &repo.path().join("app/Package.swift"),
        r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "FixtureApp",
    targets: [.target(name: "FixtureApp")]
)
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "manifest proof fixture"]);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    let warnings = doctor["map_quality"].as_array().expect("map_quality");
    assert!(
        warnings
            .iter()
            .all(|warning| warning["kind"] != "manifest_without_deterministic_proof"),
        "Cargo workspace and SwiftPM manifests should have deterministic built-in proof surfaces: {doctor:#}"
    );

    let cargo_proof = run_json(repo.path(), cache.path(), &["proof", "Cargo.toml", "--format", "json"]);
    assert!(
        cargo_proof["proofs"].as_array().expect("proofs").iter().any(
            |proof| proof["command"] == "cargo test"
                && proof["evidence"] == "cargo_manifest_command"
        ),
        "Cargo workspace manifest should expose built-in cargo proof commands: {cargo_proof:#}"
    );
    let swift_proof = run_json(repo.path(), cache.path(), &["proof", "app/Package.swift", "--format", "json"]);
    assert!(
        swift_proof["proofs"].as_array().expect("proofs").iter().any(
            |proof| proof["command"] == "cd app && swift test"
                && proof["evidence"] == "swift_package_command"
        ),
        "Swift package manifest should expose package-local SwiftPM proof commands: {swift_proof:#}"
    );
}

#[test]
fn doctor_manifest_quality_uses_pnpm_workspace_manifest_scripts_and_ci() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"pnpm-workspace-quality","private":true,"packageManager":"pnpm@9.15.0","scripts":{"test":"turbo test","lint":"turbo lint","verify:local":"pnpm install --frozen-lockfile && pnpm test"}}"#,
    );
    write(
        &repo.path().join("pnpm-workspace.yaml"),
        "packages:\n  - \"apps/*\"\n",
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm verify:local\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "pnpm workspace quality"]);

    let doctor = run_json(repo.path(), cache.path(), &["doctor", "--format", "json"]);
    let warnings = doctor["map_quality"].as_array().expect("map_quality");
    let manifest_warning = warnings
        .iter()
        .find(|warning| warning["kind"] == "manifest_without_deterministic_proof");
    assert!(
        manifest_warning.is_none_or(|warning| {
            !warning["examples"]
                .as_array()
                .expect("examples")
                .iter()
                .any(|example| example == "pnpm-workspace.yaml")
        }),
        "pnpm workspace manifest should have deterministic script/CI proof surfaces: {doctor:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "pnpm-workspace.yaml", "--format", "json"],
    );
    assert!(
        proof["proofs"].as_array().expect("proofs").iter().any(
            |surface| surface["evidence"] == "workspace_manifest_script"
                && surface["command"] == "pnpm test"
        ),
        "proof should expose workspace root script surface: {proof:#}"
    );
    assert!(
        proof["proofs"].as_array().expect("proofs").iter().any(
            |surface| surface["evidence"] == "workspace_manifest_ci_reference"
                && surface["command"] == "pnpm install --frozen-lockfile"
        ),
        "proof should expose CI workspace install surface: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .all(|unknown| unknown["kind"] != "nearest_proof_scope"),
        "workspace proof should not fall back to nearest scope when deterministic surfaces exist: {proof:#}"
    );
}
