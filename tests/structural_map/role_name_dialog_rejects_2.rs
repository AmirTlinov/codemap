#[test]
fn proof_rejects_same_line_native_role_and_aria_labelledby_on_different_elements() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return <><div role="dialog"></div><section aria-labelledby="export-title"><h2 id="export-title">Export</h2></section></>;
}
"#,
        None,
        "native dialog proof must require role and aria-labelledby on the same opening tag",
    );
}


#[test]
fn proof_rejects_same_line_component_contract_split_across_sibling_elements() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return <><div role="dialog"></div><section aria-labelledby={labelledBy}>{children}</section></>;
}
"#,
        ),
        "custom component contract must require role and aria-labelledby on the same opening tag",
    );
}


#[test]
fn proof_rejects_same_line_self_closing_label_text_from_sibling() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return <div role="dialog" aria-labelledby="export-title"><h2 id="export-title" /><p>Export</p></div>;
}
"#,
        None,
        "self-closing labelled element must not capture sibling text on the same line",
    );
}


#[test]
fn proof_rejects_dialog_label_from_unused_same_file_component() {
    assert_no_export_dialog_role_name_proof(
        r#"function UnusedExportTitle() {
  return <h2 id="export-title">Export</h2>;
}

export function ExportDialog() {
  return (
    <div role="dialog" aria-labelledby="export-title">
      <p>No proven label is rendered in this dialog subtree.</p>
    </div>
  );
}
"#,
        None,
        "dialog accessible-name proof must not use labels from unused same-file components",
    );
}


#[test]
fn proof_rejects_dialog_duplicate_labelledby_targets() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return (
    <div role="dialog" aria-labelledby="export-title">
      <h2 id="export-title">Import</h2>
      <h2 id="export-title">Export</h2>
    </div>
  );
}
"#,
        None,
        "duplicate aria-labelledby targets must fail closed instead of choosing one label",
    );
}


#[test]
fn proof_rejects_dialog_label_inside_jsx_control_flow_expression() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return (
    <div role="dialog" aria-labelledby="export-title">
      {false && <h2 id="export-title">Export</h2>}
    </div>
  );
}
"#,
        None,
        "dialog accessible-name proof must not trust labels hidden inside JSX control-flow expressions",
    );
}


#[test]
fn proof_rejects_dialog_duplicate_role_attr_override() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return (
    <div role="dialog" role="presentation" aria-labelledby="export-title">
      <h2 id="export-title">Export</h2>
    </div>
  );
}
"#,
        None,
        "duplicate role attrs must fail closed instead of trusting the first normalized role",
    );
}


#[test]
fn proof_rejects_dialog_duplicate_aria_labelledby_attr_override() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return (
    <div role="dialog" aria-labelledby="missing export-title" aria-labelledby="export-title">
      <h2 id="export-title">Export</h2>
    </div>
  );
}
"#,
        None,
        "duplicate aria-labelledby attrs must fail closed even when only one value is valid",
    );
}


#[test]
fn proof_rejects_dialog_duplicate_self_closing_label_target() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return (
    <div role="dialog" aria-labelledby="export-title">
      <h2 id="export-title" />
      <h2 id="export-title">Export</h2>
    </div>
  );
}
"#,
        None,
        "duplicate label targets must fail closed even when one target is self-closing",
    );
}


#[test]
fn proof_rejects_custom_dialog_label_from_unused_same_file_component() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

function UnusedExportTitle() {
  return <h2 id="export-title">Export</h2>;
}

export function ExportDialog() {
  return (
    <Dialog labelledBy="export-title">
      <p>No proven label is rendered in this dialog subtree.</p>
    </Dialog>
  );
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "custom dialog proof must not use labels from unused same-file components",
    );
}


#[test]
fn proof_rejects_dialog_markup_inside_template_string() {
    assert_no_export_dialog_role_name_proof(
        r#"const staticPreview = `<div role="dialog" aria-labelledby="export-title"><h2 id="export-title">Export</h2></div>`;

export function ExportDialog() {
  return null;
}
"#,
        None,
        "template-string markup must not create structural dialog accessible-name proof",
    );
}


#[test]
fn proof_rejects_custom_dialog_contract_from_template_string_render() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return (
    <Dialog labelledBy="export-title">
      <h2 id="export-title">Export</h2>
    </Dialog>
  );
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return `<div role="dialog" aria-labelledby="${labelledBy}">${children}</div>`;
}
"#,
        ),
        "component template-string render must not create structural dialog contract proof",
    );
}


#[test]
fn proof_rejects_custom_dialog_lowercase_labelledby_prop() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return (
    <Dialog labelledby="export-title">
      <h2 id="export-title">Export</h2>
    </Dialog>
  );
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "custom component labelledBy prop proof must be case-sensitive",
    );
}


#[test]
fn proof_rejects_custom_dialog_labelledby_overridden_by_spread() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

const override = { labelledBy: 'import-title' };

export function ExportDialog() {
  return (
    <Dialog labelledBy="export-title" {...override}>
      <h2 id="export-title">Export</h2>
      <h2 id="import-title">Import</h2>
    </Dialog>
  );
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "custom Dialog labelledBy with spread override must fail closed",
    );
}


#[test]
fn proof_rejects_dialog_component_contract_duplicate_aria_labelledby_attr_override() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" aria-labelledby={labelledBy} aria-labelledby="import-title">{children}</div>;
}
"#,
        ),
        "custom Dialog contract with duplicate aria-labelledby attrs must fail closed",
    );
}


#[test]
fn proof_rejects_imported_dialog_contract_when_consumer_shadows_jsx_tag() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  const Dialog = ({ children }) => <section>{children}</section>;
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "local JSX tag shadow must not inherit the imported Dialog contract",
    );
}


#[test]
fn proof_rejects_member_dialog_tag_as_imported_component_contract() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

const Other = {
  Dialog({ labelledBy, children }) {
    return <section aria-labelledby={labelledBy}>{children}</section>;
  },
};

export function ExportDialog() {
  return (
    <Other.Dialog labelledBy="export-title">
      <h2 id="export-title">Export</h2>
    </Other.Dialog>
  );
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "JSX member tags must not inherit a same-suffix imported component contract",
    );
}


#[test]
fn proof_rejects_alertdialog_source_for_dialog_e2e_role() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return <div role="alertdialog" aria-labelledby="export-title"><h2 id="export-title">Export</h2></div>;
}
"#,
        None,
        "alertdialog source role must not satisfy dialog e2e role",
    );
}


#[test]
fn proof_rejects_dialog_source_for_alertdialog_e2e_role() {
    assert_no_export_role_name_proof(
        r#"export function ExportDialog() {
  return <div role="dialog" aria-labelledby="export-title"><h2 id="export-title">Export</h2></div>;
}
"#,
        None,
        "alertdialog",
        "dialog source role must not satisfy alertdialog e2e role",
    );
}


#[test]
fn proof_links_alertdialog_source_to_alertdialog_e2e_role() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "alertdialog-accessible-proof-fixture",
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
  return <div role="alertdialog" aria-labelledby="export-title"><h2 id="export-title">Export</h2></div>;
}
"#,
    );
    write(
        &repo
            .path()
            .join("tests/e2e/canvas-blueprint-overlay-workflows.spec.ts"),
        r#"import { expect, test } from '@playwright/test';

test('command palette opens export alert dialog', async ({ page }) => {
  await page.goto('/studio');
  await expect(page.getByRole('alertdialog', { name: 'Export' })).toBeVisible();
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
        "alertdialog source should link to matching alertdialog e2e role/name proof: {proof:#}"
    );
}


#[test]
fn proof_rejects_dialog_component_local_labelledby_shadow_as_prop_mapping() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ children }) {
  const labelledBy = 'import-title';
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "local labelledBy binding inside component must not prove caller labelledBy prop mapping",
    );
}

