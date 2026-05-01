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

