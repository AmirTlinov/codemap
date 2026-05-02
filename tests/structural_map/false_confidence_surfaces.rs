#[test]
fn first_class_structural_surfaces_do_not_fall_to_unknown_roles() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"false-confidence-fixture","private":true,"workspaces":["apps/*"],"scripts":{"test":"vitest run","lint":"eslint .","build":"tsc -b"}}"#,
    );
    write(
        &repo.path().join("packages/schema/package.json"),
        r#"{"name":"@fixture/schema","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"false-confidence-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo.path().join("apps/api/prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\ngenerator client { provider = \"prisma-client-js\" }\nmodel User { id String @id }\n",
    );
    write(
        &repo.path().join(".env.example"),
        "DATABASE_URL=\nAPI_TOKEN=\n",
    );
    write(&repo.path().join("README.md"), "# False Confidence Fixture\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "first class structural surfaces"]);

    for (path, expected_kind, expected_role) in [
        ("package.json", "manifest", "manifest"),
        ("packages/schema/package.json", "manifest", "manifest"),
        ("Cargo.toml", "manifest", "manifest"),
        (
            "apps/api/prisma/schema.prisma",
            "schema_contract",
            "schema_contract",
        ),
        (".env.example", "env_config", "env_config"),
        ("README.md", "docs", "docs"),
        (".github/workflows/ci.yml", "build_ci", "build_ci"),
    ] {
        let ls = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &ls);
        let anchor = &ls["anchor"];
        assert_eq!(
            anchor["kind"], expected_kind,
            "`{path}` should have a structural kind: {ls:#}"
        );
        assert!(
            anchor["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|role| role == expected_role),
            "`{path}` should carry role `{expected_role}`: {ls:#}"
        );
        if path == "packages/schema/package.json" {
            assert!(
                !anchor["roles"]
                    .as_array()
                    .expect("roles")
                    .iter()
                    .any(|role| role == "schema_contract"),
                "package manifest must not inherit schema_contract from parent directory: {ls:#}"
            );
        }
    }
    let warm_cargo = run_json(repo.path(), cache.path(), &["ls", "Cargo.toml", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &warm_cargo);
    assert_eq!(
        warm_cargo["anchor"]["kind"], "manifest",
        "warm cache must not preserve stale public_boundary-only Cargo.toml facts: {warm_cargo:#}"
    );
    let root_ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &root_ls);
    assert!(
        root_ls["directory"]
            .as_array()
            .expect("directory")
            .iter()
            .any(|surface| surface["kind"] == "manifest"
                && surface["role"] == "manifest"
                && surface["examples"]
                    .as_array()
                    .expect("examples")
                    .iter()
                    .any(|example| example == "Cargo.toml")),
        "root ls should expose direct manifests as manifest surfaces, not only package/public_boundary: {root_ls:#}"
    );

    let cargo_cone = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "Cargo.toml", "--depth", "1"])
        .output()
        .expect("cargo cone should run");
    assert!(
        cargo_cone.status.success(),
        "cargo cone failed: {}",
        String::from_utf8_lossy(&cargo_cone.stderr)
    );
    let markdown = String::from_utf8(cargo_cone.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Roles")
            && markdown.contains("`manifest`")
            && markdown.contains("`public_boundary`")
            && !markdown.contains("`unknown`"),
        "Cargo.toml cone should expose manifest/public boundary roles, not unknown: {markdown}"
    );
}

#[test]
fn source_paths_with_ci_or_build_tokens_do_not_become_ci_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join("src/proof_owner_ci.rs"), "pub fn proof_owner_ci() {}\n");
    write(&repo.path().join("src/ci_pipeline.rs"), "pub fn ci_pipeline() {}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "source files with ci tokens"]);

    for path in ["src/proof_owner_ci.rs", "src/ci_pipeline.rs"] {
        let ls = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &ls);
        assert_eq!(
            ls["anchor"]["kind"], "source",
            "source file `{path}` must not be promoted to a CI/build surface: {ls:#}"
        );
        assert!(
            !ls["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|role| role == "build_ci"),
            "source file `{path}` must not carry build_ci role: {ls:#}"
        );
        assert!(
            !ls["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|role| role == "renderer_ui"),
            "source file `{path}` must not become renderer_ui because `build` contains `ui`: {ls:#}"
        );
    }

    let unknown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "changed",
            "--files",
            "src/proof_owner_ci.rs,src/ci_pipeline.rs",
            "--section",
            "unknown",
        ])
        .output()
        .expect("changed unknown should run");
    assert!(
        unknown.status.success(),
        "changed unknown failed: {}",
        String::from_utf8_lossy(&unknown.stderr)
    );
    let markdown = String::from_utf8(unknown.stdout).expect("markdown utf8");
    assert!(
        !markdown.contains("ci_run_step_not_found"),
        "source files with ci/build path tokens must not emit CI-surface unknowns: {markdown}"
    );
}

#[test]
fn large_lockfiles_are_indexed_as_first_class_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"lockfile-fixture","private":true}"#,
    );
    let mut lock = String::from("lockfileVersion: '9.0'\npackages:\n");
    for index in 0..18_000 {
        lock.push_str(&format!(
            "  /fixture-{index}@1.0.0:\n    resolution: {{integrity: sha512-{index}}}\n"
        ));
    }
    assert!(lock.len() > 900_000, "fixture must cross old too_large cutoff");
    write(&repo.path().join("pnpm-lock.yaml"), &lock);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "large lockfile"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "pnpm-lock.yaml", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["mode"], "file", "large lockfile must be indexed: {ls:#}");
    assert_eq!(ls["anchor"]["kind"], "lockfile", "lockfile kind: {ls:#}");
    assert!(
        ls["anchor"]["roles"]
            .as_array()
            .expect("roles")
            .iter()
            .any(|role| role == "lockfile"),
        "lockfile role: {ls:#}"
    );
}

#[test]
fn changed_roles_classify_manifest_schema_env_docs_and_ci_files() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run","db:generate":"prisma generate"}}"#,
    );
    write(
        &repo.path().join("apps/api/prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\nmodel User { id String @id }\n",
    );
    write(&repo.path().join(".env.example"), "DATABASE_URL=\n");
    write(&repo.path().join("README.md"), "# Fixture\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "baseline"]);

    let files = "apps/api/package.json,apps/api/prisma/schema.prisma,.env.example,README.md,.github/workflows/ci.yml";
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", files, "--section", "roles"])
        .output()
        .expect("changed roles should run");
    assert!(
        output.status.success(),
        "changed roles failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    for role in ["`manifest`", "`schema`", "`env`", "`docs`", "`ci`"] {
        assert!(
            markdown.contains(role),
            "changed roles should contain {role}: {markdown}"
        );
    }
    assert!(
        !markdown.contains("- `unknown`"),
        "known structural surfaces should not be grouped as unknown: {markdown}"
    );
}

#[test]
fn changed_unknown_is_fail_open_for_owner_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("apps/api/prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\nmodel User { id String @id }\n",
    );
    write(&repo.path().join(".env.example"), "DATABASE_URL=\nUNUSED_ENV=\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fail open owner surfaces"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "changed",
            "--files",
            "apps/api/package.json,apps/api/prisma/schema.prisma,.env.example",
            "--section",
            "unknown",
        ])
        .output()
        .expect("changed unknown should run");
    assert!(
        output.status.success(),
        "changed unknown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    for unknown in [
        "direct_test_import_not_found",
        "schema_migration_not_found",
        "schema_client_consumer_not_found",
        "env_consumer_not_found",
        "ci_reference_not_found",
    ] {
        assert!(
            markdown.contains(unknown),
            "changed unknown should fail open with {unknown}: {markdown}"
        );
    }
    assert!(
        !markdown.contains("None found"),
        "changed unknown must not claim None found for owner surfaces: {markdown}"
    );
}

#[test]
fn changed_reports_env_config_schema_manifest_and_ci_structural_events() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join(".env.example"), "OLD_TOKEN=\n");
    write(&repo.path().join("config/app.json"), "{\n  \"oldFlag\": true\n}\n");
    write(
        &repo.path().join("package.json"),
        r#"{"name":"event-fixture","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("prisma/schema.prisma"),
        "model User { id String @id }\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    steps:\n      - run: pnpm test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "structural event baseline"]);

    write(&repo.path().join(".env.example"), "NEW_TOKEN=\n");
    write(&repo.path().join("config/app.json"), "{\n  \"newFlag\": true\n}\n");
    write(
        &repo.path().join("package.json"),
        r#"{"name":"event-fixture","scripts":{"build":"tsc -b"}}"#,
    );
    write(
        &repo.path().join("prisma/schema.prisma"),
        "model Account { id String @id }\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    steps:\n      - run: pnpm build\n",
    );

    let changed = run_json(
        repo.path(),
        cache.path(),
        &[
            "changed",
            "--files",
            ".env.example,config/app.json,package.json,prisma/schema.prisma,.github/workflows/ci.yml",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    let events = changed["structural_events"]
        .as_array()
        .expect("structural events");
    for kind in [
        "added_env_key",
        "removed_env_key",
        "added_config_key",
        "removed_config_key",
        "added_manifest_script",
        "removed_manifest_script",
        "added_schema_surface",
        "removed_schema_surface",
        "added_ci_run_step",
        "removed_ci_run_step",
    ] {
        assert!(
            events.iter().any(|event| event["kind"] == kind
                && event["locations"][0]["line_start"].as_u64().unwrap_or_default() > 0),
            "changed should expose {kind} with line provenance: {changed:#}"
        );
    }
}
