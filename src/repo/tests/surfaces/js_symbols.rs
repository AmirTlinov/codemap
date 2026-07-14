// Responsibility: repo-tests-js-symbols
use crate::repo::extract_symbols;
use crate::repo::tests::playwright::assert_symbol;

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
