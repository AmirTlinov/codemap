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
