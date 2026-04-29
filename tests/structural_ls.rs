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

fn structural_repo() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "ls-fixture",
  "private": true,
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("src/timeline.ts"),
        "export class Timeline {\n  frameAt(cursor: number) {\n    return cursor;\n  }\n}\n",
    );
    write(
        &repo.path().join("src/session.ts"),
        "import { Timeline } from './timeline';\n\nexport function seek(cursor: number) {\n  return new Timeline().frameAt(cursor);\n}\n",
    );
    write(
        &repo.path().join("src/session.test.ts"),
        "import { seek } from './session';\n\ntest('seek maps cursor', () => {\n  expect(seek(4)).toBe(4);\n});\n",
    );
    write(&repo.path().join("src/empty.ts"), "\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    (repo, cache)
}

#[test]
fn structural_ls_file_uses_exact_anchor_and_schema_v2() {
    let (repo, cache) = structural_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["ls", "src/session.ts", "--format", "json"])
        .output()
        .expect("ctx ls should run");
    assert!(
        output.status.success(),
        "ctx ls failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/ls.schema.json", &json);
    assert_eq!(json["kind"], "ls_report");
    assert_eq!(json["schema_version"], "2");
    assert_eq!(json["mode"], "file");
    assert_eq!(json["path"], "src/session.ts");
    assert_eq!(json["anchor"]["path"], "src/session.ts");
    assert_eq!(json["anchor"]["package"], "ls-fixture");
    assert_eq!(json["anchor"]["imported_by_count"], 1);
    assert!(
        json["anchor"]["symbols"]
            .as_array()
            .expect("symbols array")
            .iter()
            .any(|symbol| symbol["name"] == "seek"
                && symbol["kind"] == "function"
                && symbol["line_start"] == 3
                && symbol["line_end"] == 5)
    );
    assert!(
        json["edges"]
            .as_array()
            .expect("edges array")
            .iter()
            .any(|edge| edge["from"] == "src/session.ts"
                && edge["to"] == "src/timeline.ts"
                && edge["type"] == "imports"
                && edge["strength"] == "high")
    );
    assert!(
        json["edges"]
            .as_array()
            .expect("edges array")
            .iter()
            .any(|edge| edge["from"] == "src/session.test.ts"
                && edge["to"] == "src/session.ts"
                && edge["type"] == "tests")
    );
    assert_eq!(json.get("read_first"), None);
    assert_eq!(json.get("confidence"), None);
}

#[test]
fn structural_ls_file_markdown_is_surface_summary_not_route() {
    let (repo, cache) = structural_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["ls", "src/session.ts"])
        .output()
        .expect("ctx ls should run");
    assert!(
        output.status.success(),
        "ctx ls failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("# Structural LS"));
    assert!(stdout.contains("Path: `src/session.ts`"));
    assert!(stdout.contains("## Symbols"));
    assert!(stdout.contains("seek"));
    assert!(stdout.contains("ctx cone src/session.ts"));
    assert!(!stdout.contains("Read first"));
    assert!(!stdout.contains("Confidence"));
}

#[test]
fn structural_ls_does_not_expose_legacy_source_of_truth_guess() {
    let (repo, cache) = structural_repo();
    let json_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["ls", "src/timeline.ts", "--format", "json"])
        .output()
        .expect("ctx ls json should run");
    assert!(
        json_output.status.success(),
        "ctx ls json failed: {}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let json: Value = serde_json::from_slice(&json_output.stdout).expect("valid json");
    assert_schema_accepts("schemas/ls.schema.json", &json);
    assert_eq!(json["mode"], "file");
    assert_eq!(json["anchor"]["kind"], "source");
    assert!(
        !json["anchor"]["roles"]
            .as_array()
            .expect("roles array")
            .iter()
            .any(|role| role == "source_of_truth")
    );

    let markdown_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["ls", "src/timeline.ts"])
        .output()
        .expect("ctx ls markdown should run");
    assert!(markdown_output.status.success());
    let stdout = String::from_utf8(markdown_output.stdout).expect("stdout utf8");
    assert!(!stdout.contains("source_of_truth"));
}

#[test]
fn structural_ls_directory_groups_surfaces_and_hides_generic_noise() {
    let (repo, cache) = structural_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["ls", "src", "--format", "json"])
        .output()
        .expect("ctx ls dir should run");
    assert!(
        output.status.success(),
        "ctx ls dir failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/ls.schema.json", &json);
    assert_eq!(json["mode"], "directory");
    assert_eq!(json["path"], "src");
    assert!(
        json["directory"]
            .as_array()
            .expect("directory array")
            .iter()
            .any(|surface| surface["kind"] == "test" && surface["count"] == 1)
    );
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden array")
            .iter()
            .any(|hidden| hidden["reason"] == "generic source files hidden"
                && hidden["count"] == 1)
    );
}

#[test]
fn structural_ls_missing_is_schema_valid_and_points_to_parent_ls() {
    let (repo, cache) = structural_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["ls", "src/missing.ts", "--format", "json"])
        .output()
        .expect("ctx ls missing should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_schema_accepts("schemas/ls.schema.json", &json);
    assert_eq!(json["mode"], "missing");
    assert_eq!(json["anchor"], Value::Null);
    assert_eq!(json["next"][0], "ctx ls src");
}

#[test]
fn structural_ls_accepts_absolute_file_anchor() {
    let (repo, cache) = structural_repo();
    let absolute = repo.path().join("src/session.ts");
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "ls",
            absolute.to_str().expect("utf8 path"),
            "--format",
            "json",
        ])
        .output()
        .expect("ctx ls absolute should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["mode"], "file");
    assert_eq!(json["anchor"]["path"], "src/session.ts");
}
