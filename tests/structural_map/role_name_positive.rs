#[test]
fn proof_links_ui_anchor_to_named_unit_and_e2e_surfaces_without_imports() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/frame-title-control.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proof surfaces");
    assert!(
        proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/frame-title-placement.test.ts"
            && surface["evidence"] == "test_surface_tokens"),
        "unit proof should link by structural surface tokens: {proof:#}"
    );
    assert!(
        proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/canvas-blueprint-title-drag.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase"
            && surface["command"]
                .as_str()
                .unwrap_or_default()
                .contains("test:e2e")),
        "e2e proof should link by UI/domain surface tokens and use e2e script: {proof:#}"
    );
    assert!(
        proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/studio-flow.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase"),
        "generic e2e path should still link through shared selector/test-id surface tokens: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/e2e/support/canvas-blueprint.ts"),
        "e2e support files are map surfaces, not runnable proof"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/canvas-text-document.test.ts"),
        "broad token-only unit surfaces must not become proof: {proof:#}"
    );
    assert!(
        proofs.iter().all(|surface| surface["path"]
            != "packages/app/tests/e2e/canvas-blueprint-rail-settings.spec.ts"),
        "e2e proof must require shared exact UI/test surface, not canvas/studio words: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "soft file-level proof commands must not hide broad fallback"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/frame-title-control.tsx",
            "--format",
            "json",
        ],
    );
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof edges")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/tests/e2e/canvas-blueprint-title-drag.spec.ts"
                && edge["evidence"] == "e2e_surface_phrase")
    );
}


#[test]
fn proof_links_dialog_labelledby_accessible_name_to_e2e_role_name() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-accessible-proof-fixture",
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

export function ExportDialog({ open, onClose }) {
  if (!open) return null;
  return (
    <Dialog open={open} onClose={onClose} labelledBy="export-title">
      <h2 id="export-title" style={{ fontSize: 18 }}>
        Export
      </h2>
      <p>
        Export files are generated locally in the browser.
      </p>
    </Dialog>
  );
}
"#,
    );
    write(
        &repo.path().join("src/design/dialog.tsx"),
        r#"export function Dialog({ open, onClose, labelledBy, children }) {
  if (!open) return null;
  return (
    <div role="dialog" aria-modal="true" aria-labelledby={labelledBy}>
      {children}
      <button onClick={onClose}>Close</button>
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
    write(
        &repo.path().join("tests/e2e/import-dialog.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('import dialog is a different accessible surface', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('dialog', { name: 'Import' })).toBeVisible();
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
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|surface| surface["path"]
                == "tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"
                && surface["evidence"] == "e2e_surface_phrase"
                && surface["command"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("test:e2e")),
        "dialog labelledBy accessible name should link to matching e2e role/name proof: {proof:#}"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "tests/e2e/import-dialog.spec.ts"),
        "role name must be exact enough to avoid sibling dialog e2e proof: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "soft e2e role/name proof must not suppress broad fallback: {proof:#}"
    );
}


#[test]
fn proof_links_dialog_labelledby_through_multiline_barrel_exports() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-barrel-accessible-proof-fixture",
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
        r#"import { Dialog } from '../../design';

export function ExportDialog({ open, onClose }) {
  if (!open) return null;
  return (
    <Dialog open={open} onClose={onClose} labelledBy="export-title">
      <h2 id="export-title">Export</h2>
    </Dialog>
  );
}
"#,
    );
    write(
        &repo.path().join("src/design/index.ts"),
        r#"export {
  Dialog,
  type ToastData,
} from './primitives'
"#,
    );
    write(
        &repo.path().join("src/design/primitives.ts"),
        r#"export {
  Dialog,
  type ToastData,
} from './primitives-overlays'
"#,
    );
    write(
        &repo.path().join("src/design/primitives-overlays.tsx"),
        r#"export type ToastData = { id: string };

export function Dialog({ open, onClose, labelledBy, children }) {
  if (!open) return null;
  return (
    <div role="dialog" aria-modal="true" aria-labelledby={labelledBy} onClick={onClose}>
      <div>{children}</div>
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
  const exportDialog = page.getByRole('dialog', { name: 'Export' });
  await expect(exportDialog).toBeVisible();
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
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|surface| surface["path"]
                == "tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"
                && surface["evidence"] == "e2e_surface_phrase"),
        "dialog accessible-name proof should resolve through multiline barrel exports: {proof:#}"
    );
}
