#[test]
fn proof_links_e2e_path_surface_to_non_ui_domain_anchor_without_imports() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/blueprint-canvas-rf-selection.ts"),
        "export function pickFocusForSelection(selection: Set<string>, orderedIds: string[]): string | null {\n  for (const id of orderedIds) {\n    if (selection.has(id)) return id;\n  }\n  return null;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/canvas-selection-focus-state.spec.ts"),
        "import { expect, test } from '@playwright/test';\n\ntest('selection focus state follows the current card focus', async ({ page }) => {\n  await page.goto('/studio');\n  await expect(page.locator('.frame-card').first()).toBeVisible();\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "e2e path surface fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/blueprint-canvas-rf-selection.ts",
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
                == "packages/app/tests/e2e/canvas-selection-focus-state.spec.ts"
                && surface["evidence"] == "e2e_path_surface"),
        "e2e specs with strong path/name overlap should remain visible as soft proof evidence: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "soft e2e path proof must not suppress broad fallback: {proof:#}"
    );

    let symbol_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/blueprint-canvas-rf-selection.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &symbol_proof);
    assert!(
        symbol_proof["proofs"]
            .as_array()
            .expect("symbol proofs")
            .iter()
            .any(|surface| surface["path"]
                == "packages/app/tests/e2e/canvas-selection-focus-state.spec.ts"
                && surface["evidence"] == "e2e_path_surface_owning_file"
                && surface["strength"] == "medium"),
        "symbol anchors without exact symbol proof should expose clearly labeled owning-file proof instead of broad fallback: {symbol_proof:#}"
    );
    assert!(
        !symbol_proof["fallback"]
            .as_array()
            .expect("symbol fallback")
            .is_empty(),
        "soft owning-file proof must not suppress broad fallback for symbol anchors: {symbol_proof:#}"
    );

    let symbol_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/blueprint-canvas-rf-selection.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &symbol_cone);
    assert!(
        symbol_cone["proof"]
            .as_array()
            .expect("symbol cone proof")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/tests/e2e/canvas-selection-focus-state.spec.ts"
                && edge["evidence"] == "e2e_path_surface_owning_file"
                && edge["strength"] == "medium"),
        "symbol cone should expose the same bounded owning-file proof edge as proof command: {symbol_cone:#}"
    );
}


#[test]
fn proof_rejects_broad_e2e_path_surface_without_specific_anchor_term() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/blueprint-canvas-rf-selection.ts"),
        "export function pickFocusForSelection(selection: Set<string>, orderedIds: string[]): string | null {\n  for (const id of orderedIds) {\n    if (selection.has(id)) return id;\n  }\n  return null;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/canvas-selection-breadcrumb.spec.ts"),
        "import { expect, test } from '@playwright/test';\n\ntest('canvas selection breadcrumb follows selected frames', async ({ page }) => {\n  await page.goto('/studio');\n  await expect(page.locator('.selection-breadcrumb')).toBeVisible();\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "broad e2e path surface fixture"],
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/blueprint-canvas-rf-selection.ts",
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
            .all(|surface| surface["path"]
                != "packages/app/tests/e2e/canvas-selection-breadcrumb.spec.ts"),
        "broad e2e path overlap should not become proof without a distinctive anchor term: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without structural e2e proof, broad fallback must stay visible: {proof:#}"
    );
}


#[test]
fn proof_rejects_direct_e2e_path_surface_when_anchor_has_code_consumer() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/blueprint-canvas-rf-selection.ts"),
        "export function pickFocusForSelection(selection: Set<string>, orderedIds: string[]): string | null {\n  for (const id of orderedIds) {\n    if (selection.has(id)) return id;\n  }\n  return null;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/blueprint-canvas-rf.tsx"),
        "import { pickFocusForSelection } from './blueprint-canvas-rf-selection';\n\nexport function BlueprintCanvasRf() {\n  const focusId = pickFocusForSelection(new Set(['frame-1']), ['frame-1']);\n  return <div data-testid=\"canvas-selection-root\">{focusId}</div>;\n}\n",
    );
    for spec in [
        "canvas-selection-breadcrumb.spec.ts",
        "canvas-selection-clipboard.spec.ts",
        "canvas-selection-frame-board.spec.ts",
    ] {
        write(
            &repo.path().join(format!("packages/app/tests/e2e/{spec}")),
            "import { expect, test } from '@playwright/test';\n\ntest('canvas selection broad flow', async ({ page }) => {\n  await page.goto('/studio');\n  await expect(page.locator('[data-testid=\"canvas-selection-root\"]')).toBeVisible();\n});\n",
        );
    }
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "consumer plus broad e2e fixture"],
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/blueprint-canvas-rf-selection.ts",
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
            .all(|surface| surface["evidence"] != "e2e_path_surface"),
        "when an anchor has a code consumer, path-only e2e overlap must not pretend to prove the helper directly: {proof:#}"
    );
}
