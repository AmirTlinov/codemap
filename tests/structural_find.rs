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

fn find_repo() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "find-root",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": { "test": "vitest run", "typecheck": "tsc --noEmit" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/package.json"),
        r#"{
  "name": "@fixture/replay",
  "version": "1.0.0",
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/src/types.ts"),
        "export interface FrameDto {\n  frame: number;\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import type { FrameDto } from './types';\n\nexport function seekFrame(cursor: number): FrameDto {\n  return { frame: cursor };\n}\n",
    );
    write(
        &repo.path().join("packages/replay/tests/session.test.ts"),
        "import { seekFrame } from '../src/session';\n\ntest('seek frame', () => {\n  expect(seekFrame(2).frame).toBe(2);\n});\n",
    );
    write(
        &repo.path().join("packages/replay/tests/token.test.ts"),
        "test('token behavior', () => {\n  expect('token').toBe('token');\n});\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    (repo, cache)
}

#[test]
fn find_exact_path_is_hard_anchor_and_points_to_ls_cone() {
    let (repo, cache) = find_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["find", "packages/replay/src/session.ts", "--format", "json"])
        .output()
        .expect("ctx find should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/find.schema.json", &json);
    assert_eq!(json["kind"], "anchor_candidates");
    assert_eq!(json["schema_version"], "2");
    let candidate = &json["candidates"][0];
    assert_eq!(candidate["path"], "packages/replay/src/session.ts");
    assert_eq!(candidate["evidence"], "exact_path");
    assert_eq!(candidate["strength"], "hard");
    assert_eq!(json["candidates"].as_array().expect("candidates").len(), 1);
    assert!(
        json["weak_matches"]
            .as_array()
            .expect("weak matches")
            .is_empty()
    );
    assert_eq!(
        candidate["next"][0],
        "ctx ls packages/replay/src/session.ts"
    );
    assert_eq!(
        candidate["next"][1],
        "ctx cone packages/replay/src/session.ts"
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
fn find_markdown_exact_path_prints_both_structural_next_steps() {
    let (repo, cache) = find_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["find", "packages/replay/src/session.ts"])
        .output()
        .expect("ctx find should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("utf8 markdown");
    assert!(markdown.contains("`ctx ls packages/replay/src/session.ts`"));
    assert!(markdown.contains("`ctx cone packages/replay/src/session.ts`"));
    assert!(
        !markdown.contains("ctx start"),
        "find markdown must not point agents back to legacy start"
    );
}

#[test]
fn find_symbol_or_export_returns_anchor_candidate_without_start_route() {
    let (repo, cache) = find_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["find", "FrameDto", "--format", "json"])
        .output()
        .expect("ctx find should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/find.schema.json", &json);
    assert!(
        json["candidates"]
            .as_array()
            .expect("candidates")
            .iter()
            .any(
                |candidate| candidate["path"] == "packages/replay/src/types.ts"
                    && (candidate["surface"] == "symbol" || candidate["surface"] == "export")
                    && candidate["strength"] == "high"
            )
    );
    assert!(
        json["candidates"]
            .as_array()
            .expect("candidates")
            .iter()
            .flat_map(|candidate| candidate["next"].as_array().into_iter().flatten())
            .all(|next| !next.as_str().unwrap_or_default().starts_with("ctx start")),
        "find must point to structural inspection commands, not legacy start"
    );
}

#[test]
fn find_package_and_script_surfaces_are_structural_candidates() {
    let (repo, cache) = find_repo();
    let package = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["find", "@fixture/replay", "--format", "json"])
        .output()
        .expect("ctx find package should run");
    assert!(package.status.success());
    let package_json: Value = serde_json::from_slice(&package.stdout).expect("valid json");
    assert_schema_accepts("schemas/find.schema.json", &package_json);
    assert!(
        package_json["candidates"]
            .as_array()
            .expect("candidates")
            .iter()
            .any(
                |candidate| candidate["path"] == "packages/replay/package.json"
                    && candidate["surface"] == "package"
                    && candidate["evidence"] == "package_manifest"
            )
    );

    let script = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["find", "typecheck", "--format", "json"])
        .output()
        .expect("ctx find script should run");
    assert!(script.status.success());
    let script_json: Value = serde_json::from_slice(&script.stdout).expect("valid json");
    assert_schema_accepts("schemas/find.schema.json", &script_json);
    assert!(
        script_json["candidates"]
            .as_array()
            .expect("candidates")
            .iter()
            .any(|candidate| candidate["path"] == "package.json"
                && candidate["surface"] == "script"
                && candidate["evidence"] == "script_hint")
    );
}

#[test]
fn find_separates_weak_token_matches_from_anchor_candidates() {
    let (repo, cache) = find_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["find", "token missingterm", "--format", "json"])
        .output()
        .expect("ctx find should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/find.schema.json", &json);
    assert!(
        json["weak_matches"]
            .as_array()
            .expect("weak matches")
            .iter()
            .any(
                |candidate| candidate["path"] == "packages/replay/tests/token.test.ts"
                    && candidate["surface"] == "token"
                    && candidate["strength"] == "low"
            )
    );
    assert!(
        json["candidates"]
            .as_array()
            .expect("candidates")
            .iter()
            .all(|candidate| candidate["path"] != "packages/replay/tests/token.test.ts"),
        "partial token overlap must not become a primary anchor candidate"
    );
}

#[test]
fn find_schema_is_exported_by_schema_command() {
    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["schema", "find"])
        .output()
        .expect("ctx schema find should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid schema json");
    assert_eq!(json["properties"]["kind"]["const"], "anchor_candidates");
}
