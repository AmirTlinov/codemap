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

#[test]
fn proof_map_scope_expands_to_matching_proof_scope() {
    let (repo, cache) = fixture();

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let expand = proof_map["expand"].as_array().expect("expand");
    assert!(
        expand
            .iter()
            .any(|command| command == "codemap proof packages/app/src"),
        "proof-map scope expand should keep the same explicit scope: {proof_map:#}"
    );
    assert!(
        expand.iter().all(|command| command != "codemap proof --changed"),
        "explicit proof-map scope must not point at changed files: {proof_map:#}"
    );
}

#[test]
fn proof_map_files_expands_to_matching_proof_files() {
    let (repo, cache) = fixture();

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            "--files",
            "packages/app/src/useReplay.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let expand = proof_map["expand"].as_array().expect("expand");
    assert!(
        expand
            .iter()
            .any(|command| command == "codemap proof --files packages/app/src/useReplay.ts"),
        "proof-map --files expand should keep the same explicit file selector: {proof_map:#}"
    );
    assert!(
        expand.iter().all(|command| command != "codemap proof --changed"),
        "explicit proof-map files must not point at changed files: {proof_map:#}"
    );
}

#[test]
fn proof_map_explicit_root_stays_current_level_until_raw_sensors() {
    let (repo, cache) = fixture();
    write(&repo.path().join("AGENTS.md"), "# Root Bootstrap\n");
    write(&repo.path().join("README.md"), "# Fixture\n");
    write(&repo.path().join("tests/AGENTS.md"), "# Test Bootstrap\n");
    write(&repo.path().join("tests/README.md"), "# Test Notes\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "proof map markdown noise fixture"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", ".", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"] == "recursive proof seeds hidden at root scope"
                && hidden["expand"]
                    .as_str()
                    .is_some_and(|expand| expand.starts_with("codemap proof-map . --raw-sensors --limit "))),
        "explicit root proof-map should make recursive proof seeds opt-in instead of scanning the whole repo by default: {proof_map:#}"
    );
    for section in ["direct", "indirect", "e2e", "contract"] {
        assert!(
            proof_map[section]
                .as_array()
                .expect("proof section")
                .iter()
                .filter_map(|proof| proof["path"].as_str())
                .all(|path| !path.starts_with("packages/")),
            "root proof-map should not surface nested package proof sensors before raw-sensors in {section}: {proof_map:#}"
        );
        assert!(
            proof_map[section]
                .as_array()
                .expect("proof section")
                .iter()
                .filter_map(|proof| proof["path"].as_str())
                .all(|path| !path.ends_with("AGENTS.md") && !path.ends_with(".md")),
            "root proof-map must not treat Markdown bootstrap/docs inside tests directories as executable proof sensors in {section}: {proof_map:#}"
        );
    }
    assert!(
        proof_map["missing_direct"]
            .as_array()
            .expect("missing direct")
            .iter()
            .all(|surface| surface["path"] != "package.json"),
        "root proof-map should not complain that the top-level manifest lacks direct proof; exact or changed manifest scopes own that question: {proof_map:#}"
    );

    let raw = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof-map",
            ".",
            "--raw-sensors",
            "--limit",
            "200",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof-map.schema.json", &raw);
    assert!(
        raw["hidden"]
            .as_array()
            .expect("raw hidden")
            .iter()
            .all(|hidden| hidden["reason"] != "recursive proof seeds hidden at root scope"),
        "raw-sensors is the explicit deeper proof-map expansion and should not keep the root seed gate: {raw:#}"
    );
}

#[test]
fn proof_map_root_shows_current_level_test_containers_not_recursive_files() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("tests/root-smoke.test.ts"),
        "test('root smoke', () => {\n  expect(true).toBe(true);\n});\n",
    );
    write(&repo.path().join("tests/README.md"), "# test notes\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "root proof container"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", ".", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    assert!(
        proof_map["direct"]
            .as_array()
            .expect("direct proof")
            .iter()
            .any(|proof| proof["path"] == "tests/"
                && proof["evidence"] == "current_level_proof_container"
                && proof["command"].is_null()
                && proof["reason"]
                    .as_str()
                    .is_some_and(|reason| reason.contains("test container"))),
        "root proof-map should show current-level test containers without listing every test file: {proof_map:#}"
    );
    assert!(
        proof_map["direct"]
            .as_array()
            .expect("direct proof")
            .iter()
            .filter_map(|proof| proof["path"].as_str())
            .all(|path| path != "tests/root-smoke.test.ts" && !path.ends_with(".md")),
        "root proof-map should keep recursive test files and markdown notes hidden by default: {proof_map:#}"
    );
    assert!(
        proof_map["commands"]
            .as_array()
            .expect("commands")
            .iter()
            .all(|proof| !(proof["path"] == "tests/"
                && proof["evidence"] == "current_level_proof_container")),
        "current-level proof containers must not invent one nested test command for the whole container: {proof_map:#}"
    );
    assert!(
        proof_map["e2e"]
            .as_array()
            .expect("e2e proof")
            .iter()
            .filter_map(|proof| proof["path"].as_str())
            .all(|path| path != "packages/"),
        "root proof-map must not collapse nested package tests into a fake packages/ proof container: {proof_map:#}"
    );
}

#[test]
fn proof_map_default_matches_changed_selector() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { seek } from '@fixture/replay';\n\nexport const frame = seek(4).frame;\n",
    );

    let default_report = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "--format", "json"],
    );
    let changed_report = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "--changed", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &default_report);
    assert_eq!(
        default_report["changed"], changed_report["changed"],
        "bare proof-map should inspect the same changed files as --changed"
    );
    let expand = default_report["expand"].as_array().expect("expand");
    assert_eq!(
        expand,
        changed_report["expand"].as_array().expect("changed expand"),
        "bare proof-map expand should match explicit --changed"
    );
    assert!(
        expand.iter().all(|command| command == "codemap proof --changed"),
        "bare proof-map must not double-prefix or degrade changed selector: {default_report:#}"
    );
}

#[test]
fn proof_map_staged_expands_to_matching_proof_staged_selector() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { seek } from '@fixture/replay';\n\nexport const frame = seek(2).frame;\n",
    );
    git(repo.path(), &["add", "packages/app/src/useReplay.ts"]);

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "--staged", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let expand = proof_map["expand"].as_array().expect("expand");
    assert!(
        expand.iter().any(|command| command == "codemap proof --staged"),
        "proof-map --staged expand should preserve the staged selector: {proof_map:#}"
    );
    assert!(
        expand.iter().all(|command| command != "codemap proof --changed"
            && !command.as_str().unwrap_or_default().starts_with("codemap proof --files")),
        "proof-map --staged must not degrade into changed/files selectors: {proof_map:#}"
    );
}

#[test]
fn proof_map_since_expands_to_matching_proof_since_selector() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor + 1) };\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/public-only.ts"),
        "export function publicOnly() {\n  return false;\n}\n",
    );

    let proof_map = run_json(
        repo.path(),
        cache.path(),
        &["proof-map", "--since", "HEAD", "--limit", "1", "--format", "json"],
    );
    assert_schema("schemas/proof-map.schema.json", &proof_map);
    let expand = proof_map["expand"].as_array().expect("expand");
    assert!(
        expand
            .iter()
            .any(|command| command == "codemap proof --since HEAD"),
        "proof-map --since expand should preserve the since selector: {proof_map:#}"
    );
    assert!(
        expand.iter().all(|command| command != "codemap proof --changed"
            && !command.as_str().unwrap_or_default().starts_with("codemap proof --files")),
        "proof-map --since must not degrade into changed/files selectors: {proof_map:#}"
    );
    let hidden = proof_map["hidden"].as_array().expect("hidden");
    assert!(
        !hidden.is_empty(),
        "fixture should force at least one hidden proof-map group: {proof_map:#}"
    );
    assert!(
        hidden.iter().all(|group| group["expand"].as_str().is_some_and(|command| {
            command.starts_with("codemap proof-map --since HEAD")
                && !command.starts_with("codemap proof-map --files")
        })),
        "proof-map hidden expands should preserve the since selector too: {proof_map:#}"
    );
}
