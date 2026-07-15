#[test]
fn anchors_validate_reports_summary_and_actionable_warnings() {
    let (repo, cache) = fixture();
    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["kind"], "anchor_validation");
    assert_eq!(validation["schema_version"], "6");
    assert_eq!(validation["ok"], true);
    assert_eq!(validation["summary"]["forbidden_boundaries"], 1);
    assert_eq!(validation["summary"]["concepts"], 0);
    assert!(
        validation["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning
                .as_str()
                .unwrap_or_default()
                .contains("no recovery steps")),
        "boundary warnings should explain why violations would be less actionable: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "forbidden_boundary"
                && detail["id"] == "#1"
                && detail["status"] == "warning"
                && detail["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("`from` matches")),
        "anchor validation should explain how boundary patterns resolved: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "forbidden_boundary"
                && detail["next"]
                    .as_array()
                    .expect("next")
                    .iter()
                    .any(|command| command == "codemap boundaries")),
        "boundary details should point to the structural boundary map command: {validation:#}"
    );
}


#[test]
fn anchors_validate_warning_details_do_not_contradict_ok_report() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".codemap.yml"),
        r#"version: 1
concepts:
  generated.assets:
    role: generated_boundary
    files:
      - src/generated/**/*.ts
boundaries:
  forbidden:
    - from: src/generated/**
      to: tests/missing/**
      reason: generated code must stay isolated
      recovery:
        - update generator
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], true);
    assert!(
        validation["problems"]
            .as_array()
            .expect("problems")
            .is_empty(),
        "fixture should only produce warnings: {validation:#}"
    );
    assert!(
        !validation["warnings"]
            .as_array()
            .expect("warnings")
            .is_empty(),
        "zero-match globs should stay visible as warnings: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .all(|detail| detail["status"] != "problem"),
        "details must not report problems when top-level problems are empty: {validation:#}"
    );
}


#[test]
fn anchors_validate_exact_boundary_paths_count_unique_targets() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".codemap.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: src/app.ts
      to: tests/app.test.ts
      reason: app code must not import test code
      recovery:
        - move shared helper to src/test-support
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    write(
        &repo.path().join("tests/app.test.ts"),
        "import { app } from '../src/app';\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], true);
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "forbidden_boundary"
                && detail["status"] == "ok"
                && detail["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("`from` matches 1; `to` matches 1;")),
        "exact boundary paths should count unique resolved targets, not mechanisms: {validation:#}"
    );
}


#[test]
fn anchors_validate_glob_boundary_paths_count_unique_targets() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"anchor-count-fixture","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".codemap.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: "*.json"
      to: src/app.ts
      reason: manifests must not drive app code directly
      recovery:
        - read manifest through config adapter
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], true);
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "forbidden_boundary"
                && detail["status"] == "ok"
                && detail["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("`from` matches 1; `to` matches 1;")),
        "glob boundary paths should count unique targets, not file/manifest mechanisms: {validation:#}"
    );
}
