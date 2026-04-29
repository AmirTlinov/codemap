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

fn impact_repo() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "impact-root",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": { "test": "pnpm test" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/package.json"),
        r#"{
  "name": "@fixture/replay",
  "version": "1.0.0",
  "main": "src/index.ts",
  "exports": { ".": "./src/index.ts" },
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "private": true,
  "dependencies": { "@fixture/replay": "workspace:*" },
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("packages/inspector/package.json"),
        r#"{
  "name": "@fixture/inspector",
  "private": true,
  "dependencies": { "@fixture/replay": "workspace:*" },
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/src/index.ts"),
        "export { seek } from './session';\nexport { internalValue } from './internal';\nexport type { FrameDto } from './types';\n",
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
        &repo.path().join("packages/replay/src/preview.ts"),
        "import type { FrameDto } from './types';\n\nexport function preview(frame: FrameDto): number {\n  return frame.frame;\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/internal.ts"),
        "export const internalValue = 1;\n",
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
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { seek } from '@fixture/replay';\n\nexport const frame = seek(1).frame;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    (repo, cache)
}

#[test]
fn structural_impact_clusters_contract_consumers_and_proof() {
    let (repo, cache) = impact_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--structural",
            "--files",
            "packages/replay/src/types.ts",
            "--depth",
            "2",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact structural should run");
    assert!(
        output.status.success(),
        "ctx impact structural failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/impact-v2.schema.json", &json);
    assert_eq!(json["kind"], "impact_v2_report");
    assert_eq!(json["schema_version"], "2");
    assert_eq!(json["changed"][0]["path"], "packages/replay/src/types.ts");
    assert_eq!(json["changed"][0]["kind"], "schema_contract");
    let cluster = &json["clusters"][0];
    assert_eq!(cluster["risk"], "high");
    assert!(
        cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/session.ts"
                && edge["to"] == "packages/replay/src/types.ts"
                && edge["type"] == "direct_consumer"
                && edge["evidence"] == "reverse_import")
    );
    assert!(
        cluster["cross_boundary_consumers"]
            .as_array()
            .expect("cross consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/app/package.json"
                && edge["to"] == "packages/replay/src/types.ts"
                && edge["type"] == "package_consumer"
                && edge["evidence"] == "package_manifest_reverse_dependency")
    );
    assert!(
        cluster["contract_risks"]
            .as_array()
            .expect("contract risks")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/types.ts"
                && edge["to"] == "packages/replay/src/types.ts"
                && edge["type"] == "contract_changed"
                && edge["evidence"] == "role:schema_contract")
    );
    assert!(
        cluster["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(
                |edge| edge["from"] == "packages/replay/tests/session.test.ts"
                    && edge["to"] == "packages/replay/src/session.ts"
                    && edge["type"] == "tests"
            )
    );
    assert_eq!(json.get("read_first"), None);
    assert_eq!(json.get("confidence"), None);
    assert!(
        json["expand"]
            .as_array()
            .expect("expand")
            .iter()
            .all(|command| !command
                .as_str()
                .unwrap_or_default()
                .starts_with("ctx proof")),
        "impact must not point at unavailable proof command"
    );
    assert!(
        json["expand"]
            .as_array()
            .expect("expand")
            .iter()
            .any(|command| command
                .as_str()
                .unwrap_or_default()
                .contains("ctx verify --files packages/replay/src/types.ts")),
        "impact follow-up verification should preserve exact changed anchors"
    );
    assert!(
        !serde_json::to_string(&json)
            .expect("json string")
            .contains("source_of_truth")
    );
}

#[test]
fn structural_impact_marks_public_boundary_reverse_consumers_as_contract_risk() {
    let (repo, cache) = impact_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--structural",
            "--files",
            "packages/replay/src/internal.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact structural should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/impact-v2.schema.json", &json);
    let cluster = &json["clusters"][0];
    assert_eq!(cluster["risk"], "high");
    assert!(
        cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/index.ts"
                && edge["to"] == "packages/replay/src/internal.ts"
                && edge["type"] == "direct_consumer")
    );
    assert!(
        cluster["contract_risks"]
            .as_array()
            .expect("contract risks")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/index.ts"
                && edge["to"] == "packages/replay/src/internal.ts"
                && edge["type"] == "contract_consumer"
                && edge["evidence"] == "role:public_boundary")
    );
    assert!(
        cluster["cross_boundary_consumers"]
            .as_array()
            .expect("cross consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/app/package.json"
                && edge["to"] == "packages/replay/src/internal.ts"
                && edge["type"] == "package_consumer"
                && edge["evidence"] == "package_manifest_reverse_dependency")
    );
}

#[test]
fn structural_impact_does_not_promote_same_domain_tests_without_import_or_name_evidence() {
    let (repo, cache) = impact_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--structural",
            "--files",
            "packages/replay/src/helper.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact structural should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/impact-v2.schema.json", &json);
    let cluster = &json["clusters"][0];
    assert!(
        cluster["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .all(|edge| edge["from"] != "packages/replay/tests/token.test.ts"),
        "same-domain tests without import or name evidence must stay out of proof"
    );
}

#[test]
fn structural_impact_reports_hidden_edges_when_limit_truncates_sections() {
    let (repo, cache) = impact_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--structural",
            "--files",
            "packages/replay/src/types.ts",
            "--limit",
            "1",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact structural should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/impact-v2.schema.json", &json);
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("direct consumer edges hidden by limit")
                && hidden["count"].as_u64().unwrap_or_default() > 0),
        "edge truncation must be visible in hidden groups"
    );
}

#[test]
fn structural_impact_reports_hidden_package_consumers_when_limit_truncates_sections() {
    let (repo, cache) = impact_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--structural",
            "--files",
            "packages/replay/package.json",
            "--limit",
            "1",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact structural should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/impact-v2.schema.json", &json);
    assert!(
        json["clusters"][0]["cross_boundary_consumers"]
            .as_array()
            .expect("cross consumers")
            .iter()
            .any(|edge| edge["type"] == "package_consumer"),
        "package consumer edge should remain visible under limit"
    );
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("cross-boundary consumer edges hidden by limit")
                && hidden["count"].as_u64().unwrap_or_default() > 0),
        "package consumer truncation must be visible in hidden groups"
    );
}

#[test]
fn structural_impact_empty_diff_is_schema_valid_empty_report() {
    let (repo, cache) = impact_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["impact", "--structural", "--changed", "--format", "json"])
        .output()
        .expect("ctx impact structural should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/impact-v2.schema.json", &json);
    assert!(json["changed"].as_array().unwrap().is_empty());
    assert!(json["clusters"].as_array().unwrap().is_empty());
}

#[test]
fn structural_impact_markdown_uses_cluster_language_not_route_language() {
    let (repo, cache) = impact_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--structural",
            "--files",
            "packages/replay/src/types.ts",
        ])
        .output()
        .expect("ctx impact structural markdown should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("# Structural Impact"));
    assert!(stdout.contains("## Changed Anchors"));
    assert!(stdout.contains("## Cluster `changed:packages/replay/src/types.ts`"));
    assert!(stdout.contains("## Direct Consumers"));
    assert!(stdout.contains("## Contract Risks"));
    assert!(!stdout.contains("Read first"));
    assert!(!stdout.contains("Confidence"));
    assert!(!stdout.contains("source_of_truth"));
}

#[test]
fn impact_v2_schema_is_exported_by_schema_command() {
    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["schema", "impact-v2"])
        .output()
        .expect("schema impact-v2 should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid schema json");
    assert_eq!(json["properties"]["kind"]["const"], "impact_v2_report");
    assert_eq!(json["properties"]["schema_version"]["const"], "2");
}
