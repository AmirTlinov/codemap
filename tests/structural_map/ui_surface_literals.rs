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

