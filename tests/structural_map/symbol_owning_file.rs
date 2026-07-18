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
    write(
        &repo.path().join("packages/app/src/runtime/paths.ts"),
        "export function runtimeRoutePathAnalysis() {\n  return 'runtime-path';\n}\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/tests/runtime-transform-paths.test.ts"),
        "test('runtime transform paths preserve route analysis', () => {\n  expect('runtime path').toContain('path');\n});\n",
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

    let runtime_cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/runtime/paths.ts#runtimeRoutePathAnalysis",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &runtime_cone);
    assert!(
        runtime_cone["proof"]
            .as_array()
            .expect("runtime cone proof")
            .iter()
            .any(|edge| {
                edge["from"] == "packages/app/tests/runtime-transform-paths.test.ts"
                    && edge["evidence"] == "test_surface_tokens_owning_file"
                    && edge["strength"] == "medium"
            }),
        "exact symbol cone should retain one soft owning-file behavioral surface: {runtime_cone:#}"
    );
    let runtime_proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/runtime/paths.ts#runtimeRoutePathAnalysis",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &runtime_proof);
    assert!(
        runtime_proof["proofs"]
            .as_array()
            .expect("runtime proofs")
            .iter()
            .all(|surface| surface["path"] != "packages/app/tests/runtime-transform-paths.test.ts"),
        "a soft owning-file cone hint must not become exact symbol proof: {runtime_proof:#}"
    );
    assert!(
        runtime_proof["unknowns"]
            .as_array()
            .expect("runtime unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "direct_test_import_not_found"),
        "the missing exact proof boundary must remain explicit: {runtime_proof:#}"
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
