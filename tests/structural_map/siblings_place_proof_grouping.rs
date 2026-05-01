#[test]
fn siblings_groups_duplicate_proof_pattern_sensors() {
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
    git(repo.path(), &["commit", "-qm", "siblings proof grouping fixture"]);

    let siblings = run_json(
        repo.path(),
        cache.path(),
        &[
            "siblings",
            "packages/app/src",
            "--limit",
            "50",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/siblings.schema.json", &siblings);
    let duplicate_count = siblings["proof_pattern"]
        .as_array()
        .expect("proof pattern")
        .iter()
        .filter(|proof| {
            proof["path"] == "packages/app/tests/dual.test.ts"
                && proof["evidence"] == "test_import"
        })
        .count();
    assert_eq!(
        duplicate_count, 1,
        "siblings should group repeated proof sensors from the same test file: {siblings:#}"
    );
    assert!(
        siblings["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"]
                == "duplicate proof pattern sensors grouped by structural key"
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.starts_with("codemap proof-map packages/app/src --raw-sensors --limit ")
                        && !expand.contains("<larger-number>")
                })),
        "grouped siblings proof sensors should expose raw proof-map zoom: {siblings:#}"
    );
}

#[test]
fn place_groups_duplicate_paired_proof_sensors() {
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
    git(repo.path(), &["commit", "-qm", "place proof grouping fixture"]);

    let place = run_json(
        repo.path(),
        cache.path(),
        &[
            "place",
            "packages/app/src",
            "--kind",
            "source",
            "--limit",
            "50",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/place.schema.json", &place);
    let duplicate_count = place["paired_proof_pattern"]
        .as_array()
        .expect("paired proof pattern")
        .iter()
        .filter(|proof| {
            proof["path"] == "packages/app/tests/dual.test.ts"
                && proof["evidence"] == "test_import"
        })
        .count();
    assert_eq!(
        duplicate_count, 1,
        "place should group repeated paired proof sensors from the same test file: {place:#}"
    );
    assert!(
        place["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|group| group["reason"]
                == "duplicate paired proof sensors grouped by structural key"
                && group["expand"].as_str().is_some_and(|expand| {
                    expand.starts_with("codemap proof-map packages/app/src --raw-sensors --limit ")
                        && !expand.contains("<larger-number>")
                })),
        "grouped place proof sensors should expose raw proof-map zoom: {place:#}"
    );
}

#[test]
fn place_paired_proof_pattern_uses_requested_kind_files() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "place-kind-proof",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/lenses/runtime.ts"),
        "export function runtimeLens() { return true; }\n",
    );
    write(
        &repo.path().join("src/helper.ts"),
        "export function helper() { return true; }\n",
    );
    write(
        &repo.path().join("tests/runtime.test.ts"),
        "import { runtimeLens } from '../src/lenses/runtime';\n\ntest('runtime lens', () => {\n  expect(runtimeLens()).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("tests/helper.test.ts"),
        "import { helper } from '../src/helper';\n\ntest('helper', () => {\n  expect(helper()).toBe(true);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "place kind proof fixture"],
    );

    let place = run_json(
        repo.path(),
        cache.path(),
        &[
            "place",
            "src",
            "--kind",
            "lens",
            "--limit",
            "20",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/place.schema.json", &place);
    let proof_paths = place["paired_proof_pattern"]
        .as_array()
        .expect("paired proof pattern")
        .iter()
        .filter_map(|proof| proof["path"].as_str())
        .collect::<Vec<_>>();
    assert!(
        proof_paths.contains(&"tests/runtime.test.ts"),
        "lens place proof should include proof for matched lens files: {place:#}"
    );
    assert!(
        !proof_paths.contains(&"tests/helper.test.ts"),
        "lens place proof should not include proof for non-lens files in the same scope: {place:#}"
    );
}
