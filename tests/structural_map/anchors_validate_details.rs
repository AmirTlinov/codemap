#[test]
fn anchors_validate_rejected_config_details_report_problem() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".codemap.yml"),
        r#"version: 2
boundaries:
  forbidden:
    - from: src/**
      to: tests/**
      reason: fixture
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
    assert_eq!(validation["ok"], false);
    assert!(
        validation["problems"]
            .as_array()
            .expect("problems")
            .iter()
            .any(|problem| problem
                .as_str()
                .unwrap_or_default()
                .contains("unsupported .codemap version `2`")),
        "rejected config should stay visible as a top-level problem: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "config"
                && detail["id"] == ".codemap.yml"
                && detail["status"] == "problem"
                && detail["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("unsupported .codemap version `2`")),
        "rejected config should produce a problem detail: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .all(|detail| detail["id"] != "zero-config"),
        "invalid config should not be explained as zero-config: {validation:#}"
    );
    assert!(
        validation["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .all(|warning| !warning
                .as_str()
                .unwrap_or_default()
                .contains("no .codemap.yml found")),
        "invalid config should not emit zero-config warnings: {validation:#}"
    );
}


#[test]
fn anchors_validate_mixed_config_details_scope_status_to_each_config() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".codemap.yml"),
        r#"version: 1
domain:
  id: app
  path: src
"#,
    );
    write(
        &repo.path().join("packages/bad/.codemap.yml"),
        r#"version: 2
domain:
  id: bad
  path: src
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
    assert_eq!(validation["ok"], false);
    let details = validation["details"].as_array().expect("details");
    assert!(
        details.iter().any(|detail| detail["kind"] == "config"
            && detail["id"] == ".codemap.yml"
            && detail["status"] == "ok"
            && detail["next"]
                .as_array()
                .expect("next")
                .iter()
                .all(|command| command == "codemap anchors validate")),
        "valid loaded config should keep ok detail but avoid map commands while validation is not ok: {validation:#}"
    );
    assert!(
        details.iter().any(|detail| detail["kind"] == "config"
            && detail["id"] == "packages/bad/.codemap.yml"
            && detail["status"] == "problem"),
        "rejected nested config should carry the problem detail: {validation:#}"
    );
}


#[test]
fn anchors_validate_problem_details_keep_next_diagnostic_only() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".codemap.yml"),
        r#"version: 1
domain:
  id: app
  path: src
boundaries:
  forbidden:
    - from: src/**
      to: tests/**
verification:
  default:
    - ""
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    write(
        &repo.path().join("tests/app.test.ts"),
        "test('app', () => {});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], false);
    let details = validation["details"].as_array().expect("details");
    for kind in ["domain", "forbidden_boundary", "verification_default"] {
        assert!(
            details.iter().any(|detail| detail["kind"] == kind
                && detail["next"]
                    .as_array()
                    .expect("next")
                    .iter()
                    .all(|command| command == "codemap anchors validate")),
            "when anchor validation is not ok, {kind} detail must not point at fail-closed map commands: {validation:#}"
        );
    }
}


#[test]
fn anchors_validate_explains_resolved_domains_concepts_and_verification() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".codemap.yml"),
        r#"version: 1
domain:
  id: app
  path: src
concepts:
  app.entry:
    role: state_model
    files:
      - src/app.ts
    invariants:
      - deterministic
  app.features:
    role: feature_surface
    files:
      - src/**/*.ts
    invariants:
      - mapped_by_files
boundaries:
  forbidden:
    - from: src/**
      to: tests/**
      reason: app code must not import test code
      recovery:
        - move helper to src/test-support
verification:
  default:
    - pnpm test
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    write(
        &repo.path().join("tests/app.test.ts"),
        "import { app } from '../src/app';\n\ntest('app', () => expect(app).toBe(1));\n",
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
    let details = validation["details"].as_array().expect("details");
    assert!(
        details.iter().any(|detail| detail["kind"] == "domain"
            && detail["id"] == "app"
            && detail["status"] == "ok"
            && detail["message"]
                .as_str()
                .unwrap_or_default()
                .contains("path `src` exists")
            && detail["next"]
                .as_array()
                .expect("next")
                .iter()
                .any(|command| command == "codemap ls src")),
        "domain detail should explain resolved path: {validation:#}"
    );
    assert!(
        details.iter().any(|detail| detail["kind"] == "concept"
            && detail["id"] == "app.entry"
            && detail["status"] == "ok"
            && detail["message"]
                .as_str()
                .unwrap_or_default()
                .contains("exact files resolved: 1")
            && detail["next"]
                .as_array()
                .expect("next")
                .iter()
                .any(|command| command == "codemap cone src/app.ts --depth 1")),
        "concept detail should explain file and invariant resolution: {validation:#}"
    );
    assert!(
        details.iter().any(|detail| detail["kind"] == "concept"
            && detail["id"] == "app.features"
            && detail["status"] == "ok"
            && detail["message"]
                .as_str()
                .unwrap_or_default()
                .contains("glob matches: 1")
            && detail["next"]
                .as_array()
                .expect("next")
                .iter()
                .any(|command| command == "codemap files --path src")),
        "glob concept details should point to bounded files listing, not a non-anchor glob ls: {validation:#}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail["kind"] == "verification_default"
                && detail["status"] == "ok"
                && detail["message"] == "pnpm test"
                && detail["next"]
                    .as_array()
                    .expect("next")
                    .iter()
                    .any(|command| command == "codemap proof changed")),
        "verification defaults should be visible in details: {validation:#}"
    );
}
