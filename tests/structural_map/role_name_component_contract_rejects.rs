#[test]
fn proof_rejects_dialog_component_nested_helper_labelledby_param_as_prop_mapping() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ children }) {
  function helper({ labelledBy }) {
    return labelledBy;
  }
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "nested helper params must not prove the exported component labelledBy prop mapping",
    );
}


#[test]
fn proof_rejects_dialog_contract_from_unused_nested_helper_jsx() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  function helper() {
    return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
  }
  return <section>{children}</section>;
}
"#,
        ),
        "unused nested helper JSX must not prove the component rendered dialog contract",
    );
}


#[test]
fn proof_rejects_arrow_component_nested_helper_params_as_own_labelledby_prop() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export const Dialog = () => {
  function helper({ labelledBy }) {
    return <div role="dialog" aria-labelledby={labelledBy}>Export</div>;
  }
  return <section>Export</section>;
};
"#,
        ),
        "nested helper params inside arrow component must not become the component prop list",
    );
}


#[test]
fn proof_rejects_object_literal_export_with_render_helper_as_component_contract() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export const Dialog = {
  render: function helper({ labelledBy, children }) {
    return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
  }
};
"#,
        ),
        "object literal render helpers must not prove an exported component contract",
    );
}


#[test]
fn proof_rejects_dialog_component_data_contract_string_as_aria_forwarding() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" data-contract="aria-labelledby={labelledBy}">{children}</div>;
}
"#,
        ),
        "string literals containing aria-labelledby={labelledBy} must not prove real aria forwarding",
    );
}


#[test]
fn proof_rejects_nested_config_labelledby_as_direct_component_prop_mapping() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ config: { labelledBy }, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "nested config.labelledBy destructuring must not prove caller labelledBy prop mapping",
    );
}


#[test]
fn proof_rejects_dialog_component_that_forwards_labelledby_but_drops_children() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ open, labelledBy }) {
  if (!open) return null;
  return <div role="dialog" aria-labelledby={labelledBy} />;
}
"#,
        ),
        "labelledBy forwarding without rendering children must not prove caller accessible name",
    );
}


#[test]
fn proof_rejects_dialog_component_contract_with_rendered_spread_override() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children, ...props }) {
  return <div role="dialog" aria-labelledby={labelledBy} {...props}>{children}</div>;
}
"#,
        ),
        "rendered dialog opening with spread props must fail closed for accessible-name proof",
    );
}


#[test]
fn proof_rejects_dialog_component_when_any_labelledby_branch_drops_children() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title" compact><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ labelledBy, children, compact }) {
  if (compact) return <div role="dialog" aria-labelledby={labelledBy} />;
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "one good dialog return must not prove a component whose other labelled dialog branch drops children",
    );
}


#[test]
fn proof_rejects_dialog_component_when_any_render_branch_is_not_dialog() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog inline labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ inline, labelledBy, children }) {
  if (inline) return <section>{children}</section>;
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "one good dialog return must not prove a component whose other render branch is not a dialog",
    );
}


#[test]
fn proof_rejects_dialog_component_with_braced_non_dialog_if_branch() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog inline labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ inline, labelledBy, children }) {
  if (inline) {
    return <section>{children}</section>;
  }
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "braced conditional render branches must fail closed instead of proving only the later dialog return",
    );
}


#[test]
fn proof_rejects_dialog_component_with_ternary_non_dialog_branch() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog inline labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ inline, labelledBy, children }) {
  return inline
    ? <section>{children}</section>
    : <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "ternary render branches must fail closed unless the branch parser can prove every branch",
    );
}


#[test]
fn proof_rejects_dialog_component_with_logical_non_dialog_branch() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog inline labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ inline, labelledBy, children }) {
  return inline && <section>{children}</section> || <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "logical render control-flow must fail closed unless every branch can be proven",
    );
}


#[test]
fn proof_rejects_dialog_component_with_switch_non_dialog_branch() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog mode="inline" labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ mode, labelledBy, children }) {
  switch (mode) {
    case 'inline':
      return <section>{children}</section>;
    default:
      break;
  }
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "switch render control-flow must fail closed instead of trusting only the later dialog return",
    );
}


#[test]
fn proof_rejects_dialog_component_with_opaque_call_around_dialog_jsx() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"function ignore(node) {
  return <section>Not a dialog</section>;
}

export function Dialog({ labelledBy, children }) {
  return ignore(<div role="dialog" aria-labelledby={labelledBy}>{children}</div>);
}
"#,
        ),
        "opaque call expressions around dialog JSX must not prove the rendered component contract",
    );
}


#[test]
fn proof_rejects_dialog_component_nested_under_custom_wrapper_boundary() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"function Wrapper() {
  return <section>Not a dialog</section>;
}

export function Dialog({ labelledBy, children }) {
  return <Wrapper>{<div role="dialog" aria-labelledby={labelledBy}>{children}</div>}</Wrapper>;
}
"#,
        ),
        "dialog JSX under an opaque custom component boundary must not prove rendered output",
    );
}


#[test]
fn proof_rejects_barrel_star_dialog_when_explicit_reexport_overrides_it() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "dialog-barrel-conflict-fixture",
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
        r#"import { Dialog } from '../../design/index';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
    );
    write(
        &repo.path().join("src/design/index.ts"),
        r#"export * from './dialog-good';
export { Dialog } from './dialog-bad';
"#,
    );
    write(
        &repo.path().join("src/design/dialog-good.tsx"),
        r#"export function Dialog({ labelledBy, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
    );
    write(
        &repo.path().join("src/design/dialog-bad.tsx"),
        r#"export function Dialog({ children }) {
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
        "explicit bad re-export must override star-exported good dialog contract: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, broad fallback must remain visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_multilabel_aria_labelledby_as_individual_exact_name() {
    assert_no_export_dialog_role_name_proof(
        r#"export function ExportDialog() {
  return (
    <div role="dialog" aria-labelledby="export-title suffix-title">
      <h2 id="export-title">Export</h2>
      <span id="suffix-title">Settings</span>
    </div>
  );
}
"#,
        None,
        "multi-id aria-labelledby must not create separate exact role/name proof surfaces",
    );
}


#[test]
fn proof_rejects_native_label_under_custom_component_boundary() {
    assert_no_export_dialog_role_name_proof(
        r#"function Wrapper({ children }) {
  return null;
}

export function ExportDialog() {
  return (
    <div role="dialog" aria-labelledby="export-title">
      <Wrapper><h2 id="export-title">Export</h2></Wrapper>
    </div>
  );
}
"#,
        None,
        "native aria-labelledby label under an opaque custom component boundary must not prove the rendered accessible name",
    );
}


#[test]
fn proof_rejects_dialog_component_alias_destructured_labelledby_as_prop_mapping() {
    assert_no_export_dialog_role_name_proof(
        r#"import { Dialog } from '../../design/dialog';

export function ExportDialog() {
  return <Dialog labelledBy="export-title"><h2 id="export-title">Export</h2></Dialog>;
}
"#,
        Some(
            r#"export function Dialog({ labelledBy: id, children }) {
  return <div role="dialog" aria-labelledby={labelledBy}>{children}</div>;
}
"#,
        ),
        "alias destructuring must not prove direct labelledBy prop mapping",
    );
}

