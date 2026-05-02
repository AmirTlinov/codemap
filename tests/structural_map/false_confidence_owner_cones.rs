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
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: |\n          pnpm --filter @fixture/api db:generate\n          pnpm --filter @fixture/api test\n",
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
            && manifest_markdown.contains("runs_command -> `prisma generate`")
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
