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
    let target_anchors = siblings["proof_pattern"]
        .as_array()
        .expect("proof pattern")
        .iter()
        .filter(|proof| {
            proof["path"] == "packages/app/tests/dual.test.ts"
                && proof["evidence"] == "test_import"
        })
        .filter_map(|proof| proof["target_anchor"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        target_anchors,
        vec![
            "packages/app/src/dual-a.ts",
            "packages/app/src/dual-b.ts"
        ],
        "siblings should keep distinct proof sensor targets from the same test file: {siblings:#}"
    );
    assert!(
        siblings["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .all(|group| group["reason"]
                != "duplicate proof pattern sensors grouped by structural key"),
        "distinct proof sensor targets should not be hidden as duplicate siblings proof sensors: {siblings:#}"
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
    let target_anchors = place["paired_proof_pattern"]
        .as_array()
        .expect("paired proof pattern")
        .iter()
        .filter(|proof| {
            proof["path"] == "packages/app/tests/dual.test.ts"
                && proof["evidence"] == "test_import"
        })
        .filter_map(|proof| proof["target_anchor"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        target_anchors,
        vec![
            "packages/app/src/dual-a.ts",
            "packages/app/src/dual-b.ts"
        ],
        "place should keep distinct paired proof sensor targets from the same test file: {place:#}"
    );
    assert!(
        place["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .all(|group| group["reason"]
                != "duplicate paired proof sensors grouped by structural key"),
        "distinct paired proof sensor targets should not be hidden as duplicate place proof sensors: {place:#}"
    );
}

#[test]
fn place_kind_test_uses_external_proof_sensors_for_scope() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "place-test-proof",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/features/studio/overlay.ts"),
        "export function overlay() { return true; }\n",
    );
    write(
        &repo.path().join("src/features/other/helper.ts"),
        "export function helper() { return true; }\n",
    );
    write(
        &repo.path().join("tests/overlay.test.ts"),
        "import { overlay } from '../src/features/studio/overlay';\n\ntest('overlay', () => {\n  expect(overlay()).toBe(true);\n});\n",
    );
    write(
        &repo.path().join("tests/helper.test.ts"),
        "import { helper } from '../src/features/other/helper';\n\ntest('helper', () => {\n  expect(helper()).toBe(true);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "place external test proof fixture"],
    );

    let place = run_json(
        repo.path(),
        cache.path(),
        &[
            "place",
            "src/features/studio",
            "--kind",
            "test",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/place.schema.json", &place);
    let examples = place["existing_surfaces"][0]["examples"]
        .as_array()
        .expect("test placement examples")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    assert!(
        examples.contains(&"tests/overlay.test.ts"),
        "place --kind test should expose external tests proven by imports into the scope: {place:#}"
    );
    assert!(
        !examples.contains(&"tests/helper.test.ts"),
        "place --kind test should not pull unrelated external tests into the scope: {place:#}"
    );
    assert_eq!(
        place["existing_surfaces"][0]["evidence"], "proof_sensor_for_scope",
        "external test placement examples should be evidence-labelled as proof sensors, not same-scope files: {place:#}"
    );
    assert!(
        place["local_conventions"]
            .as_array()
            .expect("local conventions")
            .iter()
            .filter_map(|value| value.as_str())
            .any(|line| line.contains("proof sensors already reference")),
        "place --kind test should not claim external tests live under the implementation scope: {place:#}"
    );
    assert!(
        place["paired_proof_pattern"]
            .as_array()
            .expect("paired proof pattern")
            .iter()
            .any(|proof| proof["path"] == "tests/overlay.test.ts"
                && proof["evidence"] == "test_import"),
        "place --kind test should show the paired proof sensor itself: {place:#}"
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
