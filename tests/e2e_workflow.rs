use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn ctx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ctx"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn fixture_copy(name: &str) -> TempDir {
    let temp = TempDir::new().expect("fixture tempdir");
    copy_dir(&fixture(name), temp.path());
    temp
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create fixture copy target");
    for entry in fs::read_dir(from).expect("read fixture dir") {
        let entry = entry.expect("read fixture entry");
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir(&source, &target);
        } else {
            fs::copy(&source, &target).expect("copy fixture file");
        }
    }
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

fn git_status(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(dir)
        .output()
        .expect("git status should run");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("status should be utf8")
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

#[test]
fn agent_workflow_start_impact_verify_boundaries_is_end_to_end() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1

domains:
  replay:
    path: domains/replay

concepts:
  replay.timeline:
    role: source_of_truth
    files:
      - domains/replay/src/replay-timeline.ts
    invariants:
      - deterministic_for_same_input

boundaries:
  forbidden:
    - from: domains/replay/src/**
      to: domains/renderer/src/**
      reason: replay emits DTOs; renderer consumes DTOs
      recovery:
        - extend replay DTO
        - update renderer adapter
        - update contract tests

task_routes:
  playback_session:
    match:
      - frame
      - seek
      - cursor
      - playback
    read_first:
      - domains/replay/src/replay-session.ts
      - domains/replay/src/replay-timeline.ts
      - domains/replay/tests/replay-session.test.ts
    verify:
      - test -f domains/replay/src/replay-session.ts

verification:
  default:
    - test -f domains/replay/src/replay-session.ts
"#,
    );
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let start = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--task",
            "fix replay jumping to wrong frame after seek",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start should run");
    assert!(start.status.success());
    let start_json: Value = serde_json::from_slice(&start.stdout).expect("valid start json");
    assert_schema_accepts("schemas/capsule.schema.json", &start_json);
    assert_eq!(start_json["domain"]["path"], "domains/replay");
    assert_eq!(start_json["confidence"], "high");
    assert!(
        start_json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/replay/src/replay-session.ts"))
    );
    assert!(
        start_json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/renderer/**"))
    );
    assert!(
        git_status(repo.path()).is_empty(),
        "ctx start must not write project files"
    );
    assert!(
        fs::read_dir(cache.path())
            .expect("cache dir")
            .next()
            .is_some(),
        "ctx should write cache outside the project"
    );

    write(
        &repo.path().join("domains/replay/src/replay-timeline.ts"),
        "export function frameAt(timeMs: number): number {\n  return Math.max(0, Math.floor(timeMs / 16));\n}\n",
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["impact", "--changed", "--depth", "2", "--format", "json"])
        .output()
        .expect("ctx impact should run");
    assert!(impact.status.success());
    let impact_json: Value = serde_json::from_slice(&impact.stdout).expect("valid impact json");
    assert_schema_accepts("schemas/impact.schema.json", &impact_json);
    assert!(
        impact_json["changed"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/src/replay-timeline.ts"))
    );
    assert!(
        impact_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/src/replay-session.ts"))
    );

    let verify = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["verify", "--changed", "--depth", "2", "--format", "json"])
        .output()
        .expect("ctx verify should run");
    assert!(verify.status.success());
    let verify_json: Value = serde_json::from_slice(&verify.stdout).expect("valid verify json");
    assert_schema_accepts("schemas/verify.schema.json", &verify_json);
    assert_eq!(
        verify_json["verification"]["minimal"][0],
        "test -f domains/replay/src/replay-session.ts"
    );

    let run = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["verify", "--changed", "--depth", "2", "--run"])
        .output()
        .expect("ctx verify --run should run");
    assert!(run.status.success());
    let run_stdout = String::from_utf8(run.stdout).expect("stdout should be utf8");
    assert!(run_stdout.contains("$ test -f domains/replay/src/replay-session.ts"));

    let boundaries = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["boundaries", "--changed"])
        .output()
        .expect("ctx boundaries should run");
    assert!(boundaries.status.success());
    let boundaries_stdout = String::from_utf8(boundaries.stdout).expect("stdout should be utf8");
    assert!(boundaries_stdout.contains("No boundary findings"));

    let explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["explain", "replay.timeline", "--format", "json"])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    assert_eq!(explain_json["kind"], "concept");
    assert!(
        explain_json["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/src/replay-timeline.ts"))
    );
}
