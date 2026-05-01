#[test]
fn proof_rejects_local_fake_test_page_fixture_as_playwright_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"const test = (name, callback) => callback({ page: {
  getByRole(role, options) {
    return { role, options };
  },
} });

test('fake local test callback only documents a role lookup shape', async ({ page }) => {
  page.getByRole('dialog', { name: 'Export' });
});
"#,
        "local fake test callback must not establish Playwright page fixture provenance",
    );
}


#[test]
fn proof_rejects_nested_page_param_shadow_as_playwright_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { test } from '@playwright/test';

test('nested callback page object only documents a role lookup shape', async ({ page }) => {
  const items = [{
    getByRole(role, options) {
      return { role, options };
    },
  }];
  items.map((page) => page.getByRole('dialog', { name: 'Export' }));
});
"#,
        "nested page callback parameter must shadow the Playwright fixture for proof extraction",
    );
}


#[test]
fn proof_rejects_getbyrole_inside_uninvoked_nested_helper_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('helper body is never invoked', async ({ page }) => {
  function helper() {
    page.getByRole('dialog', { name: 'Export' });
  }
  await page.goto('/studio');
});
"#,
        "uninvoked nested function body must not become runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_inside_uninvoked_multiline_function_helper_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('helper body is never invoked', async ({ page }) => {
  function helper()
  {
    page.getByRole('dialog', { name: 'Export' }).click();
  }
  await page.goto('/studio');
});
"#,
        "uninvoked multiline function helper body must not become runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_inside_uninvoked_arrow_helper_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('arrow helper body is never invoked', async ({ page }) => {
  const helper = () => {
    page.getByRole('dialog', { name: 'Export' });
  };
  await page.goto('/studio');
});
"#,
        "uninvoked nested arrow body must not become runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_inside_uninvoked_expression_arrow_helper_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('expression arrow helper is never invoked', async ({ page }) => {
  const openDialog = () => page.getByRole('dialog', { name: 'Export' }).click();
  await page.goto('/studio');
});
"#,
        "uninvoked expression-bodied arrow helper must not become runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_inside_uninvoked_multiline_expression_arrow_helper_as_runtime_evidence()
{
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('multiline expression arrow helper is never invoked', async ({ page }) => {
  const openDialog = () =>
    page.getByRole('dialog', { name: 'Export' }).click();
  await page.goto('/studio');
});
"#,
        "uninvoked multiline expression-bodied arrow helper must not become runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_inside_uninvoked_method_helper_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('method helper body is never invoked', async ({ page }) => {
  const helpers = {
    openDialog() {
      page.getByRole('dialog', { name: 'Export' });
    },
  };
  await page.goto('/studio');
});
"#,
        "uninvoked nested method body must not become runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_nested_metadata_name_as_role_name() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-playwright-nested-options-fixture",
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

test('command palette opens unnamed dialog with metadata', async ({ page }) => {
  await expect(page.getByRole('dialog', { metadata: { name: 'Export' } })).toBeVisible();
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
        "getByRole name must be a top-level options key, not nested metadata: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}

