    #[test]
    fn cargo_paths_resolve_inside_repo_without_root_escape() {
        assert_eq!(
            resolve_repo_relative_path(Path::new("crates/app"), "../renderer").as_deref(),
            Some("crates/renderer")
        );
        assert_eq!(
            resolve_repo_relative_path(Path::new("."), "crates/replay").as_deref(),
            Some("crates/replay")
        );
        assert_eq!(
            resolve_repo_relative_path(Path::new("."), "../external"),
            None
        );
        assert_eq!(
            resolve_repo_relative_path(Path::new("nested"), "../../external"),
            None
        );
        assert_eq!(
            resolve_repo_relative_path(Path::new("."), "/tmp/external"),
            None
        );
        assert!(!cargo_workspace_member_pattern_matches(
            "external",
            "../external"
        ));
    }

    #[test]
    fn pyproject_paths_use_structural_toml() {
        let pyproject = r#"[project]
name = "ctx-renderer"

[tool.uv.sources]
ctx-replay = { path = "../replay,with-comma", marker = "platform_system == 'Darwin,macOS'" }

[tool.poetry.dependencies]
ctx-tools = { path = "../tools" }
ctx-version-only = "^1"
"#;
        assert_eq!(
            pyproject_package_name(pyproject).as_deref(),
            Some("ctx-renderer")
        );
        let deps = pyproject_path_dependencies(pyproject);
        assert!(
            deps.iter()
                .any(|(name, path)| name == "ctx-replay" && path == "../replay,with-comma")
        );
        assert!(
            deps.iter()
                .any(|(name, path)| name == "ctx-tools" && path == "../tools")
        );
        assert!(deps.iter().all(|(name, _)| name != "ctx-version-only"));
    }

    #[test]
    fn pyproject_workspace_patterns_ignore_unrelated_tool_metadata() {
        let pyproject = r#"[project]
name = "ctx-python-workspace"
members = ["services/replay"]
packages = ["apps/api"]

[tool.uv.workspace]
members = ["libs/*"]

[tool.unrelated]
members = ["shadow/replay"]
packages = ["shadow/renderer"]

[tool.poetry]
packages = ["not-a-workspace"]
"#;
        let patterns = pyproject_workspace_patterns(pyproject);
        assert!(patterns.iter().any(|item| item == "services/replay"));
        assert!(patterns.iter().any(|item| item == "apps/api"));
        assert!(patterns.iter().any(|item| item == "libs/*"));
        assert!(patterns.iter().all(|item| item != "shadow/replay"));
        assert!(patterns.iter().all(|item| item != "shadow/renderer"));
        assert!(patterns.iter().all(|item| item != "not-a-workspace"));
    }

    #[test]
    fn javascript_symbols_keep_exports_and_line_ranges() {
        let text = r#"import { frameAt } from "./timeline";

export function seekFrame(timeMs: number): number {
  return frameAt(timeMs);
}

export const FeedPage = () => null;
const useReplayClock = () => 1;
export interface ReplayDto {
  frame: number;
}
"#;

        let symbols = extract_symbols(text, "tsx");

        assert_symbol(&symbols, "seekFrame", "function", true, 3, 5);
        assert_symbol(&symbols, "FeedPage", "component", true, 7, 7);
        assert_symbol(&symbols, "useReplayClock", "hook", false, 8, 8);
        assert_symbol(&symbols, "ReplayDto", "interface", true, 9, 11);
    }

    #[test]
    fn javascript_semicolonless_expression_symbol_does_not_swallow_next_block() {
        let text = r#"export const FeedPage = () => <View />

export function renderFeed() {
  return FeedPage
}
"#;

        let symbols = extract_symbols(text, "tsx");

        assert_symbol(&symbols, "FeedPage", "component", true, 1, 1);
        assert_symbol(&symbols, "renderFeed", "function", true, 3, 5);
    }

    #[test]
    fn javascript_local_export_list_does_not_hide_following_symbols() {
        let text = r#"const Foo = 1;
export { Foo };

export type {
  ReplayDto,
};

export function laterSymbol() {
  return Foo;
}
"#;

        let symbols = extract_symbols(text, "ts");

        assert_symbol(&symbols, "Foo", "const", false, 1, 1);
        assert_symbol(&symbols, "laterSymbol", "function", true, 8, 10);
        assert!(
            symbols.iter().all(|symbol| symbol.name != "ReplayDto"),
            "export-list members are not declarations"
        );
    }

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
        std::fs::create_dir_all(repo.path().join("src/features/studio"))
            .expect("create feature dir");
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

