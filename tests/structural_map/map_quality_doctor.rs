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
