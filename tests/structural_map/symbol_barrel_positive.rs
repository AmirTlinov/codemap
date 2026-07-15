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
    let incoming_horizon = horizon(&cone["observations"], "incoming");
    assert!(
        incoming_horizon["count"]["observed"]
            .as_u64()
            .expect("observed incoming")
            >= 1,
        "resolved consumers must remain an observed lower bound: {cone:#}"
    );
    assert_eq!(
        incoming_horizon["count"]["closure"], "open",
        "a resolved consumer must not hide the remaining re-export gap: {cone:#}"
    );
    assert!(
        incoming_horizon["count"]["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|reason| reason == "reexport_flow"),
        "positive lower bound must retain its typed re-export gap: {cone:#}"
    );
    assert_horizon_certificate_resolves(&cone["observations"], incoming_horizon);
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
