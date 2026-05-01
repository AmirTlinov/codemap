#[test]
fn proof_rejects_getbyrole_name_object_outside_same_call() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-playwright-call-scope-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/export-dialog.tsx"),
        r#"export function ExportDialog() {
  return <div role="dialog" aria-labelledby="export-title"><h2 id="export-title">Export</h2></div>;
}
"#,
    );
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('command palette opens dialog without naming it', async ({ page }) => {
  const metadata = { name: 'Export' }; await expect(page.getByRole('dialog')).toBeVisible();
});
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/features/studio/export-dialog.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "getByRole name must belong to the same call options object: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_getbyrole_inside_string_only_e2e_file() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-playwright-string-only-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/export-dialog.tsx"),
        r#"export function ExportDialog() {
  return <div role="dialog" aria-labelledby="export-title"><h2 id="export-title">Export</h2></div>;
}
"#,
    );
    write(
        &repo.path().join("tests/e2e/string-only.spec.ts"),
        r#"import { test } from '@playwright/test';

test('documents the expected dialog assertion', async () => {
  const docs = "await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible()";
});
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/features/studio/export-dialog.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "getByRole inside a string must not become runnable e2e proof: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_getbyrole_inside_regex_only_e2e_file() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-playwright-regex-only-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/export-dialog.tsx"),
        r#"export function ExportDialog() {
  return <div role="dialog" aria-labelledby="export-title"><h2 id="export-title">Export</h2></div>;
}
"#,
    );
    write(
        &repo.path().join("tests/e2e/docs.spec.ts"),
        r#"import { test } from '@playwright/test';

test('documents the expected dialog assertion pattern', async () => {
  const docs = /page\.getByRole\('dialog', { name: 'Export' }\)/;
});
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/features/studio/export-dialog.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "getByRole inside a regex literal must not become runnable e2e proof: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_getbyrole_name_overridden_by_spread() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('opens import dialog at runtime', async ({ page }) => {
  const metadata = { name: 'Import' };
  await expect(page.getByRole('dialog', { name: 'Export', ...metadata })).toBeVisible();
});
"#,
        "getByRole options with top-level spread must fail closed",
    );
}


#[test]
fn proof_rejects_getbyrole_duplicate_name_override() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('opens import dialog at runtime', async ({ page }) => {
  await expect(page.getByRole('dialog', { name: 'Export', name: 'Import' })).toBeVisible();
});
"#,
        "getByRole duplicate name keys must fail closed",
    );
}


#[test]
fn proof_rejects_bare_getbyrole_helper_as_playwright_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { test } from '@playwright/test';

function getByRole(role, options) {
  return { role, options };
}

test('helper only documents a role lookup shape', async () => {
  getByRole('dialog', { name: 'Export' });
});
"#,
        "bare local getByRole helper must not become Playwright proof",
    );
}


#[test]
fn proof_rejects_member_getbyrole_helper_as_playwright_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { test } from '@playwright/test';

const helper = {
  getByRole(role, options) {
    return { role, options };
  },
};

test('member helper only documents a role lookup shape', async () => {
  helper.getByRole('dialog', { name: 'Export' });
});
"#,
        "local member getByRole helper must not become Playwright proof",
    );
}


#[test]
fn proof_rejects_shadowed_page_getbyrole_as_playwright_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { test } from '@playwright/test';

test('fake page object only documents a role lookup shape', async ({ page }) => {
  {
    const page = {
      getByRole(role, options) {
        return { role, options };
      },
    };
    page.getByRole('dialog', { name: 'Export' });
  }
  await page.goto('/studio');
});
"#,
        "shadowed local page.getByRole must not become Playwright proof",
    );
}

