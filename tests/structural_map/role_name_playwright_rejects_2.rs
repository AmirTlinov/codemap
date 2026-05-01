#[test]
fn proof_rejects_skipped_playwright_test_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { test } from '@playwright/test';

test.skip('skipped export dialog assertion', async ({ page }) => {
  await page.goto('/studio');
  await page.getByRole('dialog', { name: 'Export' });
});
"#,
        "test.skip must not become runtime proof because the assertion body does not execute",
    );
}


#[test]
fn proof_rejects_fixme_playwright_test_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { test } from '@playwright/test';

test.fixme('fixme export dialog assertion', async ({ page }) => {
  await page.goto('/studio');
  await page.getByRole('dialog', { name: 'Export' });
});
"#,
        "test.fixme must not become runtime proof because the assertion body does not execute",
    );
}


#[test]
fn proof_rejects_bare_lazy_getbyrole_locator_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { test } from '@playwright/test';

test('only creates a lazy locator', async ({ page }) => {
  await page.goto('/studio');
  page.getByRole('dialog', { name: 'Export' });
});
"#,
        "bare lazy locator creation must not become runtime proof without assertion or action",
    );
}


#[test]
fn proof_rejects_test_inside_skipped_describe_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { test } from '@playwright/test';

test.describe.skip('skipped dialog group', () => {
  test('export dialog assertion never runs', async ({ page }) => {
    await page.goto('/studio');
    await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
  });
});
"#,
        "tests inside test.describe.skip must not become runtime proof",
    );
}


#[test]
fn proof_rejects_test_inside_multiline_skipped_describe_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test.describe.skip(
  'skipped dialog group',
  () => {
    test('export dialog assertion never runs', async ({ page }) => {
      await page.goto('/studio');
      await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
    });
  }
);
"#,
        "tests inside multiline test.describe.skip must not become runtime proof",
    );
}


#[test]
fn proof_rejects_test_inside_multiline_fixme_describe_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test.describe.fixme(
  'fixme dialog group',
  function () {
    test('export dialog assertion never runs', async ({ page }) => {
      await page.goto('/studio');
      await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
    });
  }
);
"#,
        "tests inside multiline test.describe.fixme must not become runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_inside_unexecuted_if_branch_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('conditionally checks export dialog', async ({ page }) => {
  await page.goto('/studio');
  if (false) {
    await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
  }
});
"#,
        "getByRole inside an unparsed if branch must not become unconditional runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_inside_unbraced_if_branch_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('conditionally checks export dialog without braces', async ({ page }) => {
  await page.goto('/studio');
  if (false)
    await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#,
        "getByRole inside an unbraced if branch must not become unconditional runtime proof",
    );
}


#[test]
fn proof_rejects_getbyrole_after_runtime_test_skip_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('skips at runtime before assertion', async ({ page }) => {
  await page.goto('/studio');
  test.skip(true, 'skip on this platform');
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#,
        "getByRole after runtime test.skip must not become executed proof",
    );
}


#[test]
fn proof_rejects_getbyrole_after_return_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('returns before assertion', async ({ page }) => {
  await page.goto('/studio');
  return;
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#,
        "getByRole after return must not become executed proof",
    );
}


#[test]
fn proof_rejects_getbyrole_after_throw_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('throws before assertion', async ({ page }) => {
  await page.goto('/studio');
  throw new Error('stop');
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#,
        "getByRole after throw must not become executed proof",
    );
}


#[test]
fn proof_rejects_getbyrole_after_same_line_return_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('returns before same-line assertion', async ({ page }) => {
  await page.goto('/studio');
  return; await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#,
        "getByRole after a same-line return terminator must not become executed proof",
    );
}


#[test]
fn proof_rejects_unscoped_getbyrole_after_regex_brace_in_finished_test() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('real unrelated test with a regex brace', async ({ page }) => {
  const regex = /{/;
  await page.goto('/real');
});

await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
"#,
        "regex braces inside a finished test must not keep Playwright page scope open for later top-level code",
    );
}


#[test]
fn proof_rejects_unscoped_getbyrole_after_slash_regex_in_one_line_finished_test() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('real unrelated test with a slash regex', async ({ page }) => { const slash = /\//; await page.goto('/real'); });

await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
"#,
        "regex slash literals inside a one-line finished test must not hide the closing Playwright scope",
    );
}


#[test]
fn proof_rejects_negative_visibility_assertion_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('export dialog is absent', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Export' })).not.toBeVisible();
});
"#,
        "negative visibility assertion must not prove the dialog runtime surface",
    );
}


#[test]
fn proof_rejects_negative_visibility_assertion_on_assigned_locator() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('export dialog is absent through assigned locator', async ({ page }) => {
  await page.goto('/studio');
  const exportDialog = page.getByRole('dialog', { name: 'Export' });
  await expect(exportDialog).not.toBeVisible();
});
"#,
        "negative assertion on assigned locator must not prove the dialog runtime surface",
    );
}


#[test]
fn proof_rejects_bare_expect_getbyrole_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('passes locator to expect without matcher', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Export' }));
});
"#,
        "bare expect(getByRole) without positive matcher must not become runtime proof",
    );
}


#[test]
fn proof_rejects_bare_expect_assigned_getbyrole_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('passes assigned locator to expect without matcher', async ({ page }) => {
  await page.goto('/studio');
  const exportDialog = page.getByRole('dialog', { name: 'Export' });
  await expect(exportDialog);
});
"#,
        "bare expect(assigned locator) without positive matcher must not become runtime proof",
    );
}


#[test]
fn proof_rejects_unawaited_expect_getbyrole_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('does not await the locator assertion', async ({ page }) => {
  await page.goto('/studio');
  expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#,
        "unawaited expect(getByRole).matcher must not become runtime proof",
    );
}


#[test]
fn proof_rejects_unawaited_expect_assigned_getbyrole_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('does not await the assigned locator assertion', async ({ page }) => {
  await page.goto('/studio');
  const exportDialog = page.getByRole('dialog', { name: 'Export' });
  expect(exportDialog).toBeVisible();
});
"#,
        "unawaited expect(assigned locator).matcher must not become runtime proof",
    );
}


#[test]
fn proof_rejects_unawaited_getbyrole_action_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('does not await the locator action', async ({ page }) => {
  await page.goto('/studio');
  page.getByRole('dialog', { name: 'Export' }).click();
});
"#,
        "unawaited getByRole action must not become runtime proof",
    );
}


#[test]
fn proof_rejects_unawaited_expect_after_unrelated_same_line_await() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('awaits navigation but not the locator assertion', async ({ page }) => {
  await page.goto('/studio'); expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#,
        "an unrelated earlier await on the same line must not govern a later expect(getByRole)",
    );
}


#[test]
fn proof_rejects_unawaited_action_after_unrelated_same_line_await() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('awaits navigation but not the locator action', async ({ page }) => {
  await page.goto('/studio'); page.getByRole('dialog', { name: 'Export' }).click();
});
"#,
        "an unrelated earlier await on the same line must not govern a later getByRole action",
    );
}


#[test]
fn proof_rejects_unawaited_expect_after_logical_same_statement_await() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('awaits the left side but not the locator assertion', async ({ page }) => {
  await page.goto('/studio') && expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#,
        "await on the left side of a logical expression must not govern a later expect(getByRole)",
    );
}


#[test]
fn proof_rejects_unawaited_action_after_logical_same_statement_await() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('awaits the left side but not the locator action', async ({ page }) => {
  await page.goto('/studio') && page.getByRole('dialog', { name: 'Export' }).click();
});
"#,
        "await on the left side of a logical expression must not govern a later getByRole action",
    );
}


#[test]
fn proof_rejects_unawaited_expect_after_parenthesized_logical_await() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('awaits a parenthesized expression that skips the assertion', async ({ page }) => {
  await (false && expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible());
});
"#,
        "await on a parenthesized logical expression must not prove a branch-gated expect(getByRole)",
    );
}


#[test]
fn proof_rejects_unawaited_action_after_parenthesized_logical_await() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('awaits a parenthesized expression that skips the action', async ({ page }) => {
  await (false && page.getByRole('dialog', { name: 'Export' }).click());
});
"#,
        "await on a parenthesized logical expression must not prove a branch-gated getByRole action",
    );
}


#[test]
fn proof_rejects_assigned_locator_substring_expect_match_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('asserts a different locator with a similar name', async ({ page }) => {
  await page.goto('/studio');
  const exportDialog = page.getByRole('dialog', { name: 'Export' });
  const otherexportDialog = page.locator('.unrelated-widget');
  await expect(otherexportDialog).toBeVisible();
});
"#,
        "assigned locator proof must require the exact expect argument, not substring matching",
    );
}


#[test]
fn proof_rejects_member_assignment_as_pending_local_locator_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('asserts a local locator after assigning a member locator', async ({ page }) => {
  await page.goto('/studio');
  const holder = {};
  holder.exportDialog = page.getByRole('dialog', { name: 'Export' });
  const exportDialog = page.locator('.unrelated-widget');
  await expect(exportDialog).toBeVisible();
});
"#,
        "member assignment must not create pending proof for a same-suffix local identifier",
    );
}


#[test]
fn proof_rejects_reassigned_pending_locator_as_runtime_evidence() {
    assert_no_export_dialog_role_name_proof_with_e2e_source(
        r#"import { expect, test } from '@playwright/test';

test('reassigns the pending locator before asserting it', async ({ page }) => {
  await page.goto('/studio');
  let exportDialog = page.getByRole('dialog', { name: 'Export' });
  exportDialog = page.locator('.unrelated-widget');
  await expect(exportDialog).toBeVisible();
});
"#,
        "pending assigned locator proof must clear when the exact local binding is reassigned",
    );
}

