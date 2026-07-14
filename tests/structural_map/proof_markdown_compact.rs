#[test]
fn proof_changed_markdown_summarizes_sensors_by_command() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"proof-compact-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/session.ts"),
        "export function sessionValue() {\n  return 1;\n}\n",
    );
    for index in 1..=8 {
        write(
            &repo.path().join(format!("tests/session-{index}.test.ts")),
            "import { sessionValue } from '../src/session';\n\ntest('session value', () => {\n  expect(sessionValue()).toBe(1);\n});\n",
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof compact fixture"]);
    write(
        &repo.path().join("src/session.ts"),
        "export function sessionValue() {\n  return 2;\n}\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed", "--limit", "20"])
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8_lossy(&output.stdout);
    assert!(
        markdown.contains("- sensors: `8`"),
        "proof markdown should summarize command sensor count: {markdown}"
    );
    assert!(
        markdown.contains("- evidence: `test_import: 8`"),
        "proof markdown should show evidence distribution: {markdown}"
    );
    assert!(
        markdown.contains("- hidden details: `3` sensors"),
        "proof markdown should hide excess per-command detail: {markdown}"
    );
    assert!(
        markdown.contains("codemap proof-map --changed --raw-sensors --limit 8"),
        "proof markdown should expose raw-sensor expansion: {markdown}"
    );
    assert_eq!(
        markdown.matches("[test_import; high]").count(),
        5,
        "proof markdown should sample, not dump every direct sensor: {markdown}"
    );

    let changed_output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--limit", "20"])
        .output()
        .expect("codemap should run");
    assert!(
        changed_output.status.success(),
        "codemap changed failed: {}",
        String::from_utf8_lossy(&changed_output.stderr)
    );
    let changed_markdown = String::from_utf8_lossy(&changed_output.stdout);
    assert!(
        changed_markdown.contains("- sensors: `8`"),
        "changed markdown should summarize proof sensor count: {changed_markdown}"
    );
    assert!(
        changed_markdown.contains("- hidden details: `3` sensors"),
        "changed markdown should hide excess proof detail: {changed_markdown}"
    );
    assert_eq!(
        changed_markdown.matches("[test_import; high]").count(),
        5,
        "changed proof section should sample, not dump every direct sensor: {changed_markdown}"
    );
}

#[test]
fn proof_changed_section_filter_works_on_clean_fast_path() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed", "--section", "unknown"])
        .output()
        .expect("proof changed unknown section should run");
    assert!(
        output.status.success(),
        "proof changed --section unknown should run through the clean fast path: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("# Verification Surface Plan") && markdown.contains("## Unknown"),
        "proof changed --section unknown should render a stable Unknown layer: {markdown}"
    );
    assert!(
        !markdown.contains("unexpected argument") && !markdown.contains("## Verification Surfaces"),
        "proof changed --section unknown should not fall through to CLI errors or verification sections: {markdown}"
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed"])
        .output()
        .expect("clean proof changed should run");
    assert!(
        output.status.success(),
        "clean proof changed failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("No changed anchors selected.")
            && !markdown.contains("No verification surface found."),
        "clean proof changed should explain the empty selector, not imply a missing verification surface: {markdown}"
    );
}

#[test]
fn proof_markdown_separates_evidence_only_surfaces_from_commands() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join(".env.example"), "DATABASE_URL=\n");
    write(
        &repo.path().join("src/config.ts"),
        "export const databaseUrl = process.env.DATABASE_URL;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env proof fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", ".env.example"])
        .output()
        .expect("env proof should run");
    assert!(
        output.status.success(),
        "env proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Linked Surfaces")
            && markdown.contains("env_consumer_reference")
            && !markdown.contains("### `no command`"),
        "env consumer references should render as linked surfaces, not no-command runnable surfaces: {markdown}"
    );
    assert!(
        !markdown.contains("only soft surface matches"),
        "linked env surfaces must not be described as soft-match only: {markdown}"
    );
    assert!(
        !markdown.contains("## Runnable Command Surfaces\n\n### `env_consumer_reference`"),
        "linked env surfaces must not be grouped as runnable command surfaces: {markdown}"
    );
}

#[test]
fn changed_proof_section_shows_evidence_only_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(&repo.path().join(".env.example"), "DATABASE_URL=\n");
    write(
        &repo.path().join("src/config.ts"),
        "export const databaseUrl = process.env.DATABASE_URL;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "env changed proof fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["changed", "--files", ".env.example", "--section", "proof"])
        .output()
        .expect("changed env proof should run");
    assert!(
        output.status.success(),
        "changed env proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Linked Surfaces")
            && markdown.contains("env_consumer_reference")
            && markdown.contains("src/config.ts"),
        "changed proof should render command-less env consumer links, not only sensor counts: {markdown}"
    );
    assert!(
        !markdown.contains("### `no command`")
            && !markdown.contains("## Soft Surface Matches\n\n### `env_consumer_reference`"),
        "changed proof must not misclassify linked env references as no-command runnable or soft matches: {markdown}"
    );
}

#[test]
fn proof_markdown_separates_setup_surfaces_from_runnable_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"workspace-proof-fixture","private":true,"packageManager":"pnpm@9.15.0","scripts":{"test":"turbo test","verify:local":"pnpm test"}}"#,
    );
    write(&repo.path().join("pnpm-workspace.yaml"), "packages:\n  - \"apps/*\"\n");
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm verify:local\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "workspace proof fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "pnpm-workspace.yaml"])
        .output()
        .expect("workspace proof should run");
    assert!(
        output.status.success(),
        "workspace proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("## Runnable Command Surfaces") && markdown.contains("pnpm test"),
        "workspace proof should still show runnable command surfaces: {markdown}"
    );
    assert!(
        markdown.contains("## Setup / Support Surfaces")
            && markdown.contains("pnpm install --frozen-lockfile"),
        "CI install steps should stay visible as setup/support, not disappear: {markdown}"
    );
    let before_support = markdown
        .split("## Setup / Support Surfaces")
        .next()
        .unwrap_or(&markdown);
    assert!(
        !before_support.contains("pnpm install --frozen-lockfile"),
        "install steps must not be rendered under runnable command surfaces: {markdown}"
    );
}

#[test]
fn schema_db_mutation_scripts_are_setup_support_not_runnable_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("apps/api/package.json"),
        r#"{"name":"@fixture/api","scripts":{"db:migrate:status":"prisma migrate status --schema prisma/schema.prisma","db:push":"prisma db push","db:normalize-rarity":"node prisma/normalize-achievement-rarity.mjs"}}"#,
    );
    write(
        &repo.path().join("apps/api/prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\ngenerator client { provider = \"prisma-client-js\" }\nmodel User { id String @id }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "schema proof fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "apps/api/prisma/schema.prisma"])
        .output()
        .expect("schema proof should run");
    assert!(
        output.status.success(),
        "schema proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    let before_support = markdown
        .split("## Setup / Support Surfaces")
        .next()
        .unwrap_or(&markdown);
    assert!(
        before_support.contains("db:migrate:status"),
        "schema status checks should remain runnable command surfaces: {markdown}"
    );
    assert!(
        !before_support.contains("db:push") && !before_support.contains("db:normalize-rarity"),
        "schema mutation scripts must not render as runnable command surfaces: {markdown}"
    );
    let support = markdown
        .split("## Setup / Support Surfaces")
        .nth(1)
        .unwrap_or("");
    assert!(
        support.contains("db:push") && support.contains("db:normalize-rarity"),
        "schema mutation scripts should remain visible as setup/support surfaces: {markdown}"
    );
}

#[test]
fn package_watch_scripts_are_setup_support_without_hiding_verify_dev_scripts() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"watch-proof-fixture","private":true,"scripts":{"test":"vitest run","test:watch":"vitest","verify:dev-fixtures":"node scripts/verify-dev-fixtures.mjs"}}"#,
    );
    write(
        &repo.path().join("scripts/verify-dev-fixtures.mjs"),
        "console.log('verify dev fixtures');\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "watch proof fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "package.json"])
        .output()
        .expect("package proof should run");
    assert!(
        output.status.success(),
        "package proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    let before_support = markdown
        .split("## Setup / Support Surfaces")
        .next()
        .unwrap_or(&markdown);
    assert!(
        before_support.contains("npm test")
            && before_support.contains("verify:dev-fixtures")
            && !before_support.contains("test:watch"),
        "watch mode must not be runnable, while verify:dev-fixtures remains a verification command surface: {markdown}"
    );
    let support = markdown
        .split("## Setup / Support Surfaces")
        .nth(1)
        .unwrap_or("");
    assert!(
        support.contains("test:watch"),
        "watch mode should stay visible as setup/support: {markdown}"
    );
}

#[test]
fn validation_scripts_with_setup_in_name_remain_runnable_proof() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"setup-name-proof-fixture","private":true,"packageManager":"pnpm@9.15.0","scripts":{"smoke:e2e:setup-templates":"node node_modules/@playwright/test/cli.js test tests/e2e/setup-templates.ensure.spec.ts","e2e:install":"node node_modules/@playwright/test/cli.js install chromium"}}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "setup name proof fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "package.json"])
        .output()
        .expect("package proof should run");
    assert!(
        output.status.success(),
        "package proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    let before_support = markdown
        .split("## Setup / Support Surfaces")
        .next()
        .unwrap_or(&markdown);
    assert!(
        before_support.contains("smoke:e2e:setup-templates"),
        "a Playwright test script should stay a runnable command surface even when its name contains setup: {markdown}"
    );
    let support = markdown
        .split("## Setup / Support Surfaces")
        .nth(1)
        .unwrap_or("");
    assert!(
        support.contains("e2e:install") && !support.contains("smoke:e2e:setup-templates"),
        "install stays support while setup-named test stays runnable: {markdown}"
    );
}

#[test]
fn proof_changed_unknown_stays_fail_open_for_changed_source_without_direct_test() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"proof-unknown-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/runtime.ts"),
        "export function runtimeValue() {\n  return 1;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof unknown fixture"]);
    write(
        &repo.path().join("src/runtime.ts"),
        "export function runtimeValue() {\n  return 2;\n}\n",
    );

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed", "--section", "unknown"])
        .output()
        .expect("proof changed unknown should run");
    assert!(
        output.status.success(),
        "proof changed unknown failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("direct_test_import_not_found")
            && markdown.contains("src/runtime.ts")
            && !markdown.contains("No Unknown entries were emitted"),
        "proof changed must not hide a missing direct link just because a script surface exists: {markdown}"
    );
}

#[test]
fn proof_section_filter_is_display_only_and_refuses_run() {
    let (repo, cache) = fixture();

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", "changed", "--section", "unknown", "--run"])
        .output()
        .expect("proof section run guard should execute");
    assert!(
        !output.status.success(),
        "proof --run must not execute with a display-only section filter"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(
        stderr.contains("--section is display-only"),
        "guard should explain the conflict without running commands: {stderr}"
    );
}
