// Responsibility: repo-tests-playwright
use crate::model::SymbolInfo;
use crate::repo::{
    CacheWriteMode, RootSelection, extract_surfaces, extract_symbols, load_project_with_cache,
    swift_import_re,
};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn javascript_surface_phrases_ignore_getbyrole_inside_non_calls() {
    let real = r#"import { expect, test } from '@playwright/test';

test("export dialog opens", async ({ page }) => {
  await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#;
    let locator_chain = r#"import { expect, test } from '@playwright/test';

test("export dialog opens", async ({ page }) => {
  await expect(page.locator('#root').getByRole('dialog', { name: 'Export' })).toBeVisible();
});
"#;
    let assigned_locator = r#"import { expect, test } from '@playwright/test';

test('export dialog opens', async ({ page, browserName }) => {
  const exportDialog = page.getByRole('dialog', { name: 'Экспорт' })
  await expect(exportDialog).toBeVisible()
});
"#;
    let assigned_locator_after_goto = r#"import { expect, test } from '@playwright/test';

test('command palette opens export dialog', async ({ page }) => {
  await page.goto('/studio');
  const exportDialog = page.getByRole('dialog', { name: 'Export' });
  await expect(exportDialog).toBeVisible();
});
"#;
    let locator_action = r#"import { test } from '@playwright/test';

test('command palette uses export dialog', async ({ page }) => {
  await page.goto('/studio');
  await page.getByRole('dialog', { name: 'Export' }).click();
});
"#;
    let bare_lazy_playwright_locator = r#"import { test } from '@playwright/test';

test('only creates lazy locator', async ({ page }) => {
  await page.goto('/studio');
  page.getByRole('dialog', { name: 'Export' });
});
"#;
    let double_quoted = r#"const docs = "await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible()";"#;
    let single_quoted = r#"const docs = 'await expect(page.getByRole("dialog", { name: "Export" })).toBeVisible()';"#;
    let template = r#"const docs = `await expect(page.getByRole('dialog', { name: 'Export' })).toBeVisible()`;"#;
    let regex = r#"const docs = /page\.getByRole\('dialog', { name: 'Export' }\)/;"#;
    let bare_helper = r#"function getByRole(role, options) { return options; }
pub(crate) const locator = getByRole('dialog', { name: 'Export' });
"#;
    let member_helper = r#"const helper = { getByRole(role, options) { return options; } };
pub(crate) const locator = helper.getByRole('dialog', { name: 'Export' });
"#;

    assert!(
        extract_surfaces(real, "tsx")
            .phrases
            .contains("a11y-role-dialog-name-export")
    );
    assert!(
        extract_surfaces(locator_chain, "tsx")
            .phrases
            .contains("a11y-role-dialog-name-export")
    );
    assert!(
        extract_surfaces(assigned_locator, "tsx")
            .phrases
            .contains("a11y-role-dialog-name-экспорт")
    );
    assert!(
        extract_surfaces(assigned_locator_after_goto, "tsx")
            .phrases
            .contains("a11y-role-dialog-name-export")
    );
    assert!(
        extract_surfaces(locator_action, "tsx")
            .phrases
            .contains("a11y-role-dialog-name-export")
    );
    for text in [
        double_quoted,
        single_quoted,
        template,
        regex,
        bare_helper,
        member_helper,
        bare_lazy_playwright_locator,
    ] {
        let surfaces = extract_surfaces(text, "tsx");
        assert!(
            !surfaces.phrases.contains("a11y-role-dialog-name-export"),
            "getByRole outside a real call must not become proof surface: {surfaces:#?}"
        );
    }
}

#[test]
fn javascript_surface_phrases_reject_getbyrole_opaque_name_overrides() {
    let spread = r#"test("dialog", async ({ page }) => {
  const metadata = { name: 'Import' };
  await expect(page.getByRole('dialog', { name: 'Export', ...metadata })).toBeVisible();
});
"#;
    let duplicate = r#"test("dialog", async ({ page }) => {
  await expect(page.getByRole('dialog', { name: 'Export', name: 'Import' })).toBeVisible();
});
"#;
    for text in [spread, duplicate] {
        let surfaces = extract_surfaces(text, "tsx");
        assert!(
            !surfaces.phrases.contains("a11y-role-dialog-name-export"),
            "opaque Playwright name overrides must fail closed: {surfaces:#?}"
        );
    }
}

#[test]
fn javascript_surface_phrases_ignore_ui_looking_module_specifiers() {
    let text = r#"import widget from '@app/aria-label-open-settings-panel';
export { widget as openSettingsPanel } from '@app/route-orders-new';
pub(crate) const lazy = import ('@app/data-testid-submit-order-button');
pub(crate) const required = require ('@app/class-name-submit-order-button');
const bareLazy = import ('aria-label-open-settings-panel');
const bareRequired = require ('data-testid-submit-order-button');
const commentedLazy = import(/* webpackChunkName: "settings" */ 'aria-label-open-settings-panel');
import {
  multi,
} from '@app/aria-label-open-settings-panel';
const bareSubpath = require ('@scope/data-testid-submit-order-button');
"#;

    let surfaces = extract_surfaces(text, "tsx");

    assert!(surfaces.phrases.is_empty(), "{surfaces:#?}");
    assert!(surfaces.tokens.is_empty(), "{surfaces:#?}");
}

#[test]
fn javascript_surface_phrases_ignore_multiline_comments() {
    let text = r#"/*
  <button aria-label="Open settings panel" data-testid="submit-order-button">Settings</button>
  await page.goto("/orders/new");
*/
export function CommentOnly() {
  return <div />;
}
"#;

    let surfaces = extract_surfaces(text, "tsx");

    assert!(surfaces.phrases.is_empty(), "{surfaces:#?}");
    assert!(surfaces.tokens.is_empty(), "{surfaces:#?}");
}

#[test]
fn rust_symbols_keep_visibility_and_ranges() {
    let text = r#"use crate::timeline::frame_at;

pub struct Session {
    frame: u64,
}

impl Session {
    pub fn seek_frame(&self, time_ms: u64) -> u64 {
        frame_at(time_ms)
    }
}

fn internal_tick() {}
"#;

    let symbols = extract_symbols(text, "rs");

    assert_symbol(&symbols, "Session", "struct", true, 3, 5);
    assert_symbol(&symbols, "Session", "impl", false, 7, 11);
    assert_symbol(&symbols, "seek_frame", "function", true, 8, 10);
    assert_symbol(&symbols, "internal_tick", "function", false, 13, 13);
}

#[test]
fn python_symbols_keep_functions_and_classes_without_export_claims() {
    let text = r#"from .timeline import frame_at


class ReplaySession:
    pass


def seek(frames: list[int], frame: int) -> int:
    return frame_at(frames, frame)


async def refresh() -> None:
    return None
"#;

    let symbols = extract_symbols(text, "py");

    assert_symbol(&symbols, "ReplaySession", "class", false, 4, 5);
    assert_symbol(&symbols, "seek", "function", false, 8, 9);
    assert_symbol(&symbols, "refresh", "function", false, 12, 13);
}

#[test]
fn go_symbols_keep_exports_functions_methods_and_types() {
    let text = r#"package session

type Frame struct {
    Index int
}

func Seek(frames []Frame, frame int) Frame {
    return frames[frame]
}

func (s Session) tick() {}
"#;

    let symbols = extract_symbols(text, "go");

    assert_symbol(&symbols, "Frame", "struct", true, 3, 5);
    assert_symbol(&symbols, "Seek", "function", true, 7, 9);
    assert_symbol(&symbols, "tick", "method", false, 11, 11);
}

#[test]
fn swift_symbols_keep_modifiers_imports_and_ranges() {
    let text = r#"import Foundation
import SwiftUI

@MainActor
public final class ReplayViewModel: ObservableObject {
    @Published public var selectedID: String?

    public struct NavigationFrame {
        let label: String
    }

    public var title: String {
        "Replay"
    }

    private let frames: [NavigationFrame] = []

    public func seekFrame(_ index: Int) -> NavigationFrame? {
        frames.indices.contains(index) ? frames[index] : nil
    }
}

private enum ReplayMode {
    case paused
}
"#;

    let symbols = extract_symbols(text, "swift");

    assert_symbol(&symbols, "ReplayViewModel", "class", true, 5, 21);
    assert_symbol(&symbols, "selectedID", "property", true, 6, 6);
    assert_symbol(&symbols, "NavigationFrame", "struct", true, 8, 10);
    assert_symbol(&symbols, "title", "property", true, 12, 14);
    assert_symbol(&symbols, "frames", "constant", false, 16, 16);
    assert_symbol(&symbols, "seekFrame", "function", true, 18, 20);
    assert_symbol(&symbols, "ReplayMode", "enum", false, 23, 25);

    let imports = swift_import_re()
        .captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();
    assert_eq!(imports, vec!["Foundation", "SwiftUI"]);

    let qualified_imports = swift_import_re()
            .captures_iter(
                "@testable import SwiftFixture\n@_spi(Internal) import Core\nimport struct Foundation.UUID\n",
            )
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect::<Vec<_>>();
    assert_eq!(
        qualified_imports,
        vec!["SwiftFixture", "Core", "Foundation"]
    );
}

#[test]
fn fixture_projects_populate_symbols_for_primary_languages() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let cases = [
        (
            "mixed-monorepo",
            "domains/replay/src/replay-session.ts",
            "seekFrame",
        ),
        (
            "rust-workspace",
            "crates/replay/src/session.rs",
            "seek_frame",
        ),
        (
            "python-workspace",
            "services/replay/replay/session.py",
            "seek",
        ),
        ("go-workspace", "services/replay/session/session.go", "Seek"),
        (
            "swift-package",
            "Sources/SwiftFixture/ViewModel.swift",
            "ReplayViewModel",
        ),
    ];

    for (fixture, rel, symbol) in cases {
        let project = load_project_with_cache(
            RootSelection::Exact(root.join(fixture)),
            CacheWriteMode::ReadOnly,
        )
        .expect("load fixture project");
        let file = project.files.get(rel).unwrap_or_else(|| {
            panic!(
                "expected file `{rel}` in fixture `{fixture}`; available: {:#?}",
                project.files.keys().collect::<Vec<_>>()
            )
        });
        assert!(
            file.symbols.iter().any(|item| item.name == symbol),
            "expected `{symbol}` in `{fixture}/{rel}` symbols: {:#?}",
            file.symbols
        );
        assert!(file.line_count > 0, "line_count should be populated");
    }
}

pub(crate) fn assert_symbol(
    symbols: &[SymbolInfo],
    name: &str,
    kind: &str,
    exported: bool,
    line_start: usize,
    line_end: usize,
) {
    let symbol = symbols
        .iter()
        .find(|item| item.name == name && item.kind == kind && item.line_start == line_start)
        .unwrap_or_else(|| {
            panic!("missing symbol `{name}` kind `{kind}` line `{line_start}` in {symbols:#?}")
        });
    assert_eq!(symbol.exported, exported, "{name} exported mismatch");
    assert_eq!(symbol.line_end, line_end, "{name} line_end mismatch");
}

pub(crate) fn write_test_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write test file");
}
