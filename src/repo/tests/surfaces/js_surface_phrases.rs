// Responsibility: repo-tests-js-surface-phrases
use crate::repo::{CacheWriteMode, RootSelection, extract_surfaces, load_project_with_cache};

#[test]
fn javascript_surface_tokens_capture_ui_selectors_without_plain_text_noise() {
    let text = r#"export function Button() {
  return <button data-testid="submit-order-button" aria-label="Submit order">Submit order</button>;
}

test("flow", async ({ page }) => {
  await test.step("submit-order-button string in prose is not evidence", async () => {});
  await page.goto("/orders/new");
  await expect(page.locator(".submit-order-button")).toBeVisible();
});
"#;

    let surfaces = extract_surfaces(text, "tsx");
    let tokens = &surfaces.tokens;

    assert!(tokens.contains("submit"));
    assert!(tokens.contains("order"));
    assert!(tokens.contains("orders"));
    assert!(surfaces.phrases.contains("submit-order-button"));
    assert!(surfaces.phrases.contains("orders-new"));
    assert!(!tokens.contains("button"));
    assert!(!tokens.contains("flow"));
    assert!(!tokens.contains("prose"));
}

#[test]
fn javascript_surface_phrases_skip_import_paths_and_broad_mode_literals() {
    let text = r#"import { useFrameTitleDrag } from './use-frame-title-drag';

export function Title() {
  return <div className="blueprint-frame-node__title-input nodrag" data-mode="frame-title" />;
}
"#;

    let surfaces = extract_surfaces(text, "tsx");

    assert!(
        surfaces
            .phrases
            .contains("blueprint-frame-node-title-input")
    );
    assert!(!surfaces.phrases.contains("use-frame-title-drag"));
    assert!(!surfaces.phrases.contains("frame-title"));
}

#[test]
fn javascript_surface_phrases_capture_labels_and_routes_only_in_ui_context() {
    let text = r#"test("Open settings panel is prose, not a surface", () => {});

export function SettingsLink() {
  return <a href="/orders/new" aria-label="Open settings panel">Orders</a>;
}

export function CartButton() {
  return <button aria-label="Remove from cart">Remove</button>;
}

export function ImportButton() {
  return <button aria-label="Import (CSV)">Import</button>;
}

test("flow", async ({ page }) => {
  await page.goto("/orders/new");
  await expect(page.getByLabel("Open settings panel")).toBeVisible();
  await expect(page.getByLabel("Remove from cart")).toBeVisible();
  await expect(page.getByLabel("Import (CSV)")).toBeVisible();
});
"#;

    let surfaces = extract_surfaces(text, "tsx");

    assert!(surfaces.phrases.contains("open-settings-panel"));
    assert!(surfaces.phrases.contains("remove-from-cart"));
    assert!(surfaces.phrases.contains("import-csv"));
    assert!(surfaces.phrases.contains("orders-new"));
    assert!(surfaces.tokens.contains("settings"));
    assert!(surfaces.tokens.contains("orders"));
    assert!(!surfaces.tokens.contains("prose"));
}

#[test]
fn javascript_surface_phrases_capture_bounded_jsx_visible_text() {
    let source = r#"export function ShellHint() {
  return (
    <div className="blueprint-canvas__hint" aria-live="polite">
      Дважды кликни по канвасу или нажми <kbd className="kbd">F</kbd> — появится новый кадр
    </div>
  );
}
"#;
    let test = r#"test("this prose is not a surface", async ({ page }) => {
  await expect(page.getByText("Дважды кликни по канвасу или нажми")).toBeVisible();
});
"#;
    let prose = r#"export function Plain() {
  return <p>Дважды кликни по канвасу или нажми</p>;
}
"#;

    let source_surfaces = extract_surfaces(source, "tsx");
    let test_surfaces = extract_surfaces(test, "tsx");
    let prose_surfaces = extract_surfaces(prose, "tsx");

    assert!(
        source_surfaces
            .phrases
            .contains("дважды-кликни-по-канвасу-или-нажми-f-—-появится-новый-кадр")
    );
    assert!(
        test_surfaces
            .phrases
            .contains("дважды-кликни-по-канвасу-или-нажми")
    );
    assert!(
        prose_surfaces.phrases.is_empty(),
        "visible text without a UI surface container should fail closed: {prose_surfaces:#?}"
    );
}

#[test]
fn javascript_surface_phrases_capture_accessible_labelledby_dialog_names() {
    let source = r#"export function ExportDialog() {
  return (
    <div role="dialog" aria-labelledby="export-title">
      <h2 id="export-title" style={{ fontSize: 18 }}>
        Экспорт
      </h2>
      <p>
        This explanatory copy is visible but is not the dialog accessible name.
      </p>
    </div>
  );
}
"#;
    let overcapture = r#"export function BrokenDialogName() {
  return (
    <div role="dialog" aria-labelledby="export-title">
      <h2 id="export-title"></h2>
      <p>Export files are generated locally</p>
    </div>
  );
}
"#;
    let test = r#"import { expect, test } from '@playwright/test';

test("export dialog opens", async ({ page }) => {
  await expect(page.getByRole('dialog', { name: 'Экспорт' })).toBeVisible();
});
"#;

    let source_surfaces = extract_surfaces(source, "tsx");
    let overcapture_surfaces = extract_surfaces(overcapture, "tsx");
    let test_surfaces = extract_surfaces(test, "tsx");

    assert!(
        source_surfaces
            .phrases
            .contains("a11y-role-dialog-name-экспорт")
    );
    assert!(
        test_surfaces
            .phrases
            .contains("a11y-role-dialog-name-экспорт")
    );
    assert!(source_surfaces.tokens.contains("экспорт"));
    assert!(
        source_surfaces
            .phrases
            .iter()
            .all(|phrase| !phrase.contains("explanatory-copy")),
        "only the labelled heading should become the dialog-name surface: {source_surfaces:#?}"
    );
    assert!(
        overcapture_surfaces
            .phrases
            .iter()
            .all(|phrase| !phrase.contains("a11y-role-dialog-name-export-files")),
        "text after the labelled element must not become the dialog-name surface: {overcapture_surfaces:#?}"
    );
}

#[test]
fn javascript_accessible_surfaces_resolve_dialog_contract_through_barrels() {
    let repo = tempfile::TempDir::new().expect("repo tempdir");
    std::fs::write(
        repo.path().join("package.json"),
        r#"{"name":"barrel-accessible-surface-fixture","private":true}"#,
    )
    .expect("write package");
    std::fs::create_dir_all(repo.path().join("src/features/studio")).expect("create feature dir");
    std::fs::create_dir_all(repo.path().join("src/design")).expect("create design dir");
    std::fs::write(
        repo.path().join("src/features/studio/export-dialog.tsx"),
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
    )
    .expect("write anchor");
    std::fs::write(
        repo.path().join("src/design/index.ts"),
        r#"export {
  Dialog,
  type ToastData,
} from './primitives'
"#,
    )
    .expect("write index");
    std::fs::write(
        repo.path().join("src/design/primitives.ts"),
        r#"export {
  Dialog,
  type ToastData,
} from './primitives-overlays'
"#,
    )
    .expect("write primitives");
    std::fs::write(
            repo.path().join("src/design/primitives-overlays.tsx"),
            r#"export type ToastData = { id: string };

export function Dialog({ open, onClose, labelledBy, children }) {
  if (!open) return null;
  return <div role="dialog" aria-modal="true" aria-labelledby={labelledBy} onClick={onClose}>{children}</div>;
}
"#,
        )
        .expect("write overlays");

    let project = load_project_with_cache(
        RootSelection::Exact(repo.path().to_path_buf()),
        CacheWriteMode::ReadOnly,
    )
    .expect("load project");
    let anchor = project
        .files
        .get("src/features/studio/export-dialog.tsx")
        .expect("anchor file");
    assert!(
        anchor
            .surface_phrases
            .contains("a11y-role-dialog-name-export"),
        "dialog accessible surface should resolve through multiline barrels: {:#?}",
        anchor.surface_phrases
    );
}
