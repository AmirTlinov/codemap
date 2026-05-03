#[test]
fn root_ls_links_expose_manifest_scripts_ci_and_lockfile_without_import_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        r#"[package]
name = "root-links-fixture"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(&repo.path().join("Cargo.lock"), "# lock\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        r#"name: ci
on: [push]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - run: cargo fmt --check
      - run: cargo test
      - run: cargo clippy --all-targets -- -D warnings
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "root links fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--section", "links", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &json);
    let edges = json["edges"].as_array().expect("edges");
    assert!(
        edges.iter().any(|edge| edge["type"] == "declares_script"
            && edge["from"] == "Cargo.toml"
            && edge["to"] == "script:test"
            && edge["evidence"] == "script_manifest"),
        "root links should expose manifest-declared proof script: {json:#}"
    );
    assert!(
        edges.iter().any(|edge| edge["type"] == "uses_lockfile"
            && edge["from"] == "Cargo.toml"
            && edge["to"] == "Cargo.lock"
            && edge["evidence"] == "lockfile"),
        "root links should expose manifest-to-lockfile relation: {json:#}"
    );
    assert!(
        edges.iter().any(|edge| edge["type"] == "ci_calls_script"
            && edge["from"] == ".github/"
            && edge["to"] == "script:test"
            && edge["locations"]
                .as_array()
                .expect("locations")
                .iter()
                .any(|location| location["path"] == ".github/workflows/ci.yml"
                    && location["line_start"] == 8)),
        "root links should expose CI -> script with line provenance: {json:#}"
    );
    assert!(
        edges.iter().any(|edge| edge["type"] == "ci_runs_command"
            && edge["from"] == ".github/"
            && edge["to"] == "command:cargo clippy --all-targets -- -D warnings"),
        "root links should expose validation CI commands as structural facts: {json:#}"
    );
}

#[test]
fn root_ls_links_keep_nested_package_commands_scoped_and_reject_ci_installs() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "root-links-noise-fixture",
  "private": true,
  "workspaces": ["apps/*"]
}
"#,
    );
    write(
        &repo.path().join("apps/web/package.json"),
        r#"{
  "name": "@fixture/web",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        r#"name: ci
on: [push]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - run: pnpm exec playwright install --with-deps chromium
      - run: pnpm --filter @fixture/web test
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "nested links fixture"]);

    let root = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--section", "links", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &root);
    let root_edges = root["edges"].as_array().expect("root edges");
    assert!(
        root_edges.iter().any(|edge| edge["type"] == "declares_script"
            && edge["from"] == "apps/web/"
            && edge["to"] == "script:apps/web:test"),
        "root links should expose package-local proof script existence: {root:#}"
    );
    assert!(
        root_edges.iter().all(|edge| {
            !(edge["type"] == "runs_command"
                && edge["from"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("script:apps/web:"))
        }),
        "root links should not dump nested package script commands by default: {root:#}"
    );
    assert!(
        root_edges.iter().all(|edge| {
            !(edge["type"] == "ci_runs_command"
                && edge["to"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("playwright install"))
        }),
        "CI setup/install steps must not become validation proof links: {root:#}"
    );

    let scoped = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "apps/web",
            "--section",
            "links",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &scoped);
    let scoped_edges = scoped["edges"].as_array().expect("scoped edges");
    assert!(
        scoped_edges.iter().any(|edge| edge["type"] == "runs_command"
            && edge["from"] == "script:apps/web:test"
            && edge["to"] == "command:vitest run"),
        "scoped package map should expose the exact package script command: {scoped:#}"
    );
}

#[test]
fn root_ls_owner_env_edges_use_report_hidden_accounting() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    let env_body = (0..40)
        .map(|index| format!("KEY_{index:02}=value\n"))
        .collect::<String>();
    write(&repo.path().join(".env.example"), &env_body);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env edge hidden fixture"]);

    let bounded = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--section", "links", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &bounded);
    let expanded = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--section", "links", "--all", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &expanded);
    let bounded_env_edges = bounded["edges"]
        .as_array()
        .expect("bounded edges")
        .iter()
        .filter(|edge| edge["type"] == "declares_env")
        .count();
    let expanded_env_edges = expanded["edges"]
        .as_array()
        .expect("expanded edges")
        .iter()
        .filter(|edge| edge["type"] == "declares_env")
        .count();
    assert!(
        expanded_env_edges > bounded_env_edges,
        "fixture should have env edges hidden by normal report limit, not by owner-edge pre-caps: bounded={bounded_env_edges}, expanded={expanded_env_edges}, bounded={bounded:#}, expanded={expanded:#}"
    );
    assert!(
        bounded["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"] == "directory edges hidden by limit"
                && hidden["expand"] == "codemap ls . --all"),
        "bounded report must expose hidden owner edges with exact expand: {bounded:#}"
    );
}
