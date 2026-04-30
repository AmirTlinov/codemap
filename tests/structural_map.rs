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
        "export { publicOnly } from './public-only';\nexport { seek } from './session';\nexport type { FrameDto } from './types';\n",
    );
    write(
        &repo.path().join("packages/replay/src/public-only.ts"),
        "export function publicOnly() {\n  return true;\n}\n",
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
        &repo.path().join("packages/replay/tests/public-api.test.ts"),
        "import { publicOnly } from '../src/index';\n\ntest('public api exposes publicOnly', () => {\n  expect(publicOnly()).toBe(true);\n});\n",
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
            .join("packages/app/src/features/studio/canvas/shell-hint.tsx"),
        "export function ShellHint() {\n  return (\n    <div className=\"blueprint-canvas__hint\" aria-live=\"polite\">\n      Дважды кликни по канвасу или нажми <kbd className=\"kbd\">F</kbd> — появится новый кадр\n    </div>\n  );\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/open-frame-board.tsx"),
        "export function OpenFrameBoard() {\n  return <button className=\"frame-board-action\">Open frame board</button>;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\nexport function ShellView() {\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-aliased-view.tsx"),
        "import { ShellHint as CanvasShellHint } from './shell-hint';\n\nexport function ShellAliasedView() {\n  return <CanvasShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-helper.ts"),
        "import { ShellHint } from './shell-hint';\n\nexport const shellHintComponent = ShellHint;\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-import-only-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\nexport function ShellImportOnlyView() {\n  const Hint = ShellHint;\n  return <div className=\"import-only-view\">{Hint.name}</div>;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-type-only-view.tsx"),
        "import type { ShellHint } from './shell-hint';\n\nexport function ShellTypeOnlyView(_props: { hint?: typeof ShellHint }) {\n  return <div className=\"type-only-view\" />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/other-shell-hint.tsx"),
        "export function ShellHint() {\n  return <div className=\"other-shell-hint\" />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-mismatch-view.tsx"),
        "import { ShellHint as WrongShellHint } from './shell-hint';\nimport { ShellHint } from './other-shell-hint';\n\nexport function ShellMismatchView() {\n  void WrongShellHint;\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-string-shadow-view.tsx"),
        "const docs = \"import { ShellHint } from './shell-hint';\";\n\nfunction ShellHint() {\n  return <div>Local unrelated shell hint</div>;\n}\n\nexport function ShellStringShadowView() {\n  void docs;\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-local-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\nexport function ShellLocalShadowView() {\n  function ShellHint() {\n    return <div>Local unrelated shell hint</div>;\n  }\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-param-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\ntype Props = { ShellHint: () => JSX.Element };\n\nexport function ShellParamShadowView({ ShellHint }: Props) {\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-default-function-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\ntype Props = { ShellHint: () => JSX.Element };\n\nexport default function({ ShellHint }: Props) {\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-method-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\ntype Props = { ShellHint: () => JSX.Element };\n\nexport const ShellMethodShadowView = {\n  render({ ShellHint }: Props) {\n    return <ShellHint />;\n  },\n};\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-destructure-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\ntype Props = { ShellHint: () => JSX.Element };\n\nexport function ShellDestructureShadowView(props: Props) {\n  const { ShellHint } = props;\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-default-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\ntype Props = { ShellHint?: () => JSX.Element };\n\nexport function ShellDefaultShadowView(props: Props) {\n  const { ShellHint = () => <div>Local fallback</div> } = props;\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-multiline-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\ntype Props = { ShellHint: () => JSX.Element };\n\nexport function ShellMultilineShadowView(props: Props) {\n  const {\n    ShellHint,\n  } = props;\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-alias-default-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\ntype Props = { hint?: () => JSX.Element };\n\nexport function ShellAliasDefaultShadowView(props: Props) {\n  const { hint: ShellHint = () => <div>Local fallback</div> } = props;\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shell-array-shadow-view.tsx"),
        "import { ShellHint } from './shell-hint';\n\ntype Props = { hints: Array<() => JSX.Element> };\n\nexport function ShellArrayShadowView(props: Props) {\n  const [ShellHint = () => <div>Local fallback</div>] = props.hints;\n  return <ShellHint />;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/unit-only-hint.tsx"),
        "export function UnitOnlyHint() {\n  return <div className=\"unit-only-hint\">Unit-only hint</div>;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/unit-only-wrapper.tsx"),
        "import { UnitOnlyHint } from './unit-only-hint';\n\nexport function UnitOnlyWrapper() {\n  return <UnitOnlyHint />;\n}\n",
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
            .join("packages/app/tests/e2e/canvas-shell-hint.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('canvas shell empty hint stays visible', async ({ page }) => {\n  await page.goto('/studio');\n  await expect(page.getByText('Дважды кликни по канвасу или нажми')).toBeVisible();\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/reopen-frame-board.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('reopen frame board stays visible', async ({ page }) => {\n  await page.goto('/studio');\n  await expect(page.getByText('Reopen frame board')).toBeVisible();\n});\n",
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
            .join("packages/app/tests/unit-only-hint.test.tsx"),
        "import { UnitOnlyHint } from '../src/features/studio/canvas/unit-only-hint';\n\ntest('dependency unit test imports only the hint', () => {\n  expect(UnitOnlyHint).toBeDefined();\n});\n",
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
    write(
        &repo.path().join("fixtures/example/package.json"),
        r#"{"name":"fixture-package","scripts":{"test":"vitest run"}}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture support package"]);

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
    assert!(
        surfaces.iter().all(|surface| !surface["kind"]
            .as_str()
            .unwrap_or_default()
            .starts_with("support_package:")),
        "root map should not surface fixture/example package internals by default: {json:#}"
    );
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"]
                == "support packages hidden below fixture/example/sample scopes"),
        "root map should expose hidden support package count, not package noise: {json:#}"
    );
    assert_eq!(json.get("read_first"), None);
    assert_eq!(json.get("confidence"), None);

    let root_with_hidden = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--include-hidden", "--format", "json"],
    );
    assert!(
        root_with_hidden["directory"]
            .as_array()
            .expect("root include-hidden directory")
            .iter()
            .any(|surface| surface["kind"]
                .as_str()
                .unwrap_or_default()
                .starts_with("support_package:")),
        "include-hidden should reveal support packages at root on explicit request: {root_with_hidden:#}"
    );

    let fixture_scope = run_json(
        repo.path(),
        cache.path(),
        &["ls", "fixtures", "--format", "json"],
    );
    assert!(
        fixture_scope["directory"]
            .as_array()
            .expect("fixture directory")
            .iter()
            .any(|surface| surface["kind"] == "package:javascript"),
        "explicit fixture scope should still show its local packages: {fixture_scope:#}"
    );

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
fn proof_directory_aggregates_member_file_proofs_without_broad_fallback() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "packages/replay/src", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs.iter().any(
            |surface| surface["path"] == "packages/replay/tests/session.test.ts"
                && surface["evidence"] == "test_import"
        ),
        "directory proof should include direct member-file unit proof: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .any(|surface| surface["path"] == "packages/replay/tests/e2e/seek.e2e.ts"),
        "directory proof should preserve e2e proof for files inside the directory: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "specific directory proofs should suppress broad package fallback: {proof:#}"
    );
    assert_eq!(proof.get("read_first"), None);
}

#[test]
fn proof_root_stays_bounded_without_expanding_test_galaxy() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", ".", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "root proof should not enumerate every repository test: {proof:#}"
    );
    assert!(
        proof["fallback"]
            .as_array()
            .expect("fallback")
            .iter()
            .any(|command| command
                .as_str()
                .is_some_and(|value| value.ends_with(" test"))),
        "root proof should stay at broad command level instead of expanding the map: {proof:#}"
    );
    assert_eq!(proof.get("read_first"), None);
}

#[test]
fn cone_shows_proof_edges_through_direct_consumers() {
    let (repo, cache) = fixture();
    let public_impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/public-only.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &public_impact);
    assert_eq!(public_impact["clusters"][0]["risk"], "high");

    let public_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/public-only.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &public_proof);
    assert_eq!(
        public_proof["risk"], public_impact["clusters"][0]["risk"],
        "proof risk should reflect structural impact when a direct consumer is a contract/public surface: {public_proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/replay/src/public-only.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/index.ts"
                && edge["to"] == "packages/replay/src/public-only.ts"),
        "direct public consumer should be visible before proof via consumer is trusted: {cone:#}"
    );
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(
                |edge| edge["from"] == "packages/replay/tests/public-api.test.ts"
                    && edge["to"] == "packages/replay/src/public-only.ts"
                    && edge["evidence"] == "test_import_via_direct_consumer"
            ),
        "cone should show proof reachable through the direct consumer, not only proof for direct imports: {cone:#}"
    );

    let session_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &session_cone);
    assert!(
        session_cone["proof"]
            .as_array()
            .expect("session proof")
            .iter()
            .all(|edge| edge["from"] != "packages/replay/tests/public-api.test.ts"),
        "a test importing a shared public consumer must still mention this anchor before becoming via-consumer proof: {session_cone:#}"
    );
}

#[test]
fn file_ls_exports_async_symbols_from_symbol_map() {
    let (repo, cache) = fixture();
    let ls = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "packages/app/tests/e2e/support/mixed-layout-page.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "openMixedLayout" && symbol["exported"] == true),
        "symbol map should mark exported async functions: {ls:#}"
    );
    assert!(
        ls["anchor"]["exports"]
            .as_array()
            .expect("exports")
            .iter()
            .any(|export| export == "openMixedLayout"),
        "file export surface should include exported async functions discovered by the symbol map: {ls:#}"
    );
    assert!(
        ls["anchor"]["exports"]
            .as_array()
            .expect("exports")
            .iter()
            .all(|export| export != "page"),
        "file export surface must not promote non-exported parameters or local bindings: {ls:#}"
    );
}

#[test]
fn proof_risk_uses_structural_edges_without_high_inflation() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/plain-value.ts"),
        "export const plainValue = 1;\n",
    );
    write(
        &repo.path().join("packages/replay/src/plain-consumer.ts"),
        "import { plainValue } from './plain-value';\n\nexport const doubled = plainValue * 2;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "plain direct consumer"]);

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/plain-value.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    assert_eq!(
        impact["clusters"][0]["risk"], "medium",
        "a plain direct consumer should raise local risk without pretending to be a contract blast radius: {impact:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/plain-value.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert_eq!(
        proof["risk"], impact["clusters"][0]["risk"],
        "proof should share structural risk semantics with impact without high inflation: {proof:#}"
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
        "e2e specs with strong path/name overlap should prove non-UI domain anchors without broad fallback: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "structural e2e path proof should suppress broad fallback: {proof:#}"
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
        symbol_proof["fallback"]
            .as_array()
            .expect("symbol fallback")
            .is_empty(),
        "owning-file proof should suppress broad fallback for symbol anchors: {symbol_proof:#}"
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
fn symbol_owning_file_proof_does_not_inherit_consumer_tests_for_sibling_symbol() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/sibling-anchor.ts"),
        "export function foo() {\n  return 'foo';\n}\n\nexport function bar() {\n  return 'bar';\n}\n",
    );
    write(
        &repo.path().join("packages/app/src/sibling-consumer.ts"),
        "import { foo } from './sibling-anchor';\n\nexport const value = foo();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/sibling-consumer.test.ts"),
        "import { value } from '../src/sibling-consumer';\n\ntest('uses foo consumer', () => {\n  expect(value).toBe('foo');\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/sibling-anchor.test.ts"),
        "import { foo } from '../src/sibling-anchor';\n\ntest('uses foo from the anchor file', () => {\n  expect(foo()).toBe('foo');\n});\n",
    );
    write(
        &repo.path().join("packages/app/src/cart-panel.ts"),
        "export function openCartPanel() {\n  return 'open';\n}\n\nexport function closeCartPanel() {\n  return 'close';\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/e2e/open-cart-panel.spec.ts"),
        "import { expect, test } from '@playwright/test';\n\ntest('open cart panel flow', async () => {\n  expect('open cart panel').toContain('open');\n});\n",
    );
    write(
        &repo.path().join("packages/app/src/panel-actions.ts"),
        "export function openCartPanel() {\n  return 'open';\n}\n\nexport function closeCartPanel() {\n  return 'close';\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "sibling consumer proof"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/sibling-anchor.ts#bar",
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
            .all(|surface| surface["path"] != "packages/app/tests/sibling-consumer.test.ts"),
        "symbol owning-file fallback must not inherit direct-consumer tests for a sibling export: {proof:#}"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/sibling-anchor.test.ts"),
        "symbol owning-file fallback must not inherit direct file-import tests for a sibling export: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without exact symbol or strict owning-file proof, broad fallback must remain visible: {proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/sibling-anchor.ts#bar",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["proof"]
            .as_array()
            .expect("cone proof")
            .iter()
            .all(|edge| edge["from"] != "packages/app/tests/sibling-anchor.test.ts"),
        "symbol cone must not inherit direct file-import proof for a sibling export: {cone:#}"
    );

    let close_cart_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/cart-panel.ts#closeCartPanel",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &close_cart_proof);
    assert!(
        close_cart_proof["proofs"]
            .as_array()
            .expect("close cart proofs")
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/e2e/open-cart-panel.spec.ts"),
        "owning-file fallback must require a symbol-distinctive term, not shared file/domain terms: {close_cart_proof:#}"
    );

    let open_cart_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/cart-panel.ts#openCartPanel",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &open_cart_proof);
    assert!(
        open_cart_proof["proofs"]
            .as_array()
            .expect("open cart proofs")
            .iter()
            .any(|surface| {
                surface["path"] == "packages/app/tests/e2e/open-cart-panel.spec.ts"
                    && surface["evidence"] == "e2e_path_surface_owning_file"
                    && surface["strength"] == "medium"
            }),
        "owning-file fallback may use e2e path surfaces when they contain a symbol-distinctive term: {open_cart_proof:#}"
    );

    let close_panel_action_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/panel-actions.ts#closeCartPanel",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &close_panel_action_proof);
    assert!(
        close_panel_action_proof["proofs"]
            .as_array()
            .expect("close panel action proofs")
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/e2e/open-cart-panel.spec.ts"),
        "owning-file fallback must require a term unique to this symbol among sibling exports: {close_panel_action_proof:#}"
    );

    let close_panel_action_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/panel-actions.ts#closeCartPanel",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &close_panel_action_cone);
    assert!(
        close_panel_action_cone["proof"]
            .as_array()
            .expect("close panel action cone proof")
            .iter()
            .all(|edge| edge["from"] != "packages/app/tests/e2e/open-cart-panel.spec.ts"),
        "symbol cone must use the same sibling-unique guard as proof: {close_panel_action_cone:#}"
    );

    let open_panel_action_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/panel-actions.ts#openCartPanel",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &open_panel_action_proof);
    assert!(
        open_panel_action_proof["proofs"]
            .as_array()
            .expect("open panel action proofs")
            .iter()
            .any(|surface| {
                surface["path"] == "packages/app/tests/e2e/open-cart-panel.spec.ts"
                    && surface["evidence"] == "e2e_path_surface_owning_file"
                    && surface["strength"] == "medium"
            }),
        "owning-file fallback may use e2e path surfaces when they contain a term unique to this sibling export: {open_panel_action_proof:#}"
    );
}

#[test]
fn proof_links_jsx_visible_text_to_e2e_get_by_text_partial() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/shell-hint.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proof surfaces");
    assert!(
        proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/canvas-shell-hint.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase"
            && surface["command"]
                .as_str()
                .unwrap_or_default()
                .contains("test:e2e")),
        "static JSX visible text should link to partial getByText e2e proof without broad fallback: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "e2e visible-text proof should hide broad fallback: {proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/shell-hint.tsx",
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
                |edge| edge["from"] == "packages/app/tests/e2e/canvas-shell-hint.spec.ts"
                    && edge["evidence"] == "e2e_surface_phrase"
            ),
        "cone should expose the same visible-text proof edge: {cone:#}"
    );
}

#[test]
fn proof_visible_text_partial_match_respects_phrase_boundaries() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/open-frame-board.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proof surfaces")
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/e2e/reopen-frame-board.spec.ts"),
        "`Open frame board` must not match `Reopen frame board` by raw substring: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a structural proof, codemap should keep the broad fallback visible: {proof:#}"
    );
}

#[test]
fn proof_follows_direct_ui_dependency_for_thin_composition_files() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/shell-view.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proof surfaces");
    assert!(
        proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/canvas-shell-hint.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase_via_direct_dependency"
            && surface["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("direct dependency")),
        "thin TSX composition should inherit proof from directly rendered UI dependency: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "direct dependency proof should hide broad fallback: {proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/shell-view.tsx",
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
                |edge| edge["from"] == "packages/app/tests/e2e/canvas-shell-hint.spec.ts"
                    && edge["evidence"] == "e2e_surface_phrase_via_direct_dependency"
            ),
        "cone should expose dependency-derived proof as an edge: {cone:#}"
    );

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/app/src/features/studio/canvas/shell-view.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    assert!(
        impact["clusters"][0]["proof"]
            .as_array()
            .expect("impact proof edges")
            .iter()
            .any(
                |edge| edge["from"] == "packages/app/tests/e2e/canvas-shell-hint.spec.ts"
                    && edge["evidence"] == "e2e_surface_phrase_via_direct_dependency"
            ),
        "impact should reuse dependency-derived structural proof instead of returning an empty proof cluster: {impact:#}"
    );
}

#[test]
fn proof_follows_direct_ui_dependency_when_rendered_component_is_aliased() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/shell-aliased-view.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proof surfaces");
    assert!(
        proofs.iter().any(|surface| surface["path"]
            == "packages/app/tests/e2e/canvas-shell-hint.spec.ts"
            && surface["evidence"] == "e2e_surface_phrase_via_direct_dependency"),
        "aliased rendered import should still inherit proof from the exact dependency export: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "aliased direct dependency proof should hide broad fallback: {proof:#}"
    );
}

#[test]
fn proof_does_not_transfer_dependency_unit_tests_to_thin_composition_files() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/unit-only-wrapper.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proof surfaces")
            .iter()
            .all(|surface| !surface["evidence"]
                .as_str()
                .unwrap_or_default()
                .ends_with("_via_direct_dependency")),
        "dependency unit tests must not become proof for a thin composition wrapper: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without transferable e2e/UI-surface proof, broad fallback must stay visible: {proof:#}"
    );
}

#[test]
fn proof_does_not_treat_string_literal_import_as_rendered_dependency_binding() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/shell-string-shadow-view.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proof surfaces")
            .iter()
            .all(|surface| !surface["evidence"]
                .as_str()
                .unwrap_or_default()
                .ends_with("_via_direct_dependency")),
        "import text inside a string literal must not bind the local JSX tag to a dependency: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without a real import binding, broad fallback must stay visible: {proof:#}"
    );
}

#[test]
fn proof_does_not_transfer_dependency_when_local_symbol_shadows_import_binding() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/shell-local-shadow-view.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proof surfaces")
            .iter()
            .all(|surface| !surface["evidence"]
                .as_str()
                .unwrap_or_default()
                .ends_with("_via_direct_dependency")),
        "a local symbol shadowing the imported JSX binding must fail closed: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without scope-accurate dependency rendering proof, broad fallback must stay visible: {proof:#}"
    );
}

#[test]
fn proof_does_not_transfer_dependency_when_param_shadows_import_binding() {
    let (repo, cache) = fixture();
    for path in [
        "packages/app/src/features/studio/canvas/shell-param-shadow-view.tsx",
        "packages/app/src/features/studio/canvas/shell-default-function-shadow-view.tsx",
        "packages/app/src/features/studio/canvas/shell-method-shadow-view.tsx",
    ] {
        let proof = run_json(
            repo.path(),
            cache.path(),
            &["proof", path, "--format", "json"],
        );
        assert_schema("schemas/proof.schema.json", &proof);
        assert!(
            proof["proofs"]
                .as_array()
                .expect("proof surfaces")
                .iter()
                .all(|surface| !surface["evidence"]
                    .as_str()
                    .unwrap_or_default()
                    .ends_with("_via_direct_dependency")),
            "a parameter/destructured prop shadowing the imported JSX binding must fail closed for {path}: {proof:#}"
        );
        assert!(
            !proof["fallback"].as_array().expect("fallback").is_empty(),
            "without scope-accurate dependency rendering proof, broad fallback must stay visible for {path}: {proof:#}"
        );
    }
}

#[test]
fn proof_does_not_transfer_dependency_when_destructuring_shadows_import_binding() {
    let (repo, cache) = fixture();
    for path in [
        "packages/app/src/features/studio/canvas/shell-destructure-shadow-view.tsx",
        "packages/app/src/features/studio/canvas/shell-default-shadow-view.tsx",
        "packages/app/src/features/studio/canvas/shell-multiline-shadow-view.tsx",
        "packages/app/src/features/studio/canvas/shell-alias-default-shadow-view.tsx",
        "packages/app/src/features/studio/canvas/shell-array-shadow-view.tsx",
    ] {
        let proof = run_json(
            repo.path(),
            cache.path(),
            &["proof", path, "--format", "json"],
        );
        assert_schema("schemas/proof.schema.json", &proof);
        assert!(
            proof["proofs"]
                .as_array()
                .expect("proof surfaces")
                .iter()
                .all(|surface| !surface["evidence"]
                    .as_str()
                    .unwrap_or_default()
                    .ends_with("_via_direct_dependency")),
            "a destructured local binding shadowing the imported JSX binding must fail closed for {path}: {proof:#}"
        );
        assert!(
            !proof["fallback"].as_array().expect("fallback").is_empty(),
            "without scope-accurate dependency rendering proof, broad fallback must stay visible for {path}: {proof:#}"
        );
    }
}

#[test]
fn proof_does_not_follow_direct_ui_dependency_from_non_ui_helpers() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/shell-helper.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proof surfaces")
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/e2e/canvas-shell-hint.spec.ts"),
        "non-UI helpers should not inherit e2e proof merely by importing a component: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "without UI composition proof, broad fallback should remain visible: {proof:#}"
    );
}

#[test]
fn proof_does_not_follow_direct_ui_dependency_without_jsx_render() {
    let (repo, cache) = fixture();
    for target in [
        "packages/app/src/features/studio/canvas/shell-import-only-view.tsx",
        "packages/app/src/features/studio/canvas/shell-type-only-view.tsx",
    ] {
        let proof = run_json(
            repo.path(),
            cache.path(),
            &["proof", target, "--format", "json"],
        );
        assert_schema("schemas/proof.schema.json", &proof);
        assert!(
            proof["proofs"]
                .as_array()
                .expect("proof surfaces")
                .iter()
                .all(
                    |surface| surface["path"] != "packages/app/tests/e2e/canvas-shell-hint.spec.ts"
                ),
            "TSX anchors should not inherit e2e proof unless they render the dependency as JSX: {target}\n{proof:#}"
        );
        assert!(
            !proof["fallback"].as_array().expect("fallback").is_empty(),
            "fallback should stay visible without rendered dependency proof: {target}\n{proof:#}"
        );
    }
}

#[test]
fn proof_direct_ui_dependency_requires_jsx_binding_from_same_dependency() {
    let (repo, cache) = fixture();
    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/shell-mismatch-view.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proof surfaces")
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/e2e/canvas-shell-hint.spec.ts"),
        "rendering `ShellHint` from another dependency must not inherit proof from the aliased dependency that merely exports the same name: {proof:#}"
    );
    assert!(
        !proof["fallback"].as_array().expect("fallback").is_empty(),
        "fallback should remain visible when no rendered dependency has structural proof: {proof:#}"
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
fn proof_and_impact_link_same_package_symbol_references_without_imports() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("go.mod"),
        "module example.com/replay\n\ngo 1.22\n",
    );
    write(
        &repo.path().join("session/session.go"),
        "package session\n\nfunc FrameLabel(frame int) string {\n\treturn \"frame\"\n}\n",
    );
    write(
        &repo.path().join("session/consumer.go"),
        "package session\n\nfunc RenderLabel() string {\n\treturn FrameLabel(1)\n}\n",
    );
    write(
        &repo.path().join("session/raw_fixture.go"),
        "package session\n\nconst fixture = `\nFrameLabel\n`\n",
    );
    write(
        &repo.path().join("session/foreign_consumer.go"),
        "package session\n\nimport other \"example.com/replay/other\"\n\nfunc RenderForeignLabel() string {\n\treturn other.FrameLabel(1)\n}\n",
    );
    write(
        &repo.path().join("other/label.go"),
        "package other\n\nfunc FrameLabel(frame int) string {\n\treturn \"foreign\"\n}\n",
    );
    write(
        &repo.path().join("session/method_session.go"),
        "package session\n\ntype Session struct{}\n\nfunc (s Session) Reset() {}\n",
    );
    write(
        &repo.path().join("session/cache.go"),
        "package session\n\ntype Cache struct{}\n\nfunc (c Cache) Reset() {}\n",
    );
    write(
        &repo.path().join("session/surface_test.go"),
        "package session\n\nimport \"testing\"\n\nfunc TestSurfaceUsesFrameLabel(t *testing.T) {\n\tif FrameLabel(2) == \"\" {\n\t\tt.Fatal(\"missing label\")\n\t}\n}\n",
    );
    write(
        &repo.path().join("session/raw_string_test.go"),
        "package session\n\nimport \"testing\"\n\nfunc TestRawStringOnly(t *testing.T) {\n\tfixture := `\nFrameLabel\n`\n\tif fixture == \"\" {\n\t\tt.Fatal(\"missing fixture\")\n\t}\n}\n",
    );
    write(
        &repo.path().join("session/foreign_test.go"),
        "package session\n\nimport (\n\t\"testing\"\n\tother \"example.com/replay/other\"\n)\n\nfunc TestForeignSelector(t *testing.T) {\n\tif other.FrameLabel(3) == \"\" {\n\t\tt.Fatal(\"missing foreign label\")\n\t}\n}\n",
    );
    write(
        &repo.path().join("session/cache_test.go"),
        "package session\n\nimport \"testing\"\n\nfunc TestCacheReset(t *testing.T) {\n\tvar cache Cache\n\tcache.Reset()\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "session/session.go", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(|surface| surface["path"] == "session/surface_test.go"
                && surface["evidence"] == "test_symbol_reference"
                && surface["strength"] == "high"),
        "same-package test symbol references should become structural proof, not fallback: {proof:#}"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "session/raw_string_test.go"),
        "symbols inside multiline raw strings must not become proof: {proof:#}"
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "session/foreign_test.go"),
        "selector tails from imported packages must not become local symbol proof: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "symbol reference proof should suppress broad fallback: {proof:#}"
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "session/session.go", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "session/consumer.go"
                && edge["to"] == "session/session.go"
                && edge["evidence"] == "same_package_symbol_reference"),
        "same-package source references should appear as incoming xref edges: {cone:#}"
    );
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"] != "session/raw_fixture.go"),
        "symbols inside multiline raw strings must not become incoming xref edges: {cone:#}"
    );
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"] != "session/foreign_consumer.go"),
        "selector tails from imported packages must not become local incoming xref edges: {cone:#}"
    );

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "session/session.go",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    let clusters = impact["clusters"].as_array().expect("clusters");
    assert!(
        clusters.iter().any(|cluster| cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .any(|edge| edge["from"] == "session/consumer.go"
                && edge["evidence"] == "same_package_symbol_reference")),
        "impact should carry same-package symbol xref consumers: {impact:#}"
    );
    assert!(
        clusters.iter().all(|cluster| cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .all(|edge| edge["from"] != "session/raw_fixture.go")),
        "raw-string-only files must not inflate impact consumers: {impact:#}"
    );
    assert!(
        clusters.iter().all(|cluster| cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .all(|edge| edge["from"] != "session/foreign_consumer.go")),
        "selector-tail references must not inflate local impact consumers: {impact:#}"
    );

    let method_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "session/method_session.go", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &method_proof);
    assert!(
        method_proof["proofs"]
            .as_array()
            .expect("method proofs")
            .iter()
            .all(|surface| surface["path"] != "session/cache_test.go"),
        "same-name methods on different receivers need type-aware xref and must not become proof: {method_proof:#}"
    );

    let method_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "session/method_session.go", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &method_cone);
    assert!(
        method_cone["incoming"]
            .as_array()
            .expect("method incoming")
            .iter()
            .all(|edge| edge["from"] != "session/cache.go"),
        "same-name method declarations on different receivers must not become incoming xref edges: {method_cone:#}"
    );

    let method_impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "session/method_session.go",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &method_impact);
    assert!(
        method_impact["clusters"]
            .as_array()
            .expect("method clusters")
            .iter()
            .all(|cluster| cluster["direct_consumers"]
                .as_array()
                .expect("method direct consumers")
                .iter()
                .all(|edge| edge["from"] != "session/cache.go")),
        "same-name methods on unrelated receivers must not inflate impact: {method_impact:#}"
    );
}

#[test]
fn symbol_anchor_cone_filters_javascript_import_bindings() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "symbol-anchor-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/card.tsx"),
        "export function GroupCard() {\n  return <section>Group</section>;\n}\n\nexport function AdminCard() {\n  return <section>Admin</section>;\n}\n",
    );
    write(
        &repo.path().join("src/home.tsx"),
        "import { GroupCard as Card } from './card';\n\nexport function HomePage() {\n  return <Card />;\n}\n",
    );
    write(
        &repo.path().join("src/two-cards.tsx"),
        "import { AdminCard, GroupCard } from './card';\n\nexport function TwoCards() {\n  return <><GroupCard /><AdminCard /></>;\n}\n",
    );
    write(
        &repo.path().join("src/panel-parts.tsx"),
        "export function PanelHeader() {\n  return <header>Panel</header>;\n}\n\nexport function PanelBody() {\n  return <main>Body</main>;\n}\n",
    );
    write(
        &repo.path().join("src/panel-view.tsx"),
        "import { PanelBody, PanelHeader } from './panel-parts';\n\ntype Props = {\n  title: string;\n};\n\nexport function PanelView({\n  title,\n}: Props) {\n  return (\n    <section aria-label={title}>\n      <PanelHeader />\n      <PanelBody />\n    </section>\n  );\n}\n",
    );
    write(
        &repo.path().join("src/helpers.tsx"),
        "export function custom() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("src/lowercase-jsx.tsx"),
        "import { custom } from './helpers';\n\nexport function LowercaseView() {\n  return <custom />;\n}\n",
    );
    write(
        &repo.path().join("src/admin.tsx"),
        "import { AdminCard } from './card';\n\nexport function AdminPage() {\n  return <AdminCard />;\n}\n",
    );
    write(
        &repo.path().join("src/unused.tsx"),
        "import { GroupCard } from './card';\n\nexport const unused = true;\n",
    );
    write(
        &repo.path().join("src/side-effect.tsx"),
        "import { GroupCard } from './card'\nimport './setup'\n\nexport function SideEffectPage() {\n  return <GroupCard />;\n}\n",
    );
    write(
        &repo.path().join("src/setup.ts"),
        "export const setup = true;\n",
    );
    write(
        &repo.path().join("src/string-only.tsx"),
        "import { GroupCard } from './card';\n\nexport const fixture = 'GroupCard';\n",
    );
    write(
        &repo.path().join("src/card.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('group card export stays usable', () => {\n  expect(GroupCard).toBeDefined();\n});\n",
    );
    write(
        &repo.path().join("src/type-only-consumer.test.tsx"),
        "import { GroupCard } from './card';\n\ntype Props = {\n  id: string;\n  component: typeof GroupCard;\n};\n\ntest('type-only mention does not prove runtime behavior', () => {\n  const props: Props | null = null;\n  expect(props).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/type-annotation-consumer.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('typeof annotation does not prove runtime behavior', () => {\n  let component: typeof GroupCard | null = null;\n  expect(component).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/type-assertion-consumer.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('typeof assertion does not prove runtime behavior', () => {\n  const value = null as unknown as typeof GroupCard;\n  expect(value).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/implements-only.test.tsx"),
        "import { GroupCard } from './card';\n\nclass Fake implements GroupCard {\n  value = 1;\n}\n\ntest('implements mention does not prove runtime behavior', () => {\n  expect(new Fake().value).toBe(1);\n});\n",
    );
    write(
        &repo.path().join("src/object-key.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('object key does not prove runtime behavior', () => {\n  const metadata = { GroupCard: true };\n  expect(metadata.GroupCard).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/regex-only.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('regex mention does not prove runtime behavior', () => {\n  expect(/GroupCard/.test('GroupCard')).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/regex-angle.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('regex markup mention does not prove runtime behavior', () => {\n  expect(/<GroupCard>/.test('<GroupCard>')).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/regex-group.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('regex group mention does not prove runtime behavior', () => {\n  expect(/foo (GroupCard) bar/.test('GroupCard')).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/arrow-regex-group.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('arrow regex group mention does not prove runtime behavior', () => {\n  const matcher = () => /foo (GroupCard) bar/.test('GroupCard');\n  expect(matcher()).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/await-regex-consumer.ts"),
        "import { GroupCard } from './card';\n\nexport async function regexConsumer(value: string) {\n  return await /foo (GroupCard) bar/.test(value);\n}\n",
    );
    write(
        &repo.path().join("src/if-regex-consumer.tsx"),
        "import { GroupCard } from './card';\n\nexport function regexConsumer(enabled: boolean, value: string) {\n  if (enabled) /foo (GroupCard) bar/.test(value);\n  if (enabled) /<GroupCard>/.test(value);\n}\n",
    );
    write(
        &repo.path().join("src/else-regex-consumer.tsx"),
        "import { GroupCard } from './card';\n\nexport function regexConsumer(enabled: boolean, value: string) {\n  if (enabled) return;\n  else /foo (GroupCard) bar/.test(value);\n}\n",
    );
    write(
        &repo.path().join("src/type-generic-consumer.tsx"),
        "import { GroupCard } from './card';\n\nfunction identity<T>(value: T) {\n  return value;\n}\n\nexport const value = identity<GroupCard | null>(null);\n",
    );
    write(
        &repo.path().join("src/template-consumer.tsx"),
        "import { GroupCard } from './card';\n\nexport const snippet = `\n  GroupCard()\n`;\n",
    );
    write(
        &repo.path().join("src/generic-arrow.tsx"),
        "import { GroupCard } from './card';\n\nexport const make = <GroupCard extends object>() => null;\n",
    );
    write(
        &repo.path().join("src/angle-assertion.ts"),
        "import { GroupCard } from './card';\n\nexport function cast(value: unknown) {\n  return <GroupCard>value;\n}\n",
    );
    write(
        &repo.path().join("src/await-regex.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('await regex mention does not prove runtime behavior', async () => {\n  const matched = await /foo (GroupCard) bar/.test('GroupCard');\n  expect(matched).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("src/if-regex.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('if regex mention does not prove runtime behavior', () => {\n  if (true) /foo (GroupCard) bar/.test('GroupCard');\n  if (true) /<GroupCard>/.test('GroupCard');\n});\n",
    );
    write(
        &repo.path().join("src/else-regex.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('else regex mention does not prove runtime behavior', () => {\n  if (false) return;\n  else /foo (GroupCard) bar/.test('GroupCard');\n});\n",
    );
    write(
        &repo.path().join("src/throw-regex.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('throw regex mention does not prove runtime behavior', () => {\n  try {\n    throw /foo (GroupCard) bar/;\n  } catch (pattern) {\n    expect(pattern.test('GroupCard')).toBe(true);\n  }\n});\n",
    );
    write(
        &repo.path().join("src/type-generic.test.tsx"),
        "import { GroupCard } from './card';\n\nfunction identity<T>(value: T) {\n  return value;\n}\n\ntest('generic type argument does not prove runtime behavior', () => {\n  expect(identity<GroupCard | null>(null)).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/type-factory.test.tsx"),
        "import { GroupCard } from './card';\n\ntype Factory = <GroupCard>() => void;\n\ntest('generic type parameter does not prove runtime behavior', () => {\n  const noop: Factory | null = null;\n  expect(noop).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/generic-arrow.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('generic arrow type parameter does not prove runtime behavior', () => {\n  const make = <GroupCard extends object>() => null;\n  expect(make()).toBeNull();\n});\n",
    );
    write(
        &repo.path().join("src/template-only.test.tsx"),
        "import { GroupCard } from './card';\n\ntest('template snippet does not prove runtime behavior', () => {\n  const snippet = `\n    GroupCard()\n  `;\n  expect(snippet).toContain('GroupCard');\n});\n",
    );
    write(
        &repo.path().join("src/admin.test.tsx"),
        "import { AdminCard } from './card';\n\ntest('admin card export stays usable', () => {\n  expect(AdminCard).toBeDefined();\n});\n",
    );
    write(
        &repo.path().join("src/local-shadow.tsx"),
        "import { GroupCard } from './card';\n\nexport function ShadowPage() {\n  const GroupCard = () => <section>Local</section>;\n  return <GroupCard />;\n}\n",
    );
    write(
        &repo.path().join("src/for-shadow.tsx"),
        "import { GroupCard } from './card';\n\nconst cards = [() => <section>Local</section>];\n\nexport function ForShadowPage() {\n  for (const GroupCard of cards) {\n    return <GroupCard />;\n  }\n  return null;\n}\n",
    );
    write(
        &repo.path().join("src/for-await-shadow.tsx"),
        "import { GroupCard } from './card';\n\nasync function* cards() {\n  yield () => <section>Local</section>;\n}\n\nexport async function ForAwaitShadowPage() {\n  for await (const GroupCard of cards()) {\n    return <GroupCard />;\n  }\n  return null;\n}\n",
    );
    write(
        &repo.path().join("src/catch-shadow.tsx"),
        "import { GroupCard } from './card';\n\nexport function CatchShadowPage() {\n  try {\n    throw new Error('x');\n  } catch (GroupCard) {\n    return <GroupCard />;\n  }\n}\n",
    );
    write(
        &repo.path().join("src/for-await-shadow.test.tsx"),
        "import { GroupCard } from './card';\n\nasync function* cards() {\n  yield () => <section>Local</section>;\n}\n\ntest('for await shadow does not prove runtime behavior', async () => {\n  for await (const GroupCard of cards()) {\n    expect(GroupCard).toBeDefined();\n  }\n});\n",
    );
    write(
        &repo.path().join("src/default-card.tsx"),
        "export default function DefaultCard() {\n  return <section>Default</section>;\n}\n",
    );
    write(
        &repo.path().join("src/default-consumer.tsx"),
        "import RenamedCard from './default-card';\n\nexport function DefaultPage() {\n  return <RenamedCard />;\n}\n",
    );
    write(
        &repo.path().join("src/default-card.test.tsx"),
        "import RenamedCard from './default-card';\n\ntest('default card export stays usable', () => {\n  expect(RenamedCard).toBeDefined();\n});\n",
    );
    write(
        &repo.path().join("src/default-const-card.tsx"),
        "const DefaultConstCard = () => <section>Default const</section>;\n\nexport default DefaultConstCard;\n",
    );
    write(
        &repo.path().join("src/default-const-consumer.tsx"),
        "import RenamedConstCard from './default-const-card';\n\nexport function DefaultConstPage() {\n  return <RenamedConstCard />;\n}\n",
    );
    write(
        &repo.path().join("src/default-const-card.test.tsx"),
        "import RenamedConstCard from './default-const-card';\n\ntest('default const card export stays usable', () => {\n  expect(RenamedConstCard).toBeDefined();\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/card.tsx#GroupCard", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["anchor"]["path"], "src/card.tsx#GroupCard");
    assert_eq!(ls["anchor"]["kind"], "symbol:component");
    assert_eq!(
        ls["anchor"]["symbols"].as_array().expect("symbols").len(),
        1
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/card.tsx#GroupCard", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let incoming = cone["incoming"].as_array().expect("incoming");
    assert!(
        incoming.iter().any(|edge| {
            edge["from"] == "src/home.tsx"
                && edge["to"] == "src/card.tsx#GroupCard"
                && edge["evidence"] == "imported_symbol_reference"
        }),
        "symbol cone should show the aliased component consumer: {cone:#}"
    );
    assert!(
        incoming
            .iter()
            .any(|edge| edge["from"] == "src/side-effect.tsx"
                && edge["to"] == "src/card.tsx#GroupCard"
                && edge["evidence"] == "imported_symbol_reference"),
        "semicolonless side-effect imports must not hide later symbol references: {cone:#}"
    );
    assert!(
        incoming.iter().all(|edge| edge["from"] != "src/admin.tsx"
            && edge["from"] != "src/unused.tsx"
            && edge["from"] != "src/string-only.tsx"
            && edge["from"] != "src/await-regex-consumer.ts"
            && edge["from"] != "src/if-regex-consumer.tsx"
            && edge["from"] != "src/else-regex-consumer.tsx"
            && edge["from"] != "src/type-generic-consumer.tsx"
            && edge["from"] != "src/template-consumer.tsx"
            && edge["from"] != "src/generic-arrow.tsx"
            && edge["from"] != "src/local-shadow.tsx"
            && edge["from"] != "src/for-shadow.tsx"
            && edge["from"] != "src/for-await-shadow.tsx"
            && edge["from"] != "src/catch-shadow.tsx"),
        "symbol cone must not include other exports, unused imports, string-only mentions, or local/loop/catch shadows: {cone:#}"
    );
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "src/card.test.tsx"
                && edge["to"] == "src/card.tsx#GroupCard"
                && edge["evidence"] == "test_imported_symbol_reference"),
        "symbol cone should expose exact symbol proof: {cone:#}"
    );

    let home_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/home.tsx#HomePage", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &home_cone);
    assert!(
        home_cone["outgoing"]
            .as_array()
            .expect("home outgoing")
            .iter()
            .any(|edge| edge["from"] == "src/home.tsx#HomePage"
                && edge["to"] == "src/card.tsx#GroupCard"
                && edge["type"] == "symbol_uses"
                && edge["evidence"] == "imported_symbol_in_symbol_body"),
        "symbol cone should show imported symbols used inside the selected symbol body: {home_cone:#}"
    );
    assert!(
        home_cone["unknowns"]
            .as_array()
            .expect("home unknowns")
            .is_empty(),
        "symbol cone should not claim outgoing unknown when it found structural symbol uses: {home_cone:#}"
    );

    let panel_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/panel-view.tsx#PanelView", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &panel_cone);
    let panel_outgoing = panel_cone["outgoing"].as_array().expect("panel outgoing");
    assert!(
        panel_outgoing
            .iter()
            .any(|edge| edge["to"] == "src/panel-parts.tsx#PanelHeader"
                && edge["type"] == "symbol_uses"
                && edge["evidence"] == "imported_symbol_in_symbol_body"),
        "symbol cone should include imported JSX symbols after multiline destructured params: {panel_cone:#}"
    );
    assert!(
        panel_outgoing
            .iter()
            .any(|edge| edge["to"] == "src/panel-parts.tsx#PanelBody"
                && edge["type"] == "symbol_uses"
                && edge["evidence"] == "imported_symbol_in_symbol_body"),
        "symbol cone should not stop the symbol body at the destructured parameter close: {panel_cone:#}"
    );
    assert!(
        panel_cone["unknowns"]
            .as_array()
            .expect("panel unknowns")
            .is_empty(),
        "symbol cone should not claim unknown outgoing once multiline-param symbol uses are found: {panel_cone:#}"
    );

    for false_anchor in [
        "src/unused.tsx#unused",
        "src/if-regex-consumer.tsx#regexConsumer",
        "src/generic-arrow.tsx#make",
        "src/angle-assertion.ts#cast",
        "src/local-shadow.tsx#ShadowPage",
    ] {
        let false_cone = run_json(
            repo.path(),
            cache.path(),
            &["cone", false_anchor, "--format", "json"],
        );
        assert_schema("schemas/cone.schema.json", &false_cone);
        assert!(
            false_cone["outgoing"]
                .as_array()
                .expect("false outgoing")
                .iter()
                .all(|edge| edge["to"] != "src/card.tsx#GroupCard"),
            "symbol outgoing must not link unused imports, regex-only mentions, or local shadows for {false_anchor}: {false_cone:#}"
        );
    }
    let lowercase_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/lowercase-jsx.tsx#LowercaseView",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &lowercase_cone);
    assert!(
        lowercase_cone["outgoing"]
            .as_array()
            .expect("lowercase outgoing")
            .iter()
            .all(|edge| edge["to"] != "src/helpers.tsx#custom"),
        "lowercase JSX tags must not become imported symbol edges: {lowercase_cone:#}"
    );

    let limited_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/two-cards.tsx#TwoCards",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &limited_cone);
    assert_eq!(
        limited_cone["outgoing"]
            .as_array()
            .expect("limited outgoing")
            .len(),
        1,
        "symbol outgoing should honor cone limit: {limited_cone:#}"
    );
    assert!(
        limited_cone["hidden"]
            .as_array()
            .expect("limited hidden")
            .iter()
            .any(
                |group| group["reason"] == "symbol outgoing edges hidden by limit"
                    && group["count"] == 1
            ),
        "symbol outgoing should report hidden edges when truncated: {limited_cone:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "src/card.tsx#GroupCard", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    let proofs = proof["proofs"].as_array().expect("proofs");
    assert!(
        proofs
            .iter()
            .any(|surface| surface["path"] == "src/card.test.tsx"
                && surface["evidence"] == "test_imported_symbol_reference"
                && surface["command"] == "npx vitest run src/card.test.tsx"),
        "symbol proof should prefer the exact importing test file: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "src/admin.test.tsx"),
        "symbol proof must not inherit tests for sibling exports: {proof:#}"
    );
    assert!(
        proofs
            .iter()
            .all(|surface| surface["path"] != "src/type-only-consumer.test.tsx"),
        "type-only symbol mentions must not become runtime proof: {proof:#}"
    );
    assert!(
        proofs.iter().all(
            |surface| surface["path"] != "src/type-annotation-consumer.test.tsx"
                && surface["path"] != "src/type-assertion-consumer.test.tsx"
                && surface["path"] != "src/implements-only.test.tsx"
                && surface["path"] != "src/object-key.test.tsx"
                && surface["path"] != "src/regex-only.test.tsx"
                && surface["path"] != "src/regex-angle.test.tsx"
                && surface["path"] != "src/regex-group.test.tsx"
                && surface["path"] != "src/arrow-regex-group.test.tsx"
                && surface["path"] != "src/await-regex.test.tsx"
                && surface["path"] != "src/if-regex.test.tsx"
                && surface["path"] != "src/else-regex.test.tsx"
                && surface["path"] != "src/throw-regex.test.tsx"
                && surface["path"] != "src/type-generic.test.tsx"
                && surface["path"] != "src/type-factory.test.tsx"
                && surface["path"] != "src/generic-arrow.test.tsx"
                && surface["path"] != "src/template-only.test.tsx"
                && surface["path"] != "src/for-await-shadow.test.tsx"
        ),
        "type-only/object-key/regex mentions must not become runtime proof: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "exact symbol proof should suppress broad fallback: {proof:#}"
    );

    let default_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/default-card.tsx#DefaultCard",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &default_cone);
    assert!(
        default_cone["incoming"]
            .as_array()
            .expect("default incoming")
            .iter()
            .any(|edge| edge["from"] == "src/default-consumer.tsx"
                && edge["to"] == "src/default-card.tsx#DefaultCard"
                && edge["evidence"] == "imported_symbol_reference"),
        "default import aliases should link to the named default symbol anchor: {default_cone:#}"
    );

    let default_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/default-card.tsx#DefaultCard",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &default_proof);
    assert!(
        default_proof["proofs"]
            .as_array()
            .expect("default proofs")
            .iter()
            .any(|surface| surface["path"] == "src/default-card.test.tsx"
                && surface["evidence"] == "test_imported_symbol_reference"),
        "default import aliases should become exact symbol proof: {default_proof:#}"
    );

    let default_const_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/default-const-card.tsx#DefaultConstCard",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &default_const_cone);
    assert!(
        default_const_cone["incoming"]
            .as_array()
            .expect("default const incoming")
            .iter()
            .any(|edge| edge["from"] == "src/default-const-consumer.tsx"
                && edge["to"] == "src/default-const-card.tsx#DefaultConstCard"
                && edge["evidence"] == "imported_symbol_reference"),
        "default identifier aliases should link to the local default-exported symbol anchor: {default_const_cone:#}"
    );

    let default_const_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/default-const-card.tsx#DefaultConstCard",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &default_const_proof);
    assert!(
        default_const_proof["proofs"]
            .as_array()
            .expect("default const proofs")
            .iter()
            .any(
                |surface| surface["path"] == "src/default-const-card.test.tsx"
                    && surface["evidence"] == "test_imported_symbol_reference"
            ),
        "default identifier aliases should become exact symbol proof: {default_const_proof:#}"
    );
}

#[test]
fn symbol_anchor_cone_links_same_file_symbol_body_uses() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "local-symbol-flow-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/local-flow.tsx"),
        "function formatFocus(id: string) {\n  return id.trim();\n}\n\nfunction combineFocus(prefix: string, formatter: (id: string) => string) {\n  return formatter(prefix);\n}\n\nfunction chooseFocus(ids: string[]) {\n  return formatFocus(ids[0] ?? '');\n}\n\nexport function SelectionPanel({ ids }: { ids: string[] }) {\n  const focus = chooseFocus(ids);\n  return <section>{focus}</section>;\n}\n\nexport function ArgumentUse() {\n  return combineFocus('x', formatFocus);\n}\n\nexport function ParameterShadow(formatFocus: (id: string) => string) {\n  return formatFocus('local');\n}\n\nexport function MultiLineParameterShadow(\n  formatFocus: (id: string) => string\n) {\n  return formatFocus('local');\n}\n\nexport function LaterMultiLineParameterShadow(\n  makeFocus: (id: string) => string,\n  formatFocus: (id: string) => string\n) {\n  return formatFocus('local');\n}\n\nexport const MultiLineArrowShadow = (\n  formatFocus: (id: string) => string\n) => formatFocus('local');\n\nexport const LaterMultiLineArrowShadow = (\n  makeFocus: (id: string) => string,\n  formatFocus: (id: string) => string\n) => formatFocus('local');\n\nexport function MultiLineDestructureShadow(props: { formatFocus: (id: string) => string }) {\n  const {\n    formatFocus,\n  } = props;\n  return formatFocus('local');\n}\n\nexport function LocalConstShadow() {\n  const formatFocus = (id: string) => id.toUpperCase();\n  return formatFocus('local');\n}\n",
    );
    write(
        &repo.path().join("src/local-flow.test.tsx"),
        "import { SelectionPanel } from './local-flow';\n\ntest('selection panel export stays usable', () => {\n  expect(SelectionPanel).toBeDefined();\n});\n",
    );
    write(
        &repo.path().join("tests/support/local-flow-page.tsx"),
        "import { SelectionPanel } from '../../src/local-flow';\n\nexport function renderSelectionPanel() {\n  return SelectionPanel;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "local symbol flow fixture"]);

    let panel_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/local-flow.tsx#SelectionPanel",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &panel_cone);
    assert!(
        panel_cone["outgoing"]
            .as_array()
            .expect("panel outgoing")
            .iter()
            .any(|edge| edge["from"] == "src/local-flow.tsx#SelectionPanel"
                && edge["to"] == "src/local-flow.tsx#chooseFocus"
                && edge["type"] == "symbol_uses"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "symbol cone should show same-file symbol uses without requiring import edges: {panel_cone:#}"
    );
    assert!(
        panel_cone["unknowns"]
            .as_array()
            .expect("panel unknowns")
            .is_empty(),
        "same-file symbol uses should make the symbol cone structurally grounded: {panel_cone:#}"
    );

    let choose_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/local-flow.tsx#chooseFocus", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &choose_cone);
    assert!(
        choose_cone["outgoing"]
            .as_array()
            .expect("choose outgoing")
            .iter()
            .any(|edge| edge["from"] == "src/local-flow.tsx#chooseFocus"
                && edge["to"] == "src/local-flow.tsx#formatFocus"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "same-file helper chains should be visible at symbol level: {choose_cone:#}"
    );
    assert!(
        choose_cone["incoming"]
            .as_array()
            .expect("choose incoming")
            .iter()
            .any(|edge| edge["from"] == "src/local-flow.tsx#SelectionPanel"
                && edge["to"] == "src/local-flow.tsx#chooseFocus"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "symbol cone should show same-file symbols that consume the selected helper: {choose_cone:#}"
    );

    let choose_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/local-flow.tsx#chooseFocus",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &choose_proof);
    assert!(
        choose_proof["proofs"]
            .as_array()
            .expect("choose proofs")
            .iter()
            .any(|surface| surface["path"] == "src/local-flow.test.tsx"
                && surface["evidence"]
                    == "test_imported_symbol_reference_via_local_symbol_consumer"
                && surface["strength"] == "medium"),
        "proof should traverse exact tests of same-file symbol consumers before broad fallback: {choose_proof:#}"
    );
    assert!(
        choose_proof["proofs"]
            .as_array()
            .expect("choose proofs")
            .iter()
            .all(|surface| surface["path"] != "tests/support/local-flow-page.tsx"),
        "test support files may explain chains but must not become runnable proof surfaces: {choose_proof:#}"
    );
    assert!(
        choose_proof["fallback"]
            .as_array()
            .expect("choose fallback")
            .is_empty(),
        "same-file consumer proof should suppress broad fallback: {choose_proof:#}"
    );

    let argument_cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/local-flow.tsx#ArgumentUse", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &argument_cone);
    let argument_outgoing = argument_cone["outgoing"]
        .as_array()
        .expect("argument outgoing");
    assert!(
        argument_outgoing
            .iter()
            .any(|edge| edge["to"] == "src/local-flow.tsx#combineFocus"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "same-file function calls should be visible from selected symbol bodies: {argument_cone:#}"
    );
    assert!(
        argument_outgoing
            .iter()
            .any(|edge| edge["to"] == "src/local-flow.tsx#formatFocus"
                && edge["evidence"] == "local_symbol_in_symbol_body"),
        "same-file symbol references passed as later call arguments must not be mistaken for local bindings: {argument_cone:#}"
    );

    for false_anchor in [
        "src/local-flow.tsx#ParameterShadow",
        "src/local-flow.tsx#MultiLineParameterShadow",
        "src/local-flow.tsx#LaterMultiLineParameterShadow",
        "src/local-flow.tsx#MultiLineArrowShadow",
        "src/local-flow.tsx#LaterMultiLineArrowShadow",
        "src/local-flow.tsx#MultiLineDestructureShadow",
        "src/local-flow.tsx#LocalConstShadow",
    ] {
        let false_cone = run_json(
            repo.path(),
            cache.path(),
            &["cone", false_anchor, "--format", "json"],
        );
        assert_schema("schemas/cone.schema.json", &false_cone);
        assert!(
            false_cone["outgoing"]
                .as_array()
                .expect("false outgoing")
                .iter()
                .all(|edge| edge["to"] != "src/local-flow.tsx#formatFocus"),
            "local params or const bindings must not become same-file symbol edges for {false_anchor}: {false_cone:#}"
        );
    }
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
        .executableTarget(name: "HostApp", dependencies: ["Core"]),
        .testTarget(name: "HostAppTests", dependencies: ["HostApp"])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/HostApp/main.swift"),
        r#"import Foundation
import Core

@MainActor
public final class HostViewModel {
    @Published public var title: String = "Host"

    public func refresh() {}
}
"#,
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
    write(
        &repo
            .path()
            .join("Tests/HostAppTests/HostViewModelTests.swift"),
        r#"@testable import HostApp
import Testing

@Test
func hostViewModelRefreshes() {
    let model = HostViewModel()
    model.refresh()
}
"#,
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
                && edge["to"] == "Packages/Core/"
                && edge["type"] == "package_internal"
                && edge["evidence"] == "package_manifest:Core"),
        "SwiftPM local path dependencies should become package graph edges: {ls:#}"
    );

    let file = run_json(
        repo.path(),
        cache.path(),
        &["ls", "Sources/HostApp/main.swift", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &file);
    let anchor = &file["anchor"];
    assert!(
        anchor["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "HostViewModel"
                && symbol["kind"] == "class"
                && symbol["line_start"] == 5
                && symbol["line_end"] == 9),
        "Swift file ls should surface class symbols with ranges: {file:#}"
    );
    assert!(
        anchor["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "title"
                && symbol["kind"] == "property"
                && symbol["exported"] == true),
        "Swift file ls should surface attributed properties: {file:#}"
    );
    assert!(
        anchor["imports"]
            .as_array()
            .expect("imports")
            .iter()
            .any(|import| import == "Foundation")
            && anchor["imports"]
                .as_array()
                .expect("imports")
                .iter()
                .any(|import| import == "Core"),
        "Swift file ls should surface imported modules: {file:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Sources/HostApp/main.swift", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(
                |surface| surface["path"] == "Tests/HostAppTests/HostViewModelTests.swift"
                    && surface["evidence"] == "test_symbol_reference"
                    && surface["strength"] == "high"
            ),
        "Swift tests that import the module and reference exported symbols should become structural proof, not fallback: {proof:#}"
    );
    assert!(
        proof["fallback"].as_array().expect("fallback").is_empty(),
        "Swift symbol reference proof should suppress broad fallback: {proof:#}"
    );
}

#[test]
fn swift_symbol_reference_proof_requires_imported_target_module() {
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
    name: "MultiTarget",
    targets: [
        .target(name: "Foo"),
        .target(name: "Bar"),
        .testTarget(name: "BarTests", dependencies: ["Bar"])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/Foo/ViewModel.swift"),
        "public final class FeatureViewModel {}\n",
    );
    write(
        &repo.path().join("Sources/Bar/ViewModel.swift"),
        "public final class FeatureViewModel {}\n",
    );
    write(
        &repo.path().join("Tests/BarTests/ViewModelTests.swift"),
        r#"@testable import Bar
import Testing

@Test
func barFeatureViewModelExists() {
    _ = FeatureViewModel()
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Sources/Foo/ViewModel.swift", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "Tests/BarTests/ViewModelTests.swift"),
        "Swift proof must require the test to import the anchor target module, not only share symbol names in one package: {proof:#}"
    );
}

#[test]
fn swift_symbol_reference_xref_scope_includes_package_root_and_target() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    for package in ["A", "B"] {
        write(
            &repo
                .path()
                .join(format!("Packages/{package}/Package.swift")),
            &format!(
                r#"// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "{package}",
    targets: [
        .target(name: "Core")
    ]
)
"#
            ),
        );
    }
    write(
        &repo.path().join("Packages/A/Sources/Core/Model.swift"),
        "public struct SharedModel {}\n",
    );
    write(
        &repo.path().join("Packages/A/Sources/Core/UseModel.swift"),
        "func useSharedModel() { _ = SharedModel() }\n",
    );
    write(
        &repo.path().join("Packages/B/Sources/Core/Other.swift"),
        "func useSharedModel() { _ = SharedModel() }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "Packages/A/Sources/Core/Model.swift",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let incoming = cone["incoming"].as_array().expect("incoming");
    assert!(
        incoming.iter().any(
            |edge| edge["from"] == "Packages/A/Sources/Core/UseModel.swift"
                && edge["evidence"] == "same_package_symbol_reference"
        ),
        "same package root and target should still produce Swift symbol xref: {cone:#}"
    );
    assert!(
        incoming
            .iter()
            .all(|edge| edge["from"] != "Packages/B/Sources/Core/Other.swift"),
        "Swift symbol xref must not cross nested packages that reuse the same target name: {cone:#}"
    );
}

#[test]
fn swift_symbol_reference_proof_ignores_commented_test_imports() {
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
    name: "CommentedImport",
    targets: [
        .target(name: "Foo"),
        .testTarget(name: "FooTests", dependencies: [])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/Foo/ViewModel.swift"),
        "public final class FeatureViewModel {}\n",
    );
    write(
        &repo.path().join("Tests/FooTests/ViewModelTests.swift"),
        r#"/*
@testable import Foo
*/
import Testing

@Test
func mentionsFeatureViewModelWithoutImportingFoo() {
    _ = FeatureViewModel.self
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let test_file = run_json(
        repo.path(),
        cache.path(),
        &[
            "ls",
            "Tests/FooTests/ViewModelTests.swift",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &test_file);
    let imports = test_file["anchor"]["imports"].as_array().expect("imports");
    assert!(
        imports.iter().all(|import| import != "Foo"),
        "Swift imports inside block comments must not become structural imports: {test_file:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Sources/Foo/ViewModel.swift", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "Tests/FooTests/ViewModelTests.swift"),
        "commented Swift imports must not unlock high-strength symbol-reference proof: {proof:#}"
    );
}

#[test]
fn swift_symbol_reference_proof_ignores_commented_anchor_symbols() {
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
    name: "CommentedSymbol",
    targets: [
        .target(name: "Foo"),
        .testTarget(name: "FooTests", dependencies: ["Foo"])
    ]
)
"#,
    );
    write(
        &repo.path().join("Sources/Foo/Legacy.swift"),
        r#"/*
public final class FeatureViewModel {}
*/
public struct RealThing {}
"#,
    );
    write(
        &repo.path().join("Tests/FooTests/FeatureTests.swift"),
        r#"@testable import Foo
import Testing

@Test
func mentionsRemovedFeatureViewModel() {
    _ = FeatureViewModel.self
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let anchor_file = run_json(
        repo.path(),
        cache.path(),
        &["ls", "Sources/Foo/Legacy.swift", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &anchor_file);
    let symbols = anchor_file["anchor"]["symbols"]
        .as_array()
        .expect("symbols");
    assert!(
        symbols
            .iter()
            .all(|symbol| symbol["name"] != "FeatureViewModel"),
        "Swift symbols inside block comments must not become anchor symbols: {anchor_file:#}"
    );
    assert!(
        symbols.iter().any(|symbol| symbol["name"] == "RealThing"),
        "real Swift symbols should still be surfaced after comment stripping: {anchor_file:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "Sources/Foo/Legacy.swift", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|surface| surface["path"] != "Tests/FooTests/FeatureTests.swift"),
        "commented Swift anchor symbols must not unlock high-strength proof: {proof:#}"
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
fn root_ls_balances_directory_edges_across_structural_sources() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "balanced-root-fixture",
  "private": true
}
"#,
    );
    for index in 0..30 {
        write(
            &repo
                .path()
                .join(format!("packages/noisy-{index:02}/src/index.ts")),
            &format!("export const noisy{index:02} = {index};\n"),
        );
        write(
            &repo
                .path()
                .join(format!("apps/control-center/src/use-{index:02}.ts")),
            &format!(
                "import {{ noisy{index:02} }} from '../../../packages/noisy-{index:02}/src/index';\nexport const use{index:02} = noisy{index:02};\n"
            ),
        );
    }
    write(
        &repo.path().join("packages/shared/src/index.ts"),
        "export const shared = true;\n",
    );
    write(
        &repo.path().join("services/api/src/use-shared.ts"),
        "import { shared } from '../../../packages/shared/src/index';\nexport const apiUsesShared = shared;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--limit", "8", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &json);
    let edges = json["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 8);

    let froms = edges
        .iter()
        .map(|edge| edge["from"].as_str().expect("edge from"))
        .collect::<Vec<_>>();
    assert!(
        froms.contains(&"apps/control-center/"),
        "noisy source should still be represented: {json:#}"
    );
    assert!(
        froms.contains(&"services/api/"),
        "bounded root map should preserve a second structural source instead of letting one source consume the edge budget: {json:#}"
    );
    assert!(
        froms
            .iter()
            .filter(|from| **from == "apps/control-center/")
            .count()
            < edges.len(),
        "default root edge budget must not be monopolized by one noisy source: {json:#}"
    );
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"] == "directory edges hidden by limit"),
        "hidden edge count should still make the bounded cut explicit: {json:#}"
    );
    assert_eq!(json.get("read_first"), None);
}

#[test]
fn root_ls_preserves_rust_workspace_package_edges_under_shared_src_parent() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = ["src/app", "src/core", "src/config"]
resolver = "2"
"#,
    );
    write(
        &repo.path().join("src/app/Cargo.toml"),
        r#"[package]
name = "app"
version = "0.1.0"
edition = "2021"

[dependencies]
masque-core = { path = "../core" }
silentway-config = { path = "../config" }
"#,
    );
    write(
        &repo.path().join("src/core/Cargo.toml"),
        r#"[package]
name = "masque-core"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(
        &repo.path().join("src/config/Cargo.toml"),
        r#"[package]
name = "silentway-config"
version = "0.1.0"
edition = "2021"
"#,
    );
    write(&repo.path().join("src/app/src/lib.rs"), "pub fn app() {}\n");
    write(
        &repo.path().join("src/core/src/lib.rs"),
        "pub fn core() {}\n",
    );
    write(
        &repo.path().join("src/config/src/lib.rs"),
        "pub fn config() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    let edges = json["edges"].as_array().expect("edges");
    assert!(
        edges.iter().any(|edge| edge["type"] == "package_internal"
            && edge["from"] == "src/app/"
            && edge["to"] == "src/core/"
            && edge["evidence"]
                .as_str()
                .unwrap_or_default()
                .contains("masque-core")),
        "root map must keep package endpoints under shared src parent instead of collapsing them away: {json:#}"
    );
    assert!(
        edges.iter().any(|edge| edge["type"] == "package_internal"
            && edge["from"] == "src/app/"
            && edge["to"] == "src/config/"
            && edge["evidence"]
                .as_str()
                .unwrap_or_default()
                .contains("silentway-config")),
        "root map should preserve each structural package dependency under src/: {json:#}"
    );
    assert!(
        !edges
            .iter()
            .any(|edge| edge["from"] == "src/" && edge["to"] == "src/"),
        "self-collapsed src edges are not useful map output: {json:#}"
    );
    assert_eq!(json.get("read_first"), None);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", ".", "--depth", "2", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    let outgoing = cone["outgoing"].as_array().expect("outgoing");
    assert!(
        outgoing
            .iter()
            .any(|edge| edge["type"] == "package_internal"
                && edge["from"] == "src/app/"
                && edge["to"] == "src/core/"),
        "directory cone should keep shared-src package endpoints instead of expanding to files: {cone:#}"
    );
    assert!(
        !outgoing
            .iter()
            .any(|edge| edge["from"] == "src/" && edge["to"] == "src/"),
        "directory cone should not expose self-collapsed shared-parent edges: {cone:#}"
    );
}

#[test]
fn zero_config_roles_do_not_label_project_maps_or_routes_as_codemap_engine() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "role-noise-fixture",
  "private": true
}
"#,
    );
    write(&repo.path().join(".agents/system_map.md"), "# System map\n");
    write(&repo.path().join("artifacts/proof-map.json"), "{}\n");
    write(
        &repo.path().join("app/api/auth/route.ts"),
        "export async function POST() {\n  return Response.json({ ok: true });\n}\n",
    );
    write(
        &repo.path().join("harness/cone-probe.ts"),
        "export const probe = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let root = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &root);
    assert!(
        root["directory"]
            .as_array()
            .expect("directory surfaces")
            .iter()
            .all(|surface| surface["kind"] != "map_engine"),
        "project-local maps/routes/proof artifacts should not be mislabeled as the codemap engine role: {root:#}"
    );

    for path in [
        ".agents/system_map.md",
        "artifacts/proof-map.json",
        "app/api/auth/route.ts",
        "harness/cone-probe.ts",
    ] {
        let file = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &file);
        assert_ne!(
            file["anchor"]["kind"], "map_engine",
            "{path} should keep its real file kind instead of codemap-specific noise: {file:#}"
        );
        assert!(
            file["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .all(|role| role != "map_engine"),
            "{path} should not carry codemap-specific map_engine role: {file:#}"
        );
    }
}

#[test]
fn directory_cone_stays_at_directory_level_without_file_galaxy() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "directory-cone-fixture",
  "private": true,
  "scripts": { "test:e2e": "playwright test" }
}
"#,
    );
    write(
        &repo.path().join("app/page.tsx"),
        "import { Hero } from './_landing/hero';\nimport { userSchema } from '../src/schema/user.dto';\n\nexport default function Page() {\n  return <Hero title={userSchema.name} />;\n}\n",
    );
    write(
        &repo.path().join("app/_landing/hero.tsx"),
        "import { Button } from '../../src/design';\n\nexport function Hero() {\n  return <Button>Start</Button>;\n}\n",
    );
    write(
        &repo.path().join("app/api/logout/route.ts"),
        "import { logout } from '../../../src/lib/server/auth';\n\nexport async function POST() {\n  return logout();\n}\n",
    );
    write(
        &repo.path().join("src/design/index.ts"),
        "export function Button(props: { children: string }) {\n  return props.children;\n}\n",
    );
    write(
        &repo.path().join("src/lib/server/auth.ts"),
        "export function logout() {\n  return Response.json({ ok: true });\n}\n",
    );
    write(
        &repo.path().join("src/schema/user.dto.ts"),
        "export const userSchema = { name: 'user' };\n",
    );
    write(
        &repo.path().join("tests/e2e/smoke.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('home route smoke', async ({ page }) => {\n  await page.goto('/');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &["cone", "app", "--depth", "1", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(cone["anchor"]["kind"], "directory");
    assert!(
        cone["outgoing"]
            .as_array()
            .expect("outgoing")
            .iter()
            .any(|edge| edge["from"] == "app/" && edge["to"] == "app/_landing/"),
        "directory cone should show same-level child edges instead of file imports: {cone:#}"
    );
    assert!(
        cone["outgoing"]
            .as_array()
            .expect("outgoing")
            .iter()
            .any(|edge| edge["from"] == "app/api/" && edge["to"] == "src/"),
        "directory cone should preserve external domain edges at this level: {cone:#}"
    );
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "tests/"
                && edge["to"] == "app/"
                && edge["evidence"] == "e2e_route"),
        "directory cone should aggregate e2e proof to the directory level: {cone:#}"
    );
    assert!(
        cone["contracts"]
            .as_array()
            .expect("contracts")
            .iter()
            .any(|edge| edge["from"] == "app/"
                && edge["to"] == "src/"
                && edge["evidence"] == "role:schema_contract"),
        "directory cone should preserve contract/schema edges at the directory level: {cone:#}"
    );
    let mut proof_keys = Vec::new();
    for edge in cone["proof"].as_array().expect("proof") {
        let key = (
            edge["from"].as_str().unwrap_or_default(),
            edge["to"].as_str().unwrap_or_default(),
            edge["type"].as_str().unwrap_or_default(),
        );
        assert!(
            !proof_keys.contains(&key),
            "directory proof should keep one strongest edge per endpoint: {cone:#}"
        );
        proof_keys.push(key);
    }
    for section in ["outgoing", "incoming", "proof", "contracts", "boundary"] {
        assert!(
            cone[section]
                .as_array()
                .expect("edge section")
                .iter()
                .all(|edge| {
                    ["from", "to"].into_iter().all(|key| {
                        let value = edge[key].as_str().unwrap_or_default();
                        !(value.ends_with(".ts")
                            || value.ends_with(".tsx")
                            || value.ends_with(".js")
                            || value.ends_with(".jsx"))
                    })
                }),
            "directory cone should not leak file-level endpoints in {section}: {cone:#}"
        );
    }

    let deeper = run_json(
        repo.path(),
        cache.path(),
        &["cone", "app", "--depth", "2", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &deeper);
    assert!(
        deeper["outgoing"]
            .as_array()
            .expect("deeper outgoing")
            .iter()
            .any(|edge| edge["from"] == "app/api/logout/" && edge["to"] == "src/lib/"),
        "directory cone --depth 2 should reveal the next external layer without file endpoints: {deeper:#}"
    );
    assert!(
        deeper["contracts"]
            .as_array()
            .expect("deeper contracts")
            .iter()
            .any(|edge| edge["from"] == "app/" && edge["to"] == "src/schema/"),
        "directory cone --depth 2 should reveal schema contract layer without file endpoints: {deeper:#}"
    );
}

#[test]
fn tsconfig_jsonc_path_aliases_create_reverse_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "jsonc-alias-fixture",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("tsconfig.json"),
        r#"{
  // Real tsconfig files commonly use JSONC syntax.
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": [
        "./src/*",
      ],
    },
  },
  "include": ["**/*.ts", "**/*.tsx"],
  "exclude": [
    "node_modules",
  ],
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/studio-shell.tsx"),
        "export function StudioShell() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/app/page.tsx"),
        "import { StudioShell } from '@/features/studio/studio-shell';\n\nexport default function Page() {\n  return <StudioShell />;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/features/studio/studio-shell.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(cone["anchor"]["imported_by_count"], 1);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"] == "app/app/page.tsx"
                && edge["to"] == "src/features/studio/studio-shell.tsx"
                && edge["type"] == "imported_by"
                && edge["evidence"] == "reverse_import"),
        "JSONC tsconfig path aliases should produce reverse structural edges: {cone:#}"
    );
    assert_eq!(cone.get("read_first"), None);
}

#[test]
fn malformed_tsconfig_jsonc_does_not_create_alias_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "malformed-jsonc-alias-fixture",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
/* unterminated
"#,
    );
    write(
        &repo.path().join("src/features/studio/studio-shell.tsx"),
        "export function StudioShell() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/app/page.tsx"),
        "import { StudioShell } from '@/features/studio/studio-shell';\n\nexport default function Page() {\n  return <StudioShell />;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/features/studio/studio-shell.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert_eq!(cone["anchor"]["imported_by_count"], 0);
    assert!(
        cone["incoming"].as_array().expect("incoming").is_empty(),
        "malformed tsconfig JSONC must fail closed instead of creating alias edges: {cone:#}"
    );
    assert_eq!(cone.get("read_first"), None);
}

#[test]
fn proof_links_next_route_files_to_e2e_route_visits() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "next-route-proof-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run",
    "test:e2e": "playwright test"
  }
}
"#,
    );
    write(
        &repo.path().join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": { "@/*": ["./src/*"] }
  }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/studio-shell.tsx"),
        "export function StudioShell() {\n  return <main data-testid=\"studio-shell\" />;\n}\n",
    );
    write(
        &repo.path().join("app/app/page.tsx"),
        "import { StudioShell } from '@/features/studio/studio-shell';\n\nexport default function StudioAppPage() {\n  return <StudioShell />;\n}\n",
    );
    write(
        &repo.path().join("tests/e2e/studio.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('/app renders studio', async ({ page }) => {\n  await page.goto('/app');\n  await expect(page.locator('[data-testid=\"studio-shell\"]')).toBeVisible();\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/not-app.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('does not land on app', async ({ page }) => {\n  await page.goto('/login');\n  await expect(page).not.toHaveURL('/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/same-line-not-app.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('does not land on app in one line', async ({ page }) => {\n  await page.goto('/login'); await expect(page).not.toHaveURL('/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/href-only.spec.ts"),
        "import { test, expect } from '@playwright/test';\n\ntest('renders app link without visiting it', async ({ page }) => {\n  await page.goto('/login');\n  await expect(page.getByRole('link')).toHaveAttribute('href', '/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/commented-route.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('commented navigation is ignored', async ({ page }) => {\n  // await page.goto('/app');\n  await page.goto('/login');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/not-playwright-page.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('non-page object goto is ignored', async () => {\n  const notPage = { goto(_path: string) {} };\n  notPage.goto('/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/goto-url.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('goto-like method is ignored', async ({ page }) => {\n  page.gotoURL('/app');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/dynamic-user.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route smoke', async ({ page }) => {\n  await page.goto('/users/123');\n});\n",
    );
    write(
        &repo
            .path()
            .join("tests/e2e/dynamic-user-extra-segment.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route extra segment is different route', async ({ page }) => {\n  await page.goto('/users/123/settings');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/dynamic-user-missing.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('dynamic route missing segment is different route', async ({ page }) => {\n  await page.goto('/users');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/docs-catchall.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('docs catchall route smoke', async ({ page }) => {\n  await page.goto('/docs/getting-started/install');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/docs-root.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('docs root route smoke', async ({ page }) => {\n  await page.goto('/docs');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/blog-root.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('blog optional catchall root route smoke', async ({ page }) => {\n  await page.goto('/blog');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/nested-admin.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('nested app route smoke', async ({ page }) => {\n  await page.goto('/admin');\n});\n",
    );
    write(
        &repo.path().join("tests/e2e/package-app-dashboard.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('package named app route smoke', async ({ page }) => {\n  await page.goto('/dashboard');\n});\n",
    );
    write(
        &repo.path().join("app/users/[id]/page.tsx"),
        "export default function UserPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/docs/[...slug]/page.tsx"),
        "export default function DocsPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("app/blog/[[...slug]]/page.tsx"),
        "export default function BlogPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("apps/web/src/app/admin/page.tsx"),
        "export default function NestedAdminPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("packages/app/app/dashboard/page.tsx"),
        "export default function PackageAppDashboardPage() {\n  return null;\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let route_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "app/app/page.tsx", "--format", "json"],
    );
    assert_schema("schemas/proof.schema.json", &route_proof);
    assert!(
        route_proof["proofs"]
            .as_array()
            .expect("route proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/studio.spec.ts"
                && proof["evidence"] == "e2e_route"
                && proof["strength"] == "high"),
        "Next route file should map to exact e2e page.goto route proof: {route_proof:#}"
    );
    for false_proof in [
        "tests/e2e/not-app.spec.ts",
        "tests/e2e/same-line-not-app.spec.ts",
        "tests/e2e/href-only.spec.ts",
        "tests/e2e/commented-route.spec.ts",
        "tests/e2e/not-playwright-page.spec.ts",
        "tests/e2e/goto-url.spec.ts",
    ] {
        assert!(
            route_proof["proofs"]
                .as_array()
                .expect("route proofs")
                .iter()
                .all(|proof| proof["path"] != false_proof),
            "non-navigation route literals must not become e2e_route proof: {route_proof:#}"
        );
    }

    let shell_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "src/features/studio/studio-shell.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &shell_proof);
    assert!(
        shell_proof["proofs"]
            .as_array()
            .expect("shell proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/studio.spec.ts"
                && proof["evidence"] == "e2e_route"),
        "route e2e proof should be available to the shell through its direct route consumer: {shell_proof:#}"
    );

    let dynamic_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "app/users/[id]/page.tsx", "--format", "json"],
    );
    assert!(
        dynamic_proof["proofs"]
            .as_array()
            .expect("dynamic proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/dynamic-user.spec.ts"
                && proof["evidence"] == "e2e_route"
                && proof["strength"] == "high"),
        "dynamic route proof should map [id] to a concrete page.goto segment: {dynamic_proof:#}"
    );
    for false_proof in [
        "tests/e2e/dynamic-user-extra-segment.spec.ts",
        "tests/e2e/dynamic-user-missing.spec.ts",
    ] {
        assert!(
            dynamic_proof["proofs"]
                .as_array()
                .expect("dynamic proofs")
                .iter()
                .all(|proof| proof["path"] != false_proof),
            "dynamic route proof must not overmatch sibling route shapes: {dynamic_proof:#}"
        );
    }

    let catchall_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "app/docs/[...slug]/page.tsx", "--format", "json"],
    );
    assert!(
        catchall_proof["proofs"]
            .as_array()
            .expect("catchall proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/docs-catchall.spec.ts"
                && proof["evidence"] == "e2e_route"),
        "catch-all route proof should map [...slug] to a deeper page.goto route: {catchall_proof:#}"
    );
    assert!(
        catchall_proof["proofs"]
            .as_array()
            .expect("catchall proofs")
            .iter()
            .all(|proof| proof["path"] != "tests/e2e/docs-root.spec.ts"),
        "non-optional catch-all route must not match the route root: {catchall_proof:#}"
    );

    let optional_catchall_proof = run_json(
        repo.path(),
        cache.path(),
        &["proof", "app/blog/[[...slug]]/page.tsx", "--format", "json"],
    );
    assert!(
        optional_catchall_proof["proofs"]
            .as_array()
            .expect("optional catchall proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/blog-root.spec.ts"
                && proof["evidence"] == "e2e_route"),
        "optional catch-all should match the route root: {optional_catchall_proof:#}"
    );

    let nested_app_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "apps/web/src/app/admin/page.tsx",
            "--format",
            "json",
        ],
    );
    assert!(
        nested_app_proof["proofs"]
            .as_array()
            .expect("nested app proofs")
            .iter()
            .any(|proof| proof["path"] == "tests/e2e/nested-admin.spec.ts"
                && proof["evidence"] == "e2e_route"),
        "Next route proof should work in nested monorepo src/app layouts: {nested_app_proof:#}"
    );

    let package_named_app_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/app/dashboard/page.tsx",
            "--format",
            "json",
        ],
    );
    assert!(
        package_named_app_proof["proofs"]
            .as_array()
            .expect("package named app proofs")
            .iter()
            .any(
                |proof| proof["path"] == "tests/e2e/package-app-dashboard.spec.ts"
                    && proof["evidence"] == "e2e_route"
            ),
        "Next route proof should use the final /app/ route root when a package is named app: {package_named_app_proof:#}"
    );
    assert_eq!(route_proof.get("read_first"), None);
}

#[test]
fn proof_does_not_link_ambiguous_duplicate_next_routes() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "duplicate-next-route-fixture",
  "private": true,
  "scripts": { "test:e2e": "playwright test" }
}
"#,
    );
    write(
        &repo.path().join("apps/web/src/app/admin/page.tsx"),
        "export default function WebAdminPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("apps/web/app/admin/page.tsx"),
        "export default function LegacyWebAdminPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("apps/ops/src/app/admin/page.tsx"),
        "export default function OpsAdminPage() {\n  return null;\n}\n",
    );
    write(
        &repo.path().join("tests/e2e/admin.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('admin route smoke', async ({ page }) => {\n  await page.goto('/admin');\n});\n",
    );
    write(
        &repo.path().join("apps/web/tests/e2e/admin.spec.ts"),
        "import { test } from '@playwright/test';\n\ntest('web admin route smoke', async ({ page }) => {\n  await page.goto('/admin');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "apps/web/src/app/admin/page.tsx",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proof surfaces")
            .iter()
            .all(|surface| surface["evidence"] != "e2e_route"),
        "root e2e route proof must not cross domains when two app roots expose the same route: {proof:#}"
    );
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
    assert_eq!(validation["schema_version"], "4");
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
    assert!(
        validation["details"]
            .as_array()
            .expect("details")
            .iter()
            .any(|detail| detail["kind"] == "forbidden_boundary"
                && detail["next"]
                    .as_array()
                    .expect("next")
                    .iter()
                    .any(|command| command == "codemap boundaries")),
        "boundary details should point to the structural boundary map command: {validation:#}"
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
            && detail["status"] == "ok"
            && detail["next"]
                .as_array()
                .expect("next")
                .iter()
                .all(|command| command == "codemap anchors validate")),
        "valid loaded config should keep ok detail but avoid map commands while validation is not ok: {validation:#}"
    );
    assert!(
        details.iter().any(|detail| detail["kind"] == "config"
            && detail["id"] == "packages/bad/.ctx.yml"
            && detail["status"] == "problem"),
        "rejected nested config should carry the problem detail: {validation:#}"
    );
}

#[test]
fn anchors_validate_problem_details_keep_next_diagnostic_only() {
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
boundaries:
  forbidden:
    - from: src/**
      to: tests/**
verification:
  default:
    - ""
"#,
    );
    write(&repo.path().join("src/app.ts"), "export const app = 1;\n");
    write(
        &repo.path().join("tests/app.test.ts"),
        "test('app', () => {});\n",
    );
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
    for kind in ["domain", "forbidden_boundary", "verification_default"] {
        assert!(
            details.iter().any(|detail| detail["kind"] == kind
                && detail["next"]
                    .as_array()
                    .expect("next")
                    .iter()
                    .all(|command| command == "codemap anchors validate")),
            "when anchor validation is not ok, {kind} detail must not point at fail-closed map commands: {validation:#}"
        );
    }
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
  app.features:
    role: feature_surface
    files:
      - src/**/*.ts
    invariants:
      - mapped_by_files
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
                .contains("path `src` exists")
            && detail["next"]
                .as_array()
                .expect("next")
                .iter()
                .any(|command| command == "codemap ls src")),
        "domain detail should explain resolved path: {validation:#}"
    );
    assert!(
        details.iter().any(|detail| detail["kind"] == "concept"
            && detail["id"] == "app.entry"
            && detail["status"] == "ok"
            && detail["message"]
                .as_str()
                .unwrap_or_default()
                .contains("exact files resolved: 1")
            && detail["next"]
                .as_array()
                .expect("next")
                .iter()
                .any(|command| command == "codemap cone src/app.ts --depth 1")),
        "concept detail should explain file and invariant resolution: {validation:#}"
    );
    assert!(
        details.iter().any(|detail| detail["kind"] == "concept"
            && detail["id"] == "app.features"
            && detail["status"] == "ok"
            && detail["message"]
                .as_str()
                .unwrap_or_default()
                .contains("glob matches: 1")
            && detail["next"]
                .as_array()
                .expect("next")
                .iter()
                .any(|command| command == "codemap files --path src")),
        "glob concept details should point to bounded files listing, not a non-anchor glob ls: {validation:#}"
    );
    assert!(
        details
            .iter()
            .any(|detail| detail["kind"] == "verification_default"
                && detail["status"] == "ok"
                && detail["message"] == "pnpm test"
                && detail["next"]
                    .as_array()
                    .expect("next")
                    .iter()
                    .any(|command| command == "codemap proof --changed")),
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
fn graph_causal_root_hides_support_packages_until_scoped() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("fixtures/example/package.json"),
        r#"{"name":"fixture-package","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("fixtures/example/src/index.ts"),
        "export const fixture = true;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture support package"]);

    let root_graph = run_json(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "causal", "--format", "json"],
    );
    assert_schema("schemas/graph.schema.json", &root_graph);
    assert!(
        root_graph["nodes"]
            .as_array()
            .expect("root graph nodes")
            .iter()
            .all(|node| !node.as_str().unwrap_or_default().starts_with("fixtures/")),
        "root graph should not be dominated by fixture/example package internals: {root_graph:#}"
    );

    let fixture_graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph", "--path", "fixtures", "--lens", "causal", "--format", "json",
        ],
    );
    assert!(
        fixture_graph["nodes"]
            .as_array()
            .expect("fixture graph nodes")
            .iter()
            .any(|node| node == "fixtures/example/package.json"),
        "explicit fixture graph scope should still reveal fixture package nodes: {fixture_graph:#}"
    );
}

#[test]
fn graph_proof_lens_uses_explicit_path_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/open-panel.ts"),
        "export function openPanel() {\n  return 'open';\n}\n",
    );
    write(
        &repo.path().join("packages/app/tests/open-panel.test.ts"),
        "import { openPanel } from '../src/open-panel';\n\ntest('opens the panel', () => {\n  expect(openPanel()).toBe('open');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof graph fixture"]);

    let root_graph = run_json(
        repo.path(),
        cache.path(),
        &["graph", "--lens", "proof", "--format", "json"],
    );
    assert_schema("schemas/graph.schema.json", &root_graph);
    assert!(
        root_graph["nodes"]
            .as_array()
            .expect("root nodes")
            .is_empty(),
        "root proof graph should not expand into the whole test galaxy without an anchor: {root_graph:#}"
    );

    let scoped_graph = run_json(
        repo.path(),
        cache.path(),
        &[
            "graph",
            "--path",
            "packages/app/src/open-panel.ts",
            "--lens",
            "proof",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/graph.schema.json", &scoped_graph);
    assert!(
        scoped_graph["nodes"]
            .as_array()
            .expect("scoped nodes")
            .iter()
            .any(|node| node == "packages/app/tests/open-panel.test.ts"),
        "explicit path proof lens should show bounded proof nodes for that scope: {scoped_graph:#}"
    );
    assert!(
        scoped_graph["edges"]
            .as_array()
            .expect("scoped edges")
            .iter()
            .any(|edge| {
                edge["from"] == "packages/app/tests/open-panel.test.ts"
                    && edge["to"] == "packages/app/src/open-panel.ts"
                    && edge["type"] == "tests"
            }),
        "explicit path proof lens should render proof edges, not an empty graph: {scoped_graph:#}"
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
fn symbol_anchor_cone_follows_export_star_barrel_consumers() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-core.ts"),
        "export function pickFocusForSelection(selection: Set<string>, orderedIds: string[]): string | null {\n  return orderedIds.find((id) => selection.has(id)) ?? null;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-barrel.ts"),
        "export * from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-consumer.ts"),
        "import { pickFocusForSelection } from './selection-barrel';\n\nexport const selectedFocus = pickFocusForSelection(new Set(['a']), ['a']);\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-core.test.ts"),
        "import { pickFocusForSelection } from './selection-barrel';\n\ntest('selection focus', () => {\n  expect(pickFocusForSelection(new Set(['a']), ['a'])).toBe('a');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "symbol barrel fixture"]);

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/selection-consumer.ts"
                && edge["evidence"] == "reexported_symbol_reference"),
        "symbol xref should follow explicit export-star barrels to concrete consumers: {cone:#}"
    );
    assert!(
        cone["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/selection-core.test.ts"
                && edge["evidence"] == "test_reexported_symbol_reference"),
        "symbol proof should follow exact re-export barrels used by tests: {cone:#}"
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/features/studio/canvas/selection-core.ts#pickFocusForSelection",
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
                == "packages/app/src/features/studio/canvas/selection-core.test.ts"
                && surface["evidence"] == "test_reexported_symbol_reference"),
        "proof command should expose re-exported symbol test evidence: {proof:#}"
    );
}

#[test]
fn symbol_anchor_cone_follows_named_reexport_barrel_aliases() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-core.ts"),
        "export function pickFocusForSelection(selection: Set<string>, orderedIds: string[]): string | null {\n  return orderedIds.find((id) => selection.has(id)) ?? null;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-barrel.ts"),
        "export { pickFocusForSelection as publicPickFocus } from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-consumer.ts"),
        "import { publicPickFocus as usePickFocus } from './selection-barrel';\n\nexport const selectedFocus = usePickFocus(new Set(['a']), ['a']);\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "symbol named barrel fixture"],
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/selection-consumer.ts"
                && edge["evidence"] == "reexported_symbol_reference"),
        "symbol xref should follow exact named re-export aliases to concrete consumers: {cone:#}"
    );
}

#[test]
fn symbol_anchor_cone_follows_transitive_reexport_barrel_chains() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-core.ts"),
        "export function pickFocusForSelection(selection: Set<string>, orderedIds: string[]): string | null {\n  return orderedIds.find((id) => selection.has(id)) ?? null;\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-mid-barrel.ts"),
        "export * from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-index.ts"),
        "export * from './selection-mid-barrel';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-star-consumer.ts"),
        "import { pickFocusForSelection } from './selection-index';\n\nexport const selectedFocus = pickFocusForSelection(new Set(['a']), ['a']);\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-alias-mid.ts"),
        "export { pickFocusForSelection as publicPickFocus } from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-alias-index.ts"),
        "export { publicPickFocus } from './selection-alias-mid';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-alias-consumer.ts"),
        "import { publicPickFocus } from './selection-alias-index';\n\nexport const selectedFocus = publicPickFocus(new Set(['a']), ['a']);\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "symbol transitive barrel fixture"],
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    for consumer in [
        "packages/app/src/features/studio/canvas/transitive-star-consumer.ts",
        "packages/app/src/features/studio/canvas/transitive-alias-consumer.ts",
    ] {
        assert!(
            cone["incoming"]
                .as_array()
                .expect("incoming")
                .iter()
                .any(|edge| {
                    edge["from"] == consumer && edge["evidence"] == "reexported_symbol_reference"
                }),
            "symbol xref should follow bounded transitive re-export chains for {consumer}: {cone:#}"
        );
    }
}

#[test]
fn symbol_anchor_cone_follows_target_local_export_lists() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-core.ts"),
        "function pickFocusForSelection(selection: Set<string>, orderedIds: string[]): string | null {\n  return orderedIds.find((id) => selection.has(id)) ?? null;\n}\n\nexport { pickFocusForSelection };\nexport { pickFocusForSelection as publicPickFocus };\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-barrel.ts"),
        "export * from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/direct-consumer.ts"),
        "import { pickFocusForSelection } from './selection-core';\n\nexport const selectedFocus = pickFocusForSelection(new Set(['a']), ['a']);\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/barrel-consumer.ts"),
        "import { pickFocusForSelection } from './selection-barrel';\n\nexport const selectedFocus = pickFocusForSelection(new Set(['a']), ['a']);\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/alias-consumer.ts"),
        "import { publicPickFocus } from './selection-core';\n\nexport const selectedFocus = publicPickFocus(new Set(['a']), ['a']);\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/barrel-alias-consumer.ts"),
        "import { publicPickFocus } from './selection-barrel';\n\nexport const selectedFocus = publicPickFocus(new Set(['a']), ['a']);\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "symbol local export list fixture"],
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    for consumer in [
        "packages/app/src/features/studio/canvas/direct-consumer.ts",
        "packages/app/src/features/studio/canvas/barrel-consumer.ts",
        "packages/app/src/features/studio/canvas/alias-consumer.ts",
        "packages/app/src/features/studio/canvas/barrel-alias-consumer.ts",
    ] {
        assert!(
            cone["incoming"]
                .as_array()
                .expect("incoming")
                .iter()
                .any(|edge| edge["from"] == consumer),
            "target-side local export lists should create structural symbol xrefs for {consumer}: {cone:#}"
        );
    }
}

#[test]
fn symbol_anchor_cone_rejects_inexact_type_only_and_shadowed_barrel_reexports() {
    let (repo, cache) = fixture();
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/selection-core.ts"),
        "export type SelectionFocus = string;\n\nexport function pickFocusForSelection(selection: Set<string>, orderedIds: string[]): string | null {\n  return orderedIds.find((id) => selection.has(id)) ?? null;\n}\n\nexport function otherSymbol() {\n  return 'other';\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/other-core.ts"),
        "export function pickFocusForSelection() {\n  return 'other';\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-core.ts"),
        "export default function PickFocus() {\n  return 'default';\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-list-core.ts"),
        "function pickFocusForSelection() {\n  return 'default';\n}\n\nexport { pickFocusForSelection as default };\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-string-core.ts"),
        "function pickFocusForSelection() {\n  return 'private';\n}\n\nconst docs = `\nexport { pickFocusForSelection };\n`;\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-comment-core.ts"),
        "function pickFocusForSelection() {\n  return 'private';\n}\n\n/*\nexport { pickFocusForSelection };\n*/\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-regex-core.ts"),
        "function pickFocusForSelection() {\n  return 'private';\n}\n\nconst exportSyntaxPattern = /export { pickFocusForSelection }/;\nexport const keep = exportSyntaxPattern.test('x');\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/comment-gap-core.ts"),
        "function localPick() {\n  return 'private-local';\n}\n\nexport { localPick as publicPick } /* valid comment gap */ from './comment-gap-remote';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/comment-gap-remote.ts"),
        "export function localPick() {\n  return 'remote';\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/other-barrel.ts"),
        "export { otherSymbol } from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/type-barrel.ts"),
        "export type { SelectionFocus } from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/star-barrel.ts"),
        "export * from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/conflict-barrel.ts"),
        "export * from './selection-core';\nexport { pickFocusForSelection } from './other-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/same-file-override-barrel.ts"),
        "export * from './selection-core';\nexport { otherSymbol as pickFocusForSelection } from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/commented-reexport-barrel.ts"),
        "export * from './selection-core';\nexport { /* pickFocusForSelection is only a comment */ otherSymbol as pickFocusForSelection } from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/duplicate-star-barrel.ts"),
        "export * from './selection-core';\nexport * from './other-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-duplicate-left.ts"),
        "export * from './selection-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-duplicate-right.ts"),
        "export * from './other-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-duplicate-index.ts"),
        "export * from './transitive-duplicate-left';\nexport * from './transitive-duplicate-right';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-local-override-mid.ts"),
        "export * from './selection-core';\n\nexport function pickFocusForSelection() {\n  return 'local-mid';\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-local-override-index.ts"),
        "export * from './transitive-local-override-mid';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/cycle-left.ts"),
        "export * from './cycle-right';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/cycle-right.ts"),
        "export * from './cycle-left';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/local-barrel.ts"),
        "export * from './selection-core';\n\nexport function pickFocusForSelection() {\n  return 'local';\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/multiline-local-barrel.ts"),
        "import { pickFocusForSelection as otherPickFocus } from './other-core';\nexport * from './selection-core';\nexport {\n  otherPickFocus as pickFocusForSelection,\n};\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/commented-local-barrel.ts"),
        "import { pickFocusForSelection } from './other-core';\nexport * from './selection-core';\nexport {\n  pickFocusForSelection, // from other-core intentionally\n};\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-star-barrel.ts"),
        "export * from './default-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-list-star-barrel.ts"),
        "export * from './default-list-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-transitive-mid.ts"),
        "export * from './default-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-transitive-index.ts"),
        "export * from './default-transitive-mid';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-string-star-barrel.ts"),
        "export * from './fake-string-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-comment-star-barrel.ts"),
        "export * from './fake-comment-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-regex-star-barrel.ts"),
        "export * from './fake-regex-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/comment-gap-star-barrel.ts"),
        "export * from './comment-gap-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-named-barrel.ts"),
        "export { default as PickFocus } from './default-core';\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/not-reexported-consumer.ts"),
        "import { otherSymbol as pickFocusForSelection } from './other-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/type-only-consumer.ts"),
        "import type { SelectionFocus } from './type-barrel';\n\nexport type FocusAlias = SelectionFocus;\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/shadowed-consumer.ts"),
        "import { pickFocusForSelection as localPickFocus } from './star-barrel';\n\nexport function selectedFocus() {\n  const localPickFocus = () => 'local';\n  return localPickFocus();\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/conflict-consumer.ts"),
        "import { pickFocusForSelection } from './conflict-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/same-file-override-consumer.ts"),
        "import { pickFocusForSelection } from './same-file-override-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/commented-reexport-consumer.ts"),
        "import { pickFocusForSelection } from './commented-reexport-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/duplicate-star-consumer.ts"),
        "import { pickFocusForSelection } from './duplicate-star-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-duplicate-consumer.ts"),
        "import { pickFocusForSelection } from './transitive-duplicate-index';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/transitive-local-override-consumer.ts"),
        "import { pickFocusForSelection } from './transitive-local-override-index';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/cycle-consumer.ts"),
        "import { pickFocusForSelection } from './cycle-left';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/local-consumer.ts"),
        "import { pickFocusForSelection } from './local-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/multiline-local-consumer.ts"),
        "import { pickFocusForSelection } from './multiline-local-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/commented-local-consumer.ts"),
        "import { pickFocusForSelection } from './commented-local-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-star-consumer.ts"),
        "import { PickFocus } from './default-star-barrel';\n\nexport const selectedFocus = PickFocus();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-list-star-consumer.ts"),
        "import { default as usePickFocus } from './default-list-star-barrel';\n\nexport const selectedFocus = usePickFocus();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-transitive-star-consumer.ts"),
        "import { PickFocus } from './default-transitive-index';\n\nexport const selectedFocus = PickFocus();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-string-consumer.ts"),
        "import { pickFocusForSelection } from './fake-string-star-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-string-core.test.ts"),
        "import { pickFocusForSelection } from './fake-string-star-barrel';\n\ntest('fake string export is documentation only', () => {\n  expect(pickFocusForSelection()).toBe('private');\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-comment-consumer.ts"),
        "import { pickFocusForSelection } from './fake-comment-star-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-comment-core.test.ts"),
        "import { pickFocusForSelection } from './fake-comment-star-barrel';\n\ntest('fake comment export is documentation only', () => {\n  expect(pickFocusForSelection()).toBe('private');\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-regex-consumer.ts"),
        "import { pickFocusForSelection } from './fake-regex-star-barrel';\n\nexport const selectedFocus = pickFocusForSelection();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/fake-regex-core.test.ts"),
        "import { pickFocusForSelection } from './fake-regex-star-barrel';\n\ntest('fake regex export is syntax text only', () => {\n  expect(pickFocusForSelection()).toBe('private');\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/comment-gap-consumer.ts"),
        "import { publicPick } from './comment-gap-star-barrel';\n\nexport const selectedFocus = publicPick();\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/comment-gap-core.test.ts"),
        "import { publicPick } from './comment-gap-star-barrel';\n\ntest('comment gap re-export stays remote-owned', () => {\n  expect(publicPick()).toBe('remote');\n});\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/src/features/studio/canvas/default-named-consumer.ts"),
        "import { PickFocus } from './default-named-barrel';\n\nexport const selectedFocus = PickFocus();\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "symbol negative barrel fixture"],
    );

    let value_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &value_cone);
    for false_consumer in [
        "packages/app/src/features/studio/canvas/not-reexported-consumer.ts",
        "packages/app/src/features/studio/canvas/shadowed-consumer.ts",
        "packages/app/src/features/studio/canvas/conflict-consumer.ts",
        "packages/app/src/features/studio/canvas/same-file-override-consumer.ts",
        "packages/app/src/features/studio/canvas/commented-reexport-consumer.ts",
        "packages/app/src/features/studio/canvas/duplicate-star-consumer.ts",
        "packages/app/src/features/studio/canvas/transitive-duplicate-consumer.ts",
        "packages/app/src/features/studio/canvas/transitive-local-override-consumer.ts",
        "packages/app/src/features/studio/canvas/cycle-consumer.ts",
        "packages/app/src/features/studio/canvas/local-consumer.ts",
        "packages/app/src/features/studio/canvas/multiline-local-consumer.ts",
        "packages/app/src/features/studio/canvas/commented-local-consumer.ts",
    ] {
        assert!(
            value_cone["incoming"]
                .as_array()
                .expect("incoming")
                .iter()
                .all(|edge| edge["from"] != false_consumer),
            "barrel xref must not link inexact or locally shadowed consumers: {value_cone:#}"
        );
    }

    let other_symbol_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#otherSymbol",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &other_symbol_cone);
    assert!(
        other_symbol_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/same-file-override-consumer.ts"
                && edge["evidence"] == "reexported_symbol_reference"),
        "explicit same-file re-export should resolve to the exact imported symbol binding: {other_symbol_cone:#}"
    );
    assert!(
        other_symbol_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/commented-reexport-consumer.ts"
                && edge["evidence"] == "reexported_symbol_reference"),
        "comments inside re-export clauses must not become imported symbol bindings: {other_symbol_cone:#}"
    );

    let default_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/default-core.ts#PickFocus",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &default_cone);
    assert!(
        default_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"]
                != "packages/app/src/features/studio/canvas/default-star-consumer.ts"),
        "export-star must not expose default export symbol names as named public exports: {default_cone:#}"
    );
    assert!(
        default_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"]
                != "packages/app/src/features/studio/canvas/default-transitive-star-consumer.ts"),
        "transitive export-star must not expose default export symbol names as named public exports: {default_cone:#}"
    );
    assert!(
        default_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .any(|edge| edge["from"]
                == "packages/app/src/features/studio/canvas/default-named-consumer.ts"
                && edge["evidence"] == "reexported_symbol_reference"),
        "explicit default-as named re-export should still link to the default symbol: {default_cone:#}"
    );

    let default_list_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/default-list-core.ts#pickFocusForSelection",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &default_list_cone);
    assert!(
        default_list_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"]
                != "packages/app/src/features/studio/canvas/default-list-star-consumer.ts"),
        "export-star must not expose target-side default export-list aliases as named public exports: {default_list_cone:#}"
    );

    for (anchor, false_consumer, false_test) in [
        (
            "packages/app/src/features/studio/canvas/fake-string-core.ts#pickFocusForSelection",
            "packages/app/src/features/studio/canvas/fake-string-consumer.ts",
            "packages/app/src/features/studio/canvas/fake-string-core.test.ts",
        ),
        (
            "packages/app/src/features/studio/canvas/fake-comment-core.ts#pickFocusForSelection",
            "packages/app/src/features/studio/canvas/fake-comment-consumer.ts",
            "packages/app/src/features/studio/canvas/fake-comment-core.test.ts",
        ),
        (
            "packages/app/src/features/studio/canvas/fake-regex-core.ts#pickFocusForSelection",
            "packages/app/src/features/studio/canvas/fake-regex-consumer.ts",
            "packages/app/src/features/studio/canvas/fake-regex-core.test.ts",
        ),
        (
            "packages/app/src/features/studio/canvas/comment-gap-core.ts#localPick",
            "packages/app/src/features/studio/canvas/comment-gap-consumer.ts",
            "packages/app/src/features/studio/canvas/comment-gap-core.test.ts",
        ),
    ] {
        let fake_cone = run_json(
            repo.path(),
            cache.path(),
            &["cone", anchor, "--format", "json"],
        );
        assert_schema("schemas/cone.schema.json", &fake_cone);
        assert!(
            fake_cone["incoming"]
                .as_array()
                .expect("incoming")
                .iter()
                .all(|edge| edge["from"] != false_consumer),
            "export-list text inside strings/comments must not create re-exported symbol xrefs for {anchor}: {fake_cone:#}"
        );
        assert!(
            fake_cone["proof"]
                .as_array()
                .expect("proof")
                .iter()
                .all(|edge| edge["from"] != false_test),
            "export-list text inside strings/comments must not create proof edges for {anchor}: {fake_cone:#}"
        );
    }

    let type_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/features/studio/canvas/selection-core.ts#SelectionFocus",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &type_cone);
    assert!(
        type_cone["incoming"]
            .as_array()
            .expect("incoming")
            .iter()
            .all(|edge| edge["from"]
                != "packages/app/src/features/studio/canvas/type-only-consumer.ts"),
        "type-only re-export/import must not become a runtime symbol xref: {type_cone:#}"
    );
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
