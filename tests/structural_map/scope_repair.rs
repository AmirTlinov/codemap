#[test]
fn proof_empty_exact_scope_points_to_nearest_parent_proof_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/client/cache.ts"),
        "export const localCache = new Map<string, string>();\n",
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/client/cache.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"].as_array().expect("proofs").is_empty(),
        "fixture anchor should start without direct proof sensors: {proof:#}"
    );
    assert!(
        proof["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "nearest_proof_scope"
                && unknown["path"] == "packages/app/src/client/cache.ts"
                && unknown["expand"] == "codemap proof packages/app/src"),
        "empty exact proof scope should expose nearest parent proof scope: {proof:#}"
    );
    assert!(
        proof["expand"]
            .as_array()
            .expect("expand")
            .iter()
            .any(|expand| expand == "codemap proof packages/app/src"),
        "proof expand should be runnable for the nearest parent scope: {proof:#}"
    );
}

#[test]
fn place_empty_test_scope_points_to_nearest_parent_proof_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/client/cache.ts"),
        "export const localCache = new Map<string, string>();\n",
    );

    let place = run_json(
        repo.path(),
        cache.path(),
        &[
            "place",
            "packages/app/src/client",
            "--kind",
            "test",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/place.schema.json", &place);
    assert!(
        place["existing_surfaces"]
            .as_array()
            .expect("existing surfaces")
            .is_empty(),
        "narrow implementation scope should have no local test placement surface: {place:#}"
    );
    assert!(
        place["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "nearest_proof_scope"
                && unknown["path"] == "packages/app/src/client"
                && unknown["expand"] == "codemap place packages/app/src --kind test"),
        "empty place test scope should expose nearest parent proof scope: {place:#}"
    );
}

#[test]
fn scope_repair_does_not_turn_missing_paths_into_parent_hints() {
    let (repo, cache) = fixture();

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/app/src/does-not-exist.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["unknowns"].as_array().expect("proof unknowns").is_empty(),
        "missing proof target should not be disguised as a narrow empty scope: {proof:#}"
    );

    let place = run_json(
        repo.path(),
        cache.path(),
        &[
            "place",
            "packages/app/src/nope",
            "--kind",
            "test",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/place.schema.json", &place);
    assert!(
        place["unknowns"].as_array().expect("place unknowns").is_empty(),
        "missing place scope should not be disguised as a narrow empty scope: {place:#}"
    );
}
