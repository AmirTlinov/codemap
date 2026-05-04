#[test]
fn changed_schema_contract_json_keys_are_schema_fields_not_config_keys() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"schema-field-fixture","private":true}"#,
    );
    write(
        &repo.path().join("schemas/widget.schema.json"),
        r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "name": { "type": "string" }
  }
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "schema baseline"]);
    write(
        &repo.path().join("schemas/widget.schema.json"),
        r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "xray": { "type": "object" }
  },
  "required": ["name"]
}
"#,
    );

    let changed = run_json(
        repo.path(),
        cache.path(),
        &[
            "changed",
            "--files",
            "schemas/widget.schema.json",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    let events = changed["structural_events"]
        .as_array()
        .expect("structural events");
    assert!(
        events.iter().any(|event| event["kind"] == "added_schema_field"
            && event["evidence"] == "git_diff_schema_field"),
        "schema contract JSON key additions should be schema-field deltas: {changed:#}"
    );
    assert!(
        events
            .iter()
            .all(|event| event["kind"] != "added_config_key"),
        "schema contract JSON must not look like runtime/config-key mutation: {changed:#}"
    );
}
