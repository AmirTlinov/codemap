fn barrel_negative_fixture() -> (TempDir, TempDir) {
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

    (repo, cache)
}
