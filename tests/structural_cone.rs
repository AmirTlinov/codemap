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

fn cone_repo() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "cone-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1

boundaries:
  forbidden:
    - from: src/session.ts
      to: src/types.ts
      reason: session must not import DTO contract directly in this fixture
"#,
    );
    write(
        &repo.path().join("src/clock.ts"),
        "export function frameClock(cursor: number) {\n  return cursor;\n}\n",
    );
    write(
        &repo.path().join("src/timeline.ts"),
        "import { frameClock } from './clock';\n\nexport class Timeline {\n  frameAt(cursor: number) {\n    return frameClock(cursor);\n  }\n}\n",
    );
    write(
        &repo.path().join("src/types.ts"),
        "export interface FrameDto {\n  frame: number;\n}\n",
    );
    write(
        &repo.path().join("src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor) };\n}\n",
    );
    write(
        &repo.path().join("src/consumer.ts"),
        "import { seek } from './session';\n\nexport const value = seek(1);\n",
    );
    write(
        &repo.path().join("src/session.test.ts"),
        "import { seek } from './session';\n\ntest('seek maps cursor', () => {\n  expect(seek(4).frame).toBe(4);\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    (repo, cache)
}

#[test]
fn structural_cone_file_reports_edges_proof_contracts_and_boundary() {
    let (repo, cache) = cone_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["cone", "src/session.ts", "--format", "json"])
        .output()
        .expect("ctx cone should run");
    assert!(
        output.status.success(),
        "ctx cone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/cone.schema.json", &json);
    assert_eq!(json["kind"], "cone_report");
    assert_eq!(json["schema_version"], "2");
    assert_eq!(json["anchor"]["path"], "src/session.ts");
    assert_eq!(json["anchor"]["package"], "cone-fixture");
    assert_eq!(json["depth"], 1);
    assert!(
        json["outgoing"]
            .as_array()
            .expect("outgoing array")
            .iter()
            .any(|edge| edge["from"] == "src/session.ts"
                && edge["to"] == "src/timeline.ts"
                && edge["type"] == "imports"
                && edge["strength"] == "high")
    );
    assert!(
        json["incoming"]
            .as_array()
            .expect("incoming array")
            .iter()
            .any(|edge| edge["from"] == "src/consumer.ts"
                && edge["to"] == "src/session.ts"
                && edge["type"] == "imported_by")
    );
    assert!(
        json["proof"]
            .as_array()
            .expect("proof array")
            .iter()
            .any(|edge| edge["from"] == "src/session.test.ts"
                && edge["to"] == "src/session.ts"
                && edge["type"] == "tests")
    );
    assert!(
        json["contracts"]
            .as_array()
            .expect("contracts array")
            .iter()
            .any(|edge| edge["from"] == "src/session.ts"
                && edge["to"] == "src/types.ts"
                && edge["type"] == "contract"
                && edge["evidence"] == "role:schema_contract")
    );
    assert!(
        json["boundary"]
            .as_array()
            .expect("boundary array")
            .iter()
            .any(|edge| edge["from"] == "src/session.ts"
                && edge["to"] == "src/types.ts"
                && edge["strength"] == "hard")
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
fn structural_cone_depth_two_traverses_edges_not_task_terms() {
    let (repo, cache) = cone_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["cone", "src/session.ts", "--depth", "2", "--format", "json"])
        .output()
        .expect("ctx cone depth should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/cone.schema.json", &json);
    assert_eq!(json["depth"], 2);
    assert!(
        json["outgoing"]
            .as_array()
            .expect("outgoing array")
            .iter()
            .any(|edge| edge["from"] == "src/timeline.ts"
                && edge["to"] == "src/clock.ts"
                && edge["evidence"] == "resolved_import")
    );
}

#[test]
fn structural_cone_hides_edges_by_limit_with_expand_command() {
    let (repo, cache) = cone_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["cone", "src/session.ts", "--limit", "1", "--format", "json"])
        .output()
        .expect("ctx cone limit should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/cone.schema.json", &json);
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden array")
            .iter()
            .any(
                |hidden| hidden["reason"] == "outgoing edges hidden by limit"
                    && hidden["count"].as_u64().unwrap_or(0) >= 1
                    && hidden["expand"] == "ctx cone src/session.ts --depth 1 --include-hidden"
            )
    );
}

#[test]
fn structural_cone_directory_anchor_is_schema_valid_and_bounded() {
    let (repo, cache) = cone_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["cone", "src", "--limit", "2", "--format", "json"])
        .output()
        .expect("ctx cone dir should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/cone.schema.json", &json);
    assert_eq!(json["anchor"]["path"], "src");
    assert_eq!(json["anchor"]["kind"], "directory");
    assert!(
        json["unknowns"]
            .as_array()
            .expect("unknowns array")
            .iter()
            .any(|item| item == "directory anchor summarizes indexed files under this path")
    );
}

#[test]
fn structural_cone_markdown_uses_sections_without_route_language() {
    let (repo, cache) = cone_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["cone", "src/session.ts"])
        .output()
        .expect("ctx cone markdown should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("# Structural Cone"));
    assert!(stdout.contains("## Outgoing"));
    assert!(stdout.contains("## Proof"));
    assert!(stdout.contains("## Contracts"));
    assert!(stdout.contains("## Boundary"));
    assert!(!stdout.contains("Read first"));
    assert!(!stdout.contains("Confidence"));
    assert!(!stdout.contains("source_of_truth"));
}
