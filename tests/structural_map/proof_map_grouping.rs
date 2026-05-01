#[test]
fn proof_map_groups_duplicate_direct_sensors_in_directory_scope() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/dual-a.ts"),
        "export function dualA() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/dual-b.ts"),
        "export function dualB() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/dual.test.ts"),
        "import { dualA } from '../src/dual-a';\nimport { dualB } from '../src/dual-b';\n\ntest('dual source proof', () => {\n  expect(dualA() && dualB()).toBe(true);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "duplicate proof-map sensors"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let duplicate_test_surfaces = proof_map["direct"]
        .as_array()
        .expect("direct proof surfaces")
        .iter()
        .filter(|proof| {
            proof["path"] == "packages/app/tests/dual.test.ts"
                && proof["evidence"] == "test_import"
        })
        .count();
    assert_eq!(
        duplicate_test_surfaces, 1,
        "directory proof-map should group identical proof sensors instead of repeating one test per seed: {proof_map:#}"
    );
    assert!(
        proof_map["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"].as_str().is_some_and(|reason| {
                reason == "duplicate direct proof sensors grouped by structural key"
            }) && hidden["expand"]
                .as_str()
                .is_some_and(|expand| expand.contains("--raw-sensors"))),
        "grouped duplicates should stay visible as hidden count, not disappear silently: {proof_map:#}"
    );
}

#[test]
fn proof_map_raw_sensors_reveals_grouped_duplicates() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/dual-a.ts"),
        "export function dualA() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/dual-b.ts"),
        "export function dualB() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/dual.test.ts"),
        "import { dualA } from '../src/dual-a';\nimport { dualB } from '../src/dual-b';\n\ntest('dual source proof', () => {\n  expect(dualA() && dualB()).toBe(true);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof-map raw sensors fixture"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "packages/app/src",
            "--raw-sensors",
            "--limit",
            "50",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let duplicate_test_surfaces = proof_map["direct"]
        .as_array()
        .expect("direct proof surfaces")
        .iter()
        .filter(|proof| {
            proof["path"] == "packages/app/tests/dual.test.ts"
                && proof["evidence"] == "test_import"
        })
        .count();
    assert_eq!(
        duplicate_test_surfaces, 2,
        "raw proof-map sensors should reveal the repeated per-seed proof facts: {proof_map:#}"
    );
    assert!(
        proof_map["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .all(|hidden| hidden["reason"]
                .as_str()
                .is_none_or(|reason| !reason.starts_with("duplicate "))),
        "raw-sensors mode should not claim duplicate grouping is still hidden: {proof_map:#}"
    );
}

#[test]
fn proof_map_hidden_expands_use_supported_flags() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/dual-a.ts"),
        "export function dualA() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/src/dual-b.ts"),
        "export function dualB() { return true; }\n",
    );
    write(
        &repo.path().join("packages/app/tests/dual.test.ts"),
        "import { dualA } from '../src/dual-a';\nimport { dualB } from '../src/dual-b';\n\ntest('dual source proof', () => {\n  expect(dualA() && dualB()).toBe(true);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof-map hidden expand fixture"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "packages/app/src",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let hidden = proof_map["hidden"].as_array().expect("hidden");
    assert!(!hidden.is_empty(), "fixture should create hidden groups: {proof_map:#}");
    assert!(
        hidden.iter().all(|group| group["expand"]
            .as_str()
            .is_some_and(|expand| expand.contains("--limit")
                && !expand.contains("--include-hidden"))),
        "proof-map hidden expand commands must be runnable by the current CLI: {proof_map:#}"
    );
}
