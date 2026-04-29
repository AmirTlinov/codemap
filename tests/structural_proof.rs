use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn ctx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ctx"))
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {:?} failed", args);
}

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write file");
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "a@example.com"]);
    git(dir, &["config", "user.name", "a"]);
}

fn assert_schema_accepts(schema_rel: &str, instance: &Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(root.join(schema_rel)).expect("schema should exist");
    let schema: Value = serde_json::from_str(&text).expect("schema should be valid json");
    let validator = jsonschema::validator_for(&schema).expect("schema should compile");
    validator
        .validate(instance)
        .unwrap_or_else(|error| panic!("{schema_rel} rejected instance: {error}"));
}

fn proof_repo() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "proof-root",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": { "test": "echo root-test" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/package.json"),
        r#"{
  "name": "@fixture/replay",
  "version": "1.0.0",
  "scripts": { "test": "echo replay-test" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/src/types.ts"),
        "export interface FrameDto {\n  frame: number;\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: cursor };\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/helper.ts"),
        "export const helper = 1;\n",
    );
    write(
        &repo.path().join("packages/replay/tests/session.test.ts"),
        "import { seek } from '../src/session';\n\ntest('seek maps frame', () => {\n  expect(seek(2).frame).toBe(2);\n});\n",
    );
    write(
        &repo.path().join("packages/replay/tests/token.test.ts"),
        "test('unrelated token behavior', () => {\n  expect('token').toBe('token');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    (repo, cache)
}

#[test]
fn proof_exact_path_uses_structural_test_import_and_package_local_command() {
    let (repo, cache) = proof_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "proof",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx proof should run");
    assert!(
        output.status.success(),
        "ctx proof failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/proof.schema.json", &json);
    assert_eq!(json["kind"], "proof_plan");
    assert_eq!(json["schema_version"], "2");
    assert_eq!(json["target"], "packages/replay/src/session.ts");
    assert!(json["changed"].as_array().unwrap().is_empty());
    assert!(
        json["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(
                |proof| proof["path"] == "packages/replay/tests/session.test.ts"
                    && proof["evidence"] == "test_import"
                    && proof["strength"] == "high"
                    && proof["command"]
                        == "cd packages/replay && npm test -- tests/session.test.ts"
            )
    );
    assert_eq!(json.get("read_first"), None);
    assert_eq!(json.get("confidence"), None);
    assert!(
        !serde_json::to_string(&json)
            .expect("json string")
            .contains("source_of_truth")
    );
}

#[test]
fn proof_changed_uses_impact_consumers_to_find_specific_tests() {
    let (repo, cache) = proof_repo();
    write(
        &repo.path().join("packages/replay/src/types.ts"),
        "export interface FrameDto {\n  frame: number;\n  label?: string;\n}\n",
    );
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["proof", "--changed", "--format", "json"])
        .output()
        .expect("ctx proof changed should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/proof.schema.json", &json);
    assert_eq!(json["target"], Value::Null);
    assert_eq!(json["changed"][0], "packages/replay/src/types.ts");
    assert_eq!(json["risk"], "high");
    assert!(
        json["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(
                |proof| proof["path"] == "packages/replay/tests/session.test.ts"
                    && proof["reason"]
                        .as_str()
                        .unwrap_or_default()
                        .contains("impact cluster")
            )
    );
}

#[test]
fn proof_does_not_promote_same_domain_tests_without_import_or_name_evidence() {
    let (repo, cache) = proof_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["proof", "packages/replay/src/helper.ts", "--format", "json"])
        .output()
        .expect("ctx proof should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/proof.schema.json", &json);
    assert!(
        json["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|proof| proof["path"] != "packages/replay/tests/token.test.ts"),
        "same-domain tests without structural evidence must stay out of proof"
    );
}

#[test]
fn proof_run_refuses_placeholder_only_plan() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(&repo.path().join("notes.txt"), "before\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);
    write(&repo.path().join("notes.txt"), "after\n");

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["proof", "--changed", "--run"])
        .output()
        .expect("ctx proof --run should run");
    assert!(
        !output.status.success(),
        "proof --run should fail closed when only a placeholder fallback exists"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(stderr.contains("cannot run placeholder verification"));
}

#[test]
fn proof_changed_empty_diff_does_not_infer_project_wide_fallback() {
    let (repo, cache) = proof_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["proof", "--changed", "--format", "json"])
        .output()
        .expect("ctx proof changed should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/proof.schema.json", &json);
    assert!(json["changed"].as_array().unwrap().is_empty());
    assert!(json["proofs"].as_array().unwrap().is_empty());
    assert!(json["fallback"].as_array().unwrap().is_empty());
}

#[test]
fn proof_markdown_is_not_legacy_verify_or_route_language() {
    let (repo, cache) = proof_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["proof", "packages/replay/src/session.ts"])
        .output()
        .expect("ctx proof markdown should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("# Proof Plan"));
    assert!(stdout.contains("## Proofs"));
    assert!(stdout.contains("test_import"));
    assert!(!stdout.contains("Read first"));
    assert!(!stdout.contains("Confidence"));
    assert!(!stdout.contains("source_of_truth"));
}
