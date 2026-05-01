#[test]
fn proof_rejects_dialog_labelledby_when_component_does_not_forward_accessible_name() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-accessible-negative-fixture",
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
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog({ open }) {
  if (!open) return null;
  return (
    <Dialog open={open} labelledBy="export-title">
      <h2 id="export-title">Export</h2>
    </Dialog>
  );
}
"#,
    );
    write(
        &repo.path().join("src/design/dialog.tsx"),
        r#"export function Dialog({ open, children }) {
  if (!open) return null;
  return <section>{children}</section>;
}
"#,
    );
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('command palette opens export dialog', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
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
        "component name alone must not create a dialog role/name proof: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_dialog_accessible_name_from_text_outside_labelled_element() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-accessible-overcapture-fixture",
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
  return (
    <div role="dialog" aria-labelledby="export-title">
      <h2 id="export-title" />
      <p>Export files are generated locally</p>
    </div>
  );
}
"#,
    );
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('command palette opens export dialog', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Export files are generated locally' })).toBeVisible();
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
        "dialog accessible-name proof must not capture text outside the labelled element: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_native_dialog_labelledby_alias_without_real_aria_labelledby() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-native-wrong-attr-fixture",
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
  return (
    <div role="dialog" labelledBy="export-title">
      <h2 id="export-title">Export</h2>
    </div>
  );
}
"#,
    );
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('command palette opens export dialog', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
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
        "native/explicit role JSX must require real aria-labelledby, not labelledBy alias: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_accessible_role_name_collision_with_generic_surface_phrase() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-accessible-collision-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/export-chip.tsx"),
        r#"export function ExportChip() {
  return <div className="dialog-export">Export</div>;
}
"#,
    );
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('command palette opens export dialog', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
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
            "src/features/studio/export-chip.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "generic class/text phrase must not collide with role/name proof surface: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_dialog_component_contract_when_role_and_label_are_on_different_elements() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-accessible-split-contract-fixture",
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
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return (
    <Dialog labelledBy="export-title">
      <h2 id="export-title">Export</h2>
    </Dialog>
  );
}
"#,
    );
    write(
        &repo.path().join("src/design/dialog.tsx"),
        r#"export function Dialog({ labelledBy, children }) {
  return (
    <div role="dialog">
      <section aria-labelledby={labelledBy}>{children}</section>
    </div>
  );
}
"#,
    );
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('command palette opens export dialog', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
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
        "component contract must require role and aria-labelledby on the same opening element: {proof:#}"
    );
}


#[test]
fn proof_rejects_custom_dialog_aria_labelledby_prop_without_proven_prop_mapping() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-accessible-wrong-prop-fixture",
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
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return (
    <Dialog aria-labelledby="export-title">
      <h2 id="export-title">Export</h2>
    </Dialog>
  );
}
"#,
    );
    write(
        &repo.path().join("src/design/dialog.tsx"),
        r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
    );
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('command palette opens export dialog', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
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
        "custom component aria-labelledby prop must not be trusted without a proven prop mapping: {proof:#}"
    );
}

