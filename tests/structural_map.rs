use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn codemap() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codemap"))
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {:?} failed", args);
}

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write file");
}

fn assert_schema(schema_rel: &str, instance: &Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema_text = fs::read_to_string(root.join(schema_rel)).expect("schema should exist");
    let schema: Value = serde_json::from_str(&schema_text).expect("schema json");
    let validator = jsonschema::validator_for(&schema).expect("schema should compile");
    validator
        .validate(instance)
        .unwrap_or_else(|error| panic!("{schema_rel} rejected instance: {error}"));
}

fn fixture() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "map-fixture",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": { "test": "pnpm test", "typecheck": "tsc -b" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/package.json"),
        r#"{
  "name": "@fixture/replay",
  "version": "1.0.0",
  "main": "src/index.ts",
  "exports": { ".": "./src/index.ts" },
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "private": true,
  "dependencies": { "@fixture/replay": "workspace:*" },
  "scripts": { "test": "vitest run", "test:e2e": "playwright test" }
}
"#,
    );
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1

boundaries:
  forbidden:
    - from: packages/app/src/**
      to: packages/replay/src/internal.ts
      reason: app must consume replay public exports
"#,
    );
    write(
        &repo.path().join("packages/replay/src/index.ts"),
        "export { seek } from './session';\nexport type { FrameDto } from './types';\n",
    );
    write(
        &repo.path().join("packages/replay/src/types.ts"),
        "export interface FrameDto {\n  frame: number;\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/timeline.ts"),
        "export class Timeline {\n  frameAt(cursor: number) {\n    return cursor;\n  }\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor) };\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/internal.ts"),
        "export const internalValue = 1;\n",
    );
    write(
        &repo.path().join("packages/replay/tests/session.test.ts"),
        "import { seek } from '../src/session';\n\ntest('seek maps frame', () => {\n  expect(seek(2).frame).toBe(2);\n});\n",
    );
    write(
        &repo.path().join("packages/replay/tests/e2e/seek.e2e.ts"),
        "import { seek } from '../../src/session';\n\ntest('e2e seek maps frame', () => {\n  expect(seek(3).frame).toBe(3);\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/replay/tests/session-surface-smoke.test.ts"),
        "test('session smoke checks package wiring', () => {\n  expect('session').toBeTruthy();\n});\n",
    );
    write(
        &repo.path().join("packages/replay/tests/support/setup.ts"),
        "export const setup = true;\n",
    );
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { seek } from '@fixture/replay';\n\nexport const frame = seek(1).frame;\n",
    );
    write(
        &repo.path().join("packages/app/src/badInternal.ts"),
        "import { internalValue } from '../../replay/src/internal';\n\nexport const bad = internalValue;\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/frame-title-control.tsx"),
        "export function FrameTitleControl() {\n  return <button className=\"frame-title-control\">Title</button>;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/settings-button.tsx"),
        "export function SettingsButton() {\n  return <button aria-label=\"Open settings panel\">Settings</button>;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/comment-only.tsx"),
        "/*\n  <button aria-label=\"Open settings panel\" data-testid=\"submit-order-button\">Settings</button>\n  await page.goto('/orders/new');\n*/\nexport function CommentOnly() {\n  return <div />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/orders-link.tsx"),
        "export function OrdersLink() {\n  return <a href=\"/orders/new\">Orders</a>;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/cart-button.tsx"),
        "export function CartButton() {\n  return <button aria-label=\"Remove from cart\">Remove</button>;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/import-csv-button.tsx"),
        "export function ImportCsvButton() {\n  return <button aria-label=\"Import (CSV)\">Import</button>;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/mixed-layout-panel.ts"),
        "export const mixedLayoutPanelSelector = '.mixed-layout-panel';\n",
    );
    write(
        &repo.path().join("packages/app/src/features/studio/foo.ts"),
        "export const fooTarget = 'foo';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/import-only-widget.tsx"),
        "import widget from '@app/aria-label-open-settings-panel';\nimport {\n  multi,\n} from '@app/data-testid-submit-order-button';\nconst lazy = import ('@app/route-orders-new');\nconst required = require ('@app/class-name-open-settings-panel');\nconst bareLazy = import ('aria-label-open-settings-panel');\nconst bareRequired = require ('data-testid-submit-order-button');\nconst commentedLazy = import(/* webpackChunkName: \"settings\" */ 'aria-label-open-settings-panel');\n\nexport function ImportOnlyWidget() {\n  return widget ?? multi ?? lazy ?? required ?? bareLazy ?? bareRequired ?? commentedLazy;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/frame-title-placement.test.ts"),
        "test('frame title placement persists', () => {\n  expect('frame-title').toContain('title');\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/canvas-blueprint-title-drag.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('blueprint canvas title drag keeps title attached', async ({ page }) => {\n  await page.goto('/studio');\n  await expect(page.locator('.frame-title-control')).toBeVisible();\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/studio-flow.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('studio surface keeps frame title control visible', async ({ page }) => {\n  await page.goto('/studio');\n  await expect(page.getByTestId('frame-title-control')).toBeVisible();\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/canvas-text-document.test.ts"),
        "test('frame title text document stores inline mode', () => {\n  expect('frame-title').toContain('title');\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/canvas-blueprint-rail-settings.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('canvas rail settings stay visible', async ({ page }) => {\n  await page.goto('/studio');\n  await expect(page.locator('.canvas-markup-rail__marker-chip')).toBeVisible();\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/accessibility-flow.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('settings button is accessible', async ({ page }) => {\n  await expect(page.getByLabel('Open settings panel')).toBeVisible();\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/orders-route.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('orders route opens', async ({ page }) => {\n  await page.goto('/orders/new');\n  await expect(page).toHaveURL('/orders/new');\n});\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/cart-flow.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('cart action is accessible', async ({ page }) => {\n  await expect(page.getByLabel('Remove from cart')).toBeVisible();\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/import-csv-flow.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('csv import action is accessible', async ({ page }) => {\n  await expect(page.getByLabel('Import (CSV)')).toBeVisible();\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/import-only-flow.spec.ts"),
        "import '@app/aria-label-open-settings-panel';\nimport {\n  multi,\n} from '@app/data-testid-submit-order-button';\nconst lazy = import ('@app/route-orders-new');\nconst required = require ('@app/class-name-open-settings-panel');\nconst bareLazy = import ('aria-label-open-settings-panel');\nconst bareRequired = require ('data-testid-submit-order-button');\nconst commentedLazy = import(/* webpackChunkName: \"settings\" */ 'aria-label-open-settings-panel');\nimport { test, expect } from '@playwright/test';\n\ntest('unrelated flow has no shared UI surface', async ({ page }) => {\n  await expect(page.locator('.unrelated-widget')).toHaveCount(0);\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/support/canvas-blueprint.ts"),
        "export const loadCanvas = true;\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/support/mixed-layout-page.ts"),
        "import { mixedLayoutPanelSelector } from '../../../src/features/studio/mixed-layout-panel';\n\nexport async function openMixedLayout(page) {\n  await page.locator(mixedLayoutPanelSelector).click();\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/support/foo-page.ts"),
        "import { fooTarget } from '../../../src/features/studio/foo';\n\nexport async function openFoo(page) {\n  await page.evaluate((value) => value, fooTarget);\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/mixed-layout.spec.ts"),
        "import { test } from '@playwright/test';\nimport { openMixedLayout } from './support/mixed-layout-page';\n\ntest('mixed layout panel opens from helper', async ({ page }) => {\n  await openMixedLayout(page);\n});\n",
    );
    write(
        &repo.path().join("packages/app/tests/e2e/foo.spec.ts"),
        "import { test } from '@playwright/test';\nimport { openFoo } from './support/foo-page';\n\ntest('foo opens through helper', async ({ page }) => {\n  await openFoo(page);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    (repo, cache)
}

fn run_json(repo: &Path, cache: &Path, args: &[&str]) -> Value {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid json")
}

#[test]
fn help_exposes_only_map_first_commands() {
    let output = codemap().arg("--help").output().expect("help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help utf8");
    for command in ["ls", "cone", "impact", "proof", "graph", "boundaries"] {
        assert!(stdout.contains(command), "help should expose {command}");
    }
    for forbidden in ["start", "locate", "find", "verify", "widen", "read_first"] {
        assert!(
            !stdout.contains(forbidden),
            "help must not expose removed surface {forbidden}"
        );
    }
}

#[test]
fn root_ls_is_a_bounded_domain_and_package_map() {
    let (repo, cache) = fixture();
    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    assert_eq!(json["kind"], "ls_report");
    assert_eq!(json["schema_version"], "2");
    assert_eq!(json["mode"], "directory");
    let surfaces = json["directory"].as_array().expect("directory surfaces");
    assert!(surfaces.iter().any(|surface| surface["kind"] == "domain"));
    assert!(
        surfaces
            .iter()
            .any(|surface| surface["kind"] == "package:javascript")
    );
    assert!(surfaces.iter().any(|surface| surface["kind"] == "dir"));
    assert!(
        json["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["type"] == "package_internal"
                && edge["from"] == "packages/app/"
                && edge["to"] == "packages/replay/")
    );
    assert_eq!(json.get("read_first"), None);
    assert_eq!(json.get("confidence"), None);

    let tests_scope = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/replay/tests", "--format", "json"],
    );
    let test_surfaces = tests_scope["directory"].as_array().expect("test surfaces");
    assert!(
        test_surfaces
            .iter()
            .any(|surface| surface["kind"] == "e2e_test")
    );
    assert!(
        test_surfaces
            .iter()
            .any(|surface| surface["kind"] == "test_support")
    );
}

#[test]
fn file_ls_and_cone_show_symbols_edges_proof_and_boundary() {
    let (repo, cache) = fixture();
    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["anchor"]["path"], "packages/replay/src/session.ts");
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "seek" && symbol["kind"] == "function")
    );
    assert!(
        ls["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(
                |edge| edge["from"] == "packages/replay/tests/session.test.ts"
                    && edge["type"] == "tests"
            )
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/badInternal.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["boundary"]
            .as_array()
            .expect("boundary")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/badInternal.ts"
                && edge["to"] == "packages/replay/src/internal.ts"
                && edge["strength"] == "hard")
    );
}

#[test]
fn impact_and_proof_are_structural_without_structural_flag() {
    let (repo, cache) = fixture();
    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/types.ts",
            "--depth",
            "2",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    assert_eq!(impact["kind"], "impact_report");
    assert_eq!(impact["schema_version"], "2");
    let cluster = &impact["clusters"][0];
    assert_eq!(cluster["risk"], "high");
    assert!(
        cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/session.ts")
    );
    assert!(
        cluster["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/tests/session.test.ts")
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/session.ts",
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
            .any(
                |proof| proof["path"] == "packages/replay/tests/session.test.ts"
                    && proof["command"]
                        .as_str()
                        .unwrap_or_default()
                        .contains("vitest run")
            )
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|proof| proof["path"] != "packages/replay/tests/session-surface-smoke.test.ts"),
        "token-only unit proof should stay hidden when direct import proof exists"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|proof| proof["path"] != "packages/replay/tests/support/setup.ts"),
        "test support files are map surfaces, not runnable proof"
    );
}

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
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "broad fallback should stay hidden when file-level proof commands exist"
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
fn proof_links_mixed_e2e_layout_through_test_support_import_chain() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/mixed-layout-panel.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/mixed-layout.spec.ts"
            && surface["evidence"] == "test_support_import"
            && surface["strength"] == "high"
            && surface["command"]
                .as_str()
                .unwrap_or_default()
                .contains("test:e2e")),
        "e2e spec should link through test support/page-object import chain, not fallback: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "support import chain should avoid broad fallback: {proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/mixed-layout-panel.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof edges")
            .iter()
            .any(
                |edge| edge["from"] == "packages/app/tests/e2e/mixed-layout.spec.ts"
                    && edge["evidence"] == "test_support_import"
            ),
        "cone should show the same structural proof edge: {cone:#}"
    );
}

#[test]
fn support_import_chain_beats_matching_test_name_for_e2e_specs() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/foo.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(
            |surface| surface["path"] == "packages/app/tests/e2e/foo.spec.ts"
                && surface["evidence"] == "test_support_import"
        ),
        "e2e support import chain is stronger map evidence than matching test name: {proof:#}"
    );
    assert!(
        proofs.iter().all(
            |surface| !(surface["path"] == "packages/app/tests/e2e/foo.spec.ts"
                && surface["evidence"] == "test_name")
        ),
        "matching e2e spec name must not mask the import chain: {proof:#}"
    );
}

#[test]
fn python_proof_without_package_manifest_uses_pytest_file_and_skips_init_support() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo
            .path()
            .join("economy_analytics/metric_contract/surface_specs/catalog.py"),
        "SURFACES = {}\n",
    );
    write(
        &repo.path().join("tests/__init__.py"),
        "from economy_analytics.metric_contract.surface_specs.catalog import SURFACES\n",
    );
    write(
        &repo.path().join("tests/economy_analytics/__init__.py"),
        "from economy_analytics.metric_contract.surface_specs.catalog import SURFACES\n",
    );
    write(
        &repo.path().join("tests/economy_analytics/test_catalog.py"),
        "from economy_analytics.metric_contract.surface_specs.catalog import SURFACES\n\n\ndef test_catalog_exports_surfaces():\n    assert SURFACES == {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "economy_analytics/metric_contract/surface_specs/catalog.py",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(
            |surface| surface["path"] == "tests/economy_analytics/test_catalog.py"
                && surface["evidence"] == "test_import"
                && surface["command"] == "pytest tests/economy_analytics/test_catalog.py"
        ),
        "python test file proof should be runnable without package manifest: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "tests/__init__.py"
                && surface["path"] != "tests/economy_analytics/__init__.py"),
        "python package marker files are test support, not proof: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "file-level pytest proof should suppress broad fallback: {proof:#}"
    );
}

#[test]
fn swift_package_manifest_surfaces_packages_scripts_and_local_path_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Package.swift"),
        r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "HostApp",
    dependencies: [
        .package(path: "Packages/Core")
    ],
    targets: [
        .executableTarget(name: "HostApp", dependencies: ["Core"])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/HostApp/main.swift"),
        "import Foundation\nprint(\"host\")\n",
    );
    write(
        &repo.path().join("Packages/Core/Package.swift"),
        r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "Core",
    targets: [
        .target(name: "Core")
    ]
)
"#,
    );
    write(
        &repo.path().join("Packages/Core/Sources/Core/Core.swift"),
        "public struct Core {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let status = run_json(repo.path(), cache.path(), &["status", "--format", "json"]);
    assert_schema("schemas/status.schema.json", &status);
    assert_eq!(status["package_manager"], "swift");
    assert!(
        status["scripts"]
            .as_array()
            .expect("scripts")
            .iter()
            .any(|script| script.as_str().unwrap_or_default() == "swift test")
    );

    let ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        ls["directory"]
            .as_array()
            .expect("directory")
            .iter()
            .any(|surface| surface["kind"] == "package:swift"
                && surface["examples"]
                    .as_array()
                    .expect("examples")
                    .iter()
                    .any(|example| example == "Package.swift")),
        "root map should surface SwiftPM package manifests: {ls:#}"
    );
    assert!(
        ls["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["from"] == "Package.swift"
                && edge["to"] == "Packages/"
                && edge["type"] == "package_internal"
                && edge["evidence"] == "package_manifest:Core"),
        "SwiftPM local path dependencies should become package graph edges: {ls:#}"
    );
}

#[test]
fn proof_does_not_treat_module_specifiers_as_ui_surfaces() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/import-only-widget.tsx",
            "--format",
            "json",
        ],
    );

    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/e2e/import-only-flow.spec.ts"),
        "module specifier strings must not become e2e UI proof: {proof:#}"
    );
}

#[test]
fn proof_does_not_treat_multiline_comments_as_ui_surfaces() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/comment-only.tsx",
            "--format",
            "json",
        ],
    );

    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs
            .iter()
            .all(|surface| surface["evidence"] != "e2e_surface_phrase"),
        "commented-out UI surfaces must not become e2e proof: {proof:#}"
    );
    assert!(
        proofs.iter().all(|surface| surface["path"]
            != "packages/app/tests/e2e/accessibility-flow.spec.ts"
            && surface["path"] != "packages/app/tests/e2e/orders-route.spec.ts"),
        "commented aria labels/routes must not link proof: {proof:#}"
    );
}

#[test]
fn proof_links_aria_labels_and_routes_as_exact_surfaces() {
    let (repo, cache) = fixture();

    let label_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/settings-button.tsx",
            "--format",
            "json",
        ],
    );
    let label_proofs = label_proof["proofs"].as_array().expect("label proofs");
    assert!(
        label_proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/accessibility-flow.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase"),
        "aria-label/getByLabel exact surface should become e2e proof: {label_proof:#}"
    );

    let route_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/orders-link.tsx",
            "--format",
            "json",
        ],
    );
    let route_proofs = route_proof["proofs"].as_array().expect("route proofs");
    assert!(
        route_proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/orders-route.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase"),
        "exact shared two-segment routes should become e2e proof: {route_proof:#}"
    );

    let cart_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/cart-button.tsx",
            "--format",
            "json",
        ],
    );
    let cart_proofs = cart_proof["proofs"].as_array().expect("cart proofs");
    assert!(
        cart_proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/cart-flow.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase"),
        "aria labels containing `from` should not be mistaken for import syntax: {cart_proof:#}"
    );

    let import_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/import-csv-button.tsx",
            "--format",
            "json",
        ],
    );
    let import_proofs = import_proof["proofs"].as_array().expect("import proofs");
    assert!(
        import_proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/import-csv-flow.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase"),
        "aria labels containing `Import (` should not be mistaken for dynamic import syntax: {import_proof:#}"
    );
}

#[test]
fn flat_huge_directory_ls_stays_bounded_without_expanding_the_galaxy() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "flat-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    for index in 0..80 {
        write(
            &repo.path().join(format!("src/flat/module-{index:02}.ts")),
            &format!("export const module{index:02} = {index};\n"),
        );
    }
    write(
        &repo.path().join("src/flat/deep/nested-owner.ts"),
        "export function nestedOwner() { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/flat", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &json);
    let surfaces = json["directory"].as_array().expect("directory surfaces");
    let source_surface = surfaces
        .iter()
        .find(|surface| surface["kind"] == "source")
        .expect("source surface");
    assert_eq!(source_surface["count"], 80);
    assert!(
        source_surface["examples"]
            .as_array()
            .expect("examples")
            .len()
            <= 5,
        "flat directory examples must stay bounded"
    );
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"] == "recursive files below this level hidden"),
        "recursive detail must stay hidden unless explicitly expanded"
    );
    assert_eq!(json.get("read_first"), None);
}

#[test]
fn anchors_validate_reports_summary_and_actionable_warnings() {
    let (repo, cache) = fixture();
    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["kind"], "anchor_validation");
    assert_eq!(validation["schema_version"], "3");
    assert_eq!(validation["ok"], true);
    assert_eq!(validation["summary"]["forbidden_boundaries"], 1);
    assert_eq!(validation["summary"]["concepts"], 0);
    assert!(
        validation["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .any(|warning| warning
                .as_str()
                .unwrap_or_default()
                .contains("no recovery steps")),
        "boundary warnings should explain why violations would be less actionable: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "forbidden_boundary"
                && detail["id"] == "#1"
                && detail["status"] == "warning"
                && detail["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("`from` matches")),
        "anchor validation should explain how boundary patterns resolved: {validation:#}"
    );
}

#[test]
fn anchors_validate_warning_details_do_not_contradict_ok_report() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
concepts:
  generated.assets:
    role: generated_boundary
    files:
      - src/generated/**/*.ts
boundaries:
  forbidden:
    - from: src/generated/**
      to: tests/missing/**
      reason: generated code must stay isolated
      recovery:
        - update generator
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], true);
    assert!(
        validation["problems"]
            .as_array()
            .expect("problems")
            .is_empty(),
        "fixture should only produce warnings: {validation:#}"
    );
    assert!(
        !validation["warnings"]
            .as_array()
            .expect("warnings")
            .is_empty(),
        "zero-match globs should stay visible as warnings: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .all(|detail| detail["status"] != "problem"),
        "details must not report problems when top-level problems are empty: {validation:#}"
    );
}

#[test]
fn anchors_validate_exact_boundary_paths_count_unique_targets() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: src/app.ts
      to: tests/app.test.ts
      reason: app code must not import test code
      recovery:
        - move shared helper to src/test-support
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    write(
        &repo.path().join("tests/app.test.ts"),
        "import { app } from '../src/app';\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], true);
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "forbidden_boundary"
                && detail["status"] == "ok"
                && detail["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("`from` matches 1; `to` matches 1;")),
        "exact boundary paths should count unique resolved targets, not mechanisms: {validation:#}"
    );
}

#[test]
fn anchors_validate_glob_boundary_paths_count_unique_targets() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"anchor-count-fixture","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: "*.json"
      to: src/app.ts
      reason: manifests must not drive app code directly
      recovery:
        - read manifest through config adapter
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], true);
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "forbidden_boundary"
                && detail["status"] == "ok"
                && detail["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("`from` matches 1; `to` matches 1;")),
        "glob boundary paths should count unique targets, not file/manifest mechanisms: {validation:#}"
    );
}

#[test]
fn anchors_validate_rejected_config_details_report_problem() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 2
boundaries:
  forbidden:
    - from: src/**
      to: tests/**
      reason: fixture
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], false);
    assert!(
        validation["problems"]
            .as_array()
            .expect("problems")
            .iter()
            .any(|problem| problem
                .as_str()
                .unwrap_or_default()
                .contains("unsupported .ctx version `2`")),
        "rejected config should stay visible as a top-level problem: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "config"
                && detail["id"] == ".ctx.yml"
                && detail["status"] == "problem"
                && detail["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("unsupported .ctx version `2`")),
        "rejected config should produce a problem detail: {validation:#}"
    );
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .all(|detail| detail["id"] != "zero-config"),
        "invalid config should not be explained as zero-config: {validation:#}"
    );
    assert!(
        validation["warnings"]
            .as_array()
            .expect("warnings")
            .iter()
            .all(|warning| !warning
                .as_str()
                .unwrap_or_default()
                .contains("no .ctx.yml found")),
        "invalid config should not emit zero-config warnings: {validation:#}"
    );
}

#[test]
fn anchors_validate_mixed_config_details_scope_status_to_each_config() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
domain:
  id: app
  path: src
"#,
    );
    write(
        &repo.path().join("packages/bad/.ctx.yml"),
        r#"version: 2
domain:
  id: bad
  path: src
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], false);
    let details = validation["details"].as_array().expect("details");
    assert!(
        details.iter().any(|detail| detail["kind"] == "config"
            && detail["id"] == ".ctx.yml"
            && detail["status"] == "ok"),
        "valid loaded config should keep ok detail even when another config is rejected: {validation:#}"
    );
    assert!(
        details.iter().any(|detail| detail["kind"] == "config"
            && detail["id"] == "packages/bad/.ctx.yml"
            && detail["status"] == "problem"),
        "rejected nested config should carry the problem detail: {validation:#}"
    );
}

#[test]
fn anchors_validate_explains_resolved_domains_concepts_and_verification() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
domain:
  id: app
  path: src
concepts:
  app.entry:
    role: source_of_truth
    files:
      - src/app.ts
    invariants:
      - deterministic
boundaries:
  forbidden:
    - from: src/**
      to: tests/**
      reason: app code must not import test code
      recovery:
        - move helper to src/test-support
verification:
  default:
    - pnpm test
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    write(
        &repo.path().join("tests/app.test.ts"),
        "import { app } from '../src/app';\n\ntest('app', () => expect(app).toBe(1));\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let validation = run_json(
        repo.path(),
        cache.path(),
        &["anchors", "validate", "--format", "json"],
    );
    assert_schema("schemas/anchor-validation.schema.json", &validation);
    assert_eq!(validation["ok"], true);
    let details = validation["details"].as_array().expect("details");
    assert!(
        details.iter().any(|detail| detail["kind"] == "domain"
            && detail["id"] == "app"
            && detail["status"] == "ok"
            && detail["message"]
                .as_str()
                .unwrap_or_default()
                .contains("path `src` exists")),
        "domain detail should explain resolved path: {validation:#}"
    );
    assert!(
        details.iter().any(|detail| detail["kind"] == "concept"
            && detail["id"] == "app.entry"
            && detail["status"] == "ok"
            && detail["message"]
                .as_str()
                .unwrap_or_default()
                .contains("exact files resolved: 1")),
        "concept detail should explain file and invariant resolution: {validation:#}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail["kind"] == "verification_default"
                && detail["status"] == "ok"
                && detail["message"] == "pnpm test"),
        "verification defaults should be visible in details: {validation:#}"
    );
}

#[test]
fn boundaries_check_transitive_package_dependency_graph_without_imports() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "package-boundary-fixture",
  "private": true,
  "workspaces": ["packages/*"]
}
"#,
    );
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: packages/app/src/**
      to: packages/replay/src/**
      reason: app must consume replay through the public API only
      recovery:
        - remove transitive package dependency
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "dependencies": { "@fixture/bridge": "workspace:*" }
}
"#,
    );
    write(
        &repo.path().join("packages/bridge/package.json"),
        r#"{
  "name": "@fixture/bridge",
  "dependencies": { "@fixture/replay": "workspace:*" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/package.json"),
        r#"{ "name": "@fixture/replay" }
"#,
    );
    write(
        &repo.path().join("packages/app/src/index.ts"),
        "export const app = true;\n",
    );
    write(
        &repo.path().join("packages/bridge/src/index.ts"),
        "export const bridge = true;\n",
    );
    write(
        &repo.path().join("packages/replay/src/index.ts"),
        "export const replay = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["boundaries", "--format", "json"])
        .output()
        .expect("codemap should run");
    assert!(
        !output.status.success(),
        "boundary violations should fail closed"
    );
    let boundaries: Value =
        serde_json::from_slice(&output.stdout).expect("boundary report should be json");
    assert_schema("schemas/boundaries.schema.json", &boundaries);
    assert!(
        boundaries["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(
                |finding| finding["provenance"] == "package_manifest_transitive+semantic_anchor"
                    && finding["from"] == "packages/app/package.json"
                    && finding["to"] == "packages/replay/package.json"
                    && finding["reason"]
                        .as_str()
                        .unwrap_or_default()
                        .contains("@fixture/bridge -> @fixture/replay")
            ),
        "transitive package manifest boundary must be reported without source imports: {boundaries:#}"
    );
}

#[test]
fn schema_manifest_has_no_removed_router_contracts_and_schema_command_is_side_effect_free() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest_text =
        fs::read_to_string(root.join("schemas/manifest.json")).expect("manifest should exist");
    let manifest: Value = serde_json::from_str(&manifest_text).expect("manifest json");
    assert_eq!(manifest["version"], 2);
    let schemas = manifest["schemas"].as_array().expect("schemas");
    let kinds = schemas
        .iter()
        .map(|entry| entry["kind"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    for forbidden in [
        "capsule",
        "find",
        "verify",
        "locate",
        "explain",
        "widen",
        "impact-v2",
    ] {
        assert!(!kinds.iter().any(|kind| kind == forbidden));
    }

    let actual_schema_files = fs::read_dir(root.join("schemas"))
        .expect("schemas dir")
        .map(|entry| entry.expect("schema dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("manifest.json"))
        .map(|path| format!("schemas/{}", path.file_name().unwrap().to_string_lossy()))
        .collect::<std::collections::BTreeSet<_>>();
    let manifest_schema_files = schemas
        .iter()
        .map(|entry| entry["file"].as_str().unwrap().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(manifest_schema_files, actual_schema_files);

    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    for entry in schemas {
        let kind = entry["kind"].as_str().unwrap();
        let rel = entry["file"].as_str().unwrap();
        let schema_json: Value =
            serde_json::from_str(&fs::read_to_string(root.join(rel)).expect("schema"))
                .expect("schema json");
        assert_eq!(
            schema_json["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema_json["$id"],
            format!("https://github.com/AmirTlinov/codemap/{rel}")
        );
        let output = codemap()
            .current_dir(outside.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args(["schema", kind])
            .output()
            .expect("schema command should run");
        assert!(output.status.success());
        let printed: Value = serde_json::from_slice(&output.stdout).expect("printed schema json");
        assert_eq!(printed, schema_json);
    }
    assert_eq!(fs::read_dir(cache.path()).expect("cache dir").count(), 0);
}

#[test]
fn removed_graph_lens_aliases_fail_closed() {
    let (repo, cache) = fixture();
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["graph", "--lens", "verify", "--format", "json"])
        .output()
        .expect("codemap should run");
    assert!(
        !output.status.success(),
        "removed verify lens alias must not silently fall back"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown graph lens"));
}

#[test]
fn removed_router_commands_and_flags_fail_closed() {
    let (repo, cache) = fixture();
    let removed_command = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["explain", "packages/replay/src/session.ts"])
        .output()
        .expect("codemap should run");
    assert!(
        !removed_command.status.success(),
        "removed explain command must not be accepted"
    );

    let removed_flag = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "packages/replay/src/session.ts",
            "--structural",
        ])
        .output()
        .expect("codemap should run");
    assert!(
        !removed_flag.status.success(),
        "removed --structural flag must not be accepted"
    );
}
