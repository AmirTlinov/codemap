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
fn proof_map_empty_exact_scope_points_to_nearest_parent_proof_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/client/cache.ts"),
        "export const localCache = new Map<string, string>();\n",
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "packages/app/src/client",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["direct"].as_array().expect("direct").is_empty()
            && proof_map["e2e"].as_array().expect("e2e").is_empty()
            && proof_map["contract"].as_array().expect("contract").is_empty(),
        "narrow proof-map scope should start without deterministic sensors: {proof_map:#}"
    );
    assert!(
        proof_map["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "nearest_proof_scope"
                && unknown["path"] == "packages/app/src/client"
                && unknown["expand"] == "codemap proof packages/app/src"),
        "empty exact proof-map scope should expose nearest parent proof scope: {proof_map:#}"
    );
    assert!(
        proof_map["expand"]
            .as_array()
            .expect("expand")
            .iter()
            .any(|expand| expand == "codemap proof-map packages/app/src"),
        "proof-map expand should include the broader parent proof-map: {proof_map:#}"
    );
}

#[test]
fn changed_empty_direct_proof_anchor_points_to_nearest_parent_proof_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/client/cache.ts"),
        "export const localCache = new Map<string, string>();\n",
    );

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert!(
        changed["changed"]
            .as_array()
            .expect("changed anchors")
            .iter()
            .any(|file| file["path"] == "packages/app/src/client/cache.ts"),
        "changed should include the touched narrow anchor: {changed:#}"
    );
    assert!(
        changed["unknowns"]
            .as_array()
            .expect("unknowns")
            .iter()
            .any(|unknown| unknown["kind"] == "nearest_proof_scope"
                && unknown["path"] == "packages/app/src/client/cache.ts"
                && unknown["expand"] == "codemap proof packages/app/src"),
        "changed should inherit nearest parent proof scope from proof-map facts: {changed:#}"
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
