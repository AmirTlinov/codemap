#[test]
fn owner_surface_cones_expose_manifest_schema_env_and_ci_neighborhoods() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"owner-cone-fixture","private":true,"workspaces":["apps/*"],"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run","db:generate":"prisma generate","db:migrate:deploy":"prisma migrate deploy"},"dependencies":{"@prisma/client":"^5.0.0"}}"#,
    );
    write(
        &repo.path().join("apps/api/prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\ngenerator client { provider = \"prisma-client-js\" }\nmodel User { id String @id }\n",
    );
    write(
        &repo
            .path()
            .join("apps/api/prisma/migrations/20260101000000_init/migration.sql"),
        "create table users (id text primary key);\n",
    );
    write(
        &repo.path().join("apps/api/src/db.ts"),
        "import { PrismaClient } from '@prisma/client';\nexport const prisma = new PrismaClient();\nexport const databaseUrl = process.env.DATABASE_URL;\n",
    );
    write(&repo.path().join(".env.example"), "DATABASE_URL=\nUNUSED_ENV=\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: test -n \"${DATABASE_URL}\" || exit 1\n      - run: |\n          pnpm --filter @fixture/api db:generate\n          pnpm --filter @fixture/api test\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "owner surface cones"]);

    let manifest = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "apps/api/package.json", "--depth", "1"])
        .output()
        .expect("manifest cone should run");
    assert!(
        manifest.status.success(),
        "manifest cone failed: {}",
        String::from_utf8_lossy(&manifest.stderr)
    );
    let manifest_markdown = String::from_utf8(manifest.stdout).expect("markdown utf8");
    assert!(
        manifest_markdown.contains("declares_script -> `script:db:generate`")
            && manifest_markdown.contains("runs_command -> `command:prisma generate`")
            && manifest_markdown.contains("proof_surface -> `cd apps/api && npm test`"),
        "manifest cone should show package-local scripts and proof surfaces: {manifest_markdown}"
    );
    let manifest_roles = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "cone",
            "apps/api/package.json",
            "--depth",
            "1",
            "--section",
            "roles",
        ])
        .output()
        .expect("manifest cone roles should run");
    assert!(
        manifest_roles.status.success(),
        "manifest cone roles failed: {}",
        String::from_utf8_lossy(&manifest_roles.stderr)
    );
    let manifest_roles_markdown = String::from_utf8(manifest_roles.stdout).expect("markdown utf8");
    assert!(
        manifest_roles_markdown.contains("## Roles")
            && manifest_roles_markdown.contains("`manifest`")
            && !manifest_roles_markdown.contains("## Links")
            && !manifest_roles_markdown.contains("## Proof"),
        "cone --section roles should show only the roles layer: {manifest_roles_markdown}"
    );

    let schema = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "apps/api/prisma/schema.prisma", "--depth", "1"])
        .output()
        .expect("schema cone should run");
    assert!(
        schema.status.success(),
        "schema cone failed: {}",
        String::from_utf8_lossy(&schema.stderr)
    );
    let schema_markdown = String::from_utf8(schema.stdout).expect("markdown utf8");
    assert!(
        schema_markdown.contains("reads_env -> `env:DATABASE_URL`")
            && schema_markdown.contains("schema_migration -> `apps/api/prisma/migrations/20260101000000_init/migration.sql`")
            && schema_markdown.contains("schema_client_consumer")
            && schema_markdown.contains("schema_package_script"),
        "schema cone should show env, migrations, Prisma consumers, and schema proof scripts: {schema_markdown}"
    );
    assert!(
        !schema_markdown.contains("pnpm run build")
            && !schema_markdown.contains("pnpm run lint")
            && !schema_markdown.contains("pnpm test"),
        "schema proof should not treat generic package scripts as schema proof: {schema_markdown}"
    );
    let schema_proof_unknown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "proof",
            "apps/api/prisma/schema.prisma",
            "--section",
            "unknown",
        ])
        .output()
        .expect("schema proof unknown should run");
    assert!(
        schema_proof_unknown.status.success(),
        "schema proof unknown failed: {}",
        String::from_utf8_lossy(&schema_proof_unknown.stderr)
    );
    let schema_proof_unknown =
        String::from_utf8(schema_proof_unknown.stdout).expect("markdown utf8");
    assert!(
        schema_proof_unknown.contains("direct_test_import_not_found")
            && !schema_proof_unknown.contains("No Unknown entries were emitted"),
        "proof --section unknown should stay fail-open even when schema scripts exist: {schema_proof_unknown}"
    );

    let env = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", ".env.example", "--depth", "1"])
        .output()
        .expect("env cone should run");
    assert!(
        env.status.success(),
        "env cone failed: {}",
        String::from_utf8_lossy(&env.stderr)
    );
    let env_markdown = String::from_utf8(env.stdout).expect("markdown utf8");
    assert!(
        env_markdown.contains("declares_env -> `env:DATABASE_URL`")
            && env_markdown.contains("env_consumer -> `apps/api/src/db.ts`")
            && env_markdown.contains("env_consumer_not_found")
            && env_markdown.contains("`UNUSED_ENV`"),
        "env cone should show declared keys, static readers, and missing-reader Unknowns: {env_markdown}"
    );
    let env_observed = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "cone",
            ".env.example",
            "--depth",
            "1",
            "--section",
            "observed",
        ])
        .output()
        .expect("env cone observed should run");
    assert!(
        env_observed.status.success(),
        "env cone observed failed: {}",
        String::from_utf8_lossy(&env_observed.stderr)
    );
    let env_observed_markdown = String::from_utf8(env_observed.stdout).expect("markdown utf8");
    assert!(
        env_observed_markdown.contains("declared env keys")
            && env_observed_markdown.contains("`DATABASE_URL`")
            && env_observed_markdown.contains("`.env.example:1`")
            && !env_observed_markdown.contains("## Links"),
        "cone --section observed should keep env declarations in observed facts: {env_observed_markdown}"
    );

    let ci = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", ".github/workflows/ci.yml", "--depth", "1"])
        .output()
        .expect("ci cone should run");
    assert!(
        ci.status.success(),
        "ci cone failed: {}",
        String::from_utf8_lossy(&ci.stderr)
    );
    let ci_markdown = String::from_utf8(ci.stdout).expect("markdown utf8");
    assert!(
        ci_markdown.contains("ci_run_step -> `pnpm --filter @fixture/api db:generate`")
            && ci_markdown.contains("ci_run_step -> `pnpm --filter @fixture/api test`"),
        "CI cone should show workflow run steps as deterministic edges: {ci_markdown}"
    );
    assert!(
        !ci_markdown.contains("ci_run_step -> `|`"),
        "CI cone must not treat YAML block scalar markers as commands: {ci_markdown}"
    );
}

#[test]
fn package_manifest_cone_exposes_package_consumers() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"manifest-consumer-root","private":true,"workspaces":["apps/*","packages/*"]}"#,
    );
    write(
        &repo.path().join("packages/shared/package.json"),
        r#"{"name":"@fixture/shared","version":"1.0.0","exports":{"./index":"./src/index.ts"}}"#,
    );
    write(&repo.path().join("packages/shared/src/index.ts"), "export const shared = 1;\n");
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","dependencies":{"@fixture/shared":"workspace:*"},"scripts":{"test":"vitest run"}}"#,
    );
    write(&repo.path().join("apps/api/src/index.ts"), "import { shared } from '@fixture/shared';\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "package consumer cone"]);

    let links = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "cone",
            "packages/shared/package.json",
            "--depth",
            "1",
            "--section",
            "links",
        ])
        .output()
        .expect("consumer cone should run");
    assert!(
        links.status.success(),
        "consumer cone failed: {}",
        String::from_utf8_lossy(&links.stderr)
    );
    let links = String::from_utf8(links.stdout).expect("markdown utf8");
    assert!(
        links.contains("package_export -> `packages/shared/src/index.ts`")
            && links.contains("incoming:")
            && links.contains("`apps/api/package.json`")
            && links.contains("package_consumer -> `packages/shared/package.json`")
            && links.contains("apps/api/package.json:1"),
        "package manifest cone should expose exports and deterministic package consumers: {links}"
    );
}

#[test]
fn env_cone_links_prioritize_static_consumers_over_long_declaration_lists() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);

    let env_body = (0..41)
        .map(|index| format!("KEY_{index}=\n"))
        .collect::<String>();
    write(&repo.path().join(".env.example"), &env_body);
    write(
        &repo.path().join("src/config.ts"),
        "export const value = process.env.KEY_24;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env cone links"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", ".env.example", "--section", "links"])
        .output()
        .expect("env cone links should run");
    assert!(
        output.status.success(),
        "env cone links failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("env_consumer -> `src/config.ts`")
            && markdown.contains("declares_env -> `env:KEY_0`"),
        "env cone links should keep static consumer evidence visible even when declarations exceed the default limit: {markdown}"
    );

    let observed = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", ".env.example", "--section", "observed"])
        .output()
        .expect("env cone observed should run");
    assert!(
        observed.status.success(),
        "env cone observed failed: {}",
        String::from_utf8_lossy(&observed.stderr)
    );
    let observed = String::from_utf8(observed.stdout).expect("markdown utf8");
    assert!(
        observed.contains("declared env keys: `41`")
            && observed.contains("hidden: 29 env keys")
            && observed.contains("`KEY_0` `.env.example:1`"),
        "env cone observed should count source env declarations, not the truncated link budget: {observed}"
    );
}

#[test]
fn pnpm_workspace_manifest_cone_exposes_members_scripts_and_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"workspace-owner-fixture","private":true,"packageManager":"pnpm@9.15.0","scripts":{"test":"turbo test","lint":"turbo lint","verify:local":"pnpm install --frozen-lockfile && pnpm test"}}"#,
    );
    write(
        &repo.path().join("pnpm-workspace.yaml"),
        "packages:\n  - \"apps/*\"\n  - \"packages/*\"\n  - \"!packages/archive\"\n",
    );
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("packages/ui/package.json"),
        r#"{"name":"@fixture/ui","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("packages/archive/package.json"),
        r#"{"name":"@fixture/archive","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm verify:local\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "pnpm workspace owner"]);

    let links = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "pnpm-workspace.yaml", "--section", "links"])
        .output()
        .expect("workspace cone links should run");
    assert!(
        links.status.success(),
        "workspace cone links failed: {}",
        String::from_utf8_lossy(&links.stderr)
    );
    let links = String::from_utf8(links.stdout).expect("markdown utf8");
    assert!(
        links.contains("declares_workspace_pattern -> `workspace_pattern:apps/*`")
            && links.contains("workspace_member -> `apps/api/package.json`")
            && links.contains("workspace_member -> `packages/ui/package.json`")
            && links.contains("workspace_script -> `script:test`")
            && links.contains("runs_command -> `command:turbo test`"),
        "workspace cone should expose patterns, member manifests, and root scripts: {links}"
    );
    assert!(
        !links.contains("workspace_member -> `packages/archive/package.json`"),
        "workspace cone should respect negated pnpm workspace patterns: {links}"
    );

    let proof = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "pnpm-workspace.yaml", "--section", "proof"])
        .output()
        .expect("workspace cone proof should run");
    assert!(
        proof.status.success(),
        "workspace cone proof failed: {}",
        String::from_utf8_lossy(&proof.stderr)
    );
    let proof = String::from_utf8(proof.stdout).expect("markdown utf8");
    assert!(
        proof.contains("workspace_manifest_script")
            && proof.contains("workspace_manifest_ci_reference")
            && proof.contains("pnpm install --frozen-lockfile"),
        "workspace cone proof should expose root script and CI workspace proof surfaces: {proof}"
    );

    let changed = run_json(
        repo.path(),
        cache.path(),
        &[
            "changed",
            "--files",
            "pnpm-workspace.yaml",
            "--section",
            "proof",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/changed.schema.json", &changed);
    let unknowns = changed["unknowns"].as_array().expect("unknowns");
    for kind in [
        "package_local_script_not_found",
        "ci_reference_not_found",
        "package_consumer_not_found",
        "workspace_members_not_found",
    ] {
        assert!(
            unknowns.iter().all(|unknown| unknown["kind"] != kind),
            "workspace manifest changed proof should not emit stale manifest unknown {kind}: {changed:#}"
        );
    }
}

#[test]
fn prisma_client_consumer_edges_stay_inside_schema_owner_package() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"prisma-owner-fixture","private":true,"workspaces":["apps/*"]}"#,
    );
    for app in ["api", "other"] {
        write(
            &repo.path().join(format!("apps/{app}/package.json")),
            &format!(
                r#"{{"name":"@fixture/{app}","dependencies":{{"@prisma/client":"^5.0.0"}}}}"#
            ),
        );
        write(
            &repo.path().join(format!("apps/{app}/prisma/schema.prisma")),
            "generator client { provider = \"prisma-client-js\" }\nmodel User { id String @id }\n",
        );
    }
    write(
        &repo.path().join("apps/api/src/db.ts"),
        "import { PrismaClient } from '@prisma/client';\nexport const prisma = new PrismaClient();\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "two prisma owners"]);

    let other_cone = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["cone", "apps/other/prisma/schema.prisma", "--section", "links"])
        .output()
        .expect("other schema cone should run");
    assert!(
        other_cone.status.success(),
        "other schema cone failed: {}",
        String::from_utf8_lossy(&other_cone.stderr)
    );
    let markdown = String::from_utf8(other_cone.stdout).expect("markdown utf8");
    assert!(
        !markdown.contains("schema_client_consumer"),
        "Prisma consumer from apps/api must not attach to apps/other schema: {markdown}"
    );

    let unknowns = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "changed",
            "--files",
            "apps/other/prisma/schema.prisma",
            "--section",
            "unknown",
        ])
        .output()
        .expect("changed unknown should run");
    assert!(
        unknowns.status.success(),
        "changed unknown failed: {}",
        String::from_utf8_lossy(&unknowns.stderr)
    );
    let markdown = String::from_utf8(unknowns.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("schema_client_consumer_not_found"),
        "other schema should keep missing client consumer Unknown open: {markdown}"
    );
}
