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

fn git_status(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["status", "--short"])
        .current_dir(dir)
        .output()
        .expect("git status should run");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("status should be utf8")
}

#[test]
fn doctor_runs_with_zero_footprint_contract() {
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .arg("doctor")
        .env("CTX_CACHE_DIR", cache.path())
        .output()
        .expect("failed to run ctx doctor");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("# ctx doctor"));
    assert!(stdout.contains("Zero-footprint default"));
    assert!(stdout.contains("true"));
}

#[test]
fn start_routes_task_without_writing_to_project() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"echo test ok","typecheck":"echo typecheck ok"}}"#,
    );
    write(
        &repo.path().join("src/index.ts"),
        "export { saveGame } from './save';\nexport { reopenGame } from './reopen';\n",
    );
    write(
        &repo.path().join("src/save.ts"),
        "export function saveGame(x: string) { return 'saved:' + x }\n",
    );
    write(
        &repo.path().join("src/reopen.ts"),
        "import { saveGame } from './save';\nexport function reopenGame(x: string) { return saveGame(x); }\n",
    );
    write(
        &repo.path().join("tests/save.test.ts"),
        "import { saveGame } from '../src/save';\nconsole.log(saveGame('x'));\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--task",
            "fix broken save after reopen",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["kind"], "task_context_capsule");
    assert_eq!(json["task_kind"], "persistence");
    let read_first = json["read_first"].as_array().expect("read_first array");
    assert!(
        read_first
            .iter()
            .any(|item| item["path"].as_str() == Some("src/save.ts"))
    );
    assert!(
        git_status(repo.path()).is_empty(),
        "ctx start must not write project files"
    );
}

#[test]
fn nested_agents_does_not_replace_git_root() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(&repo.path().join("root.txt"), "root\n");
    write(&repo.path().join("packages/foo/AGENTS.md"), "# Local\n");
    write(
        &repo.path().join("packages/foo/index.ts"),
        "export const x = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path().join("packages/foo"))
        .env("CTX_CACHE_DIR", cache.path())
        .args(["status", "--format", "json"])
        .output()
        .expect("ctx status should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let expected_root = fs::canonicalize(repo.path())
        .expect("canonical temp repo")
        .to_string_lossy()
        .to_string();
    assert_eq!(json["root"].as_str(), Some(expected_root.as_str()));
    assert_eq!(
        json["nearest_agents"].as_str(),
        Some("packages/foo/AGENTS.md")
    );
}

#[test]
fn verify_prints_plan_and_does_not_run_without_run() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"echo SHOULD_NOT_RUN"}}"#,
    );
    write(&repo.path().join("index.ts"), "export const x = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);
    write(&repo.path().join("index.ts"), "export const x = 2;\n");

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["verify", "--changed"])
        .output()
        .expect("ctx verify should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("npm test"));
    assert!(stdout.contains("does not run commands unless `--run` is explicit"));
    assert!(!stdout.contains("SHOULD_NOT_RUN"));
}

#[test]
fn verify_run_fails_when_only_placeholder_is_inferred() {
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
        .args(["verify", "--changed", "--run"])
        .output()
        .expect("ctx verify should run");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("cannot run placeholder verification"));
}

#[test]
fn init_default_writes_nothing() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(&repo.path().join("main.py"), "print('x')\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .arg("init")
        .output()
        .expect("ctx init should run");
    assert!(output.status.success());
    assert!(
        git_status(repo.path()).is_empty(),
        "plain ctx init must not write"
    );
}

#[test]
fn invalid_ctx_config_is_reported_and_blocks_routing() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(&repo.path().join(".ctx.yml"), "version: [\n");
    write(&repo.path().join("src/lib.rs"), "pub fn demo() {}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let validate = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["anchors", "validate", "--format", "json"])
        .output()
        .expect("ctx anchors validate should run");
    assert!(validate.status.success());
    let json: Value = serde_json::from_slice(&validate.stdout).expect("valid json");
    assert_eq!(json["ok"], false);
    assert!(
        json["problems"]
            .as_array()
            .unwrap()
            .iter()
            .any(|problem| problem.as_str().unwrap().contains(".ctx.yml"))
    );

    let start = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix demo"])
        .output()
        .expect("ctx start should run");
    assert!(!start.status.success());
    let stderr = String::from_utf8(start.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("invalid .ctx semantic anchors"));
}

#[test]
fn domain_local_anchor_paths_resolve_under_domain_path() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
domain:
  id: replay
  path: domains/replay
concepts:
  replay.timeline:
    role: source_of_truth
    files:
      - src/replay-timeline.ts
    invariants:
      - deterministic_for_same_input
task_routes:
  playback_session:
    match:
      - seek
    read_first:
      - src/replay-session.ts
      - src/replay-timeline.ts
    verify:
      - cargo test -p replay
"#,
    );
    write(
        &repo.path().join("domains/replay/src/replay-session.ts"),
        "import { timeline } from './replay-timeline';\nexport const session = timeline;\n",
    );
    write(
        &repo.path().join("domains/replay/src/replay-timeline.ts"),
        "export const timeline = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let validate = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["anchors", "validate", "--format", "json"])
        .output()
        .expect("ctx anchors validate should run");
    assert!(validate.status.success());
    let validation: Value = serde_json::from_slice(&validate.stdout).expect("valid json");
    assert_eq!(validation["ok"], true);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix seek frame", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let read_first = json["read_first"].as_array().expect("read_first array");
    assert!(
        read_first
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/replay/src/replay-session.ts"))
    );
    assert!(
        json["source_of_truth"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item.as_str() == Some("domains/replay/src/replay-timeline.ts") })
    );
    assert_eq!(json["verification"]["minimal"][0], "cargo test -p replay");
}

#[test]
fn explicit_forbidden_boundary_edge_fails_closed() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: domains/replay/src/**
      to: domains/renderer/src/**
      reason: replay emits DTOs; renderer consumes DTOs
      recovery:
        - extend replay DTO
        - update renderer adapter
"#,
    );
    write(
        &repo.path().join("domains/replay/src/session.ts"),
        "import { renderReplay } from '../../renderer/src/replay-renderer';\nexport const session = renderReplay;\n",
    );
    write(
        &repo.path().join("domains/renderer/src/replay-renderer.ts"),
        "export function renderReplay() { return 'rendered'; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .arg("boundaries")
        .output()
        .expect("ctx boundaries should run");
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("replay emits DTOs"));
    assert!(stdout.contains("domains/replay/src/session.ts"));
}

#[test]
fn package_manifest_boundary_edge_fails_closed() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: domains/replay/src/**
      to: domains/renderer/src/**
      reason: replay emits DTOs; renderer consumes DTOs
"#,
    );
    write(
        &repo.path().join("domains/replay/package.json"),
        r#"{
  "name": "@fixture/replay",
  "dependencies": {
    "@fixture/renderer": "workspace:*"
  }
}"#,
    );
    write(
        &repo.path().join("domains/replay/src/session.ts"),
        "export const session = 1;\n",
    );
    write(
        &repo.path().join("domains/renderer/package.json"),
        r#"{"name":"@fixture/renderer"}"#,
    );
    write(
        &repo.path().join("domains/renderer/src/replay-renderer.ts"),
        "export const renderer = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .arg("boundaries")
        .output()
        .expect("ctx boundaries should run");
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("package manifest dependency `@fixture/renderer`"));
    assert!(stdout.contains("domains/replay/package.json"));
}

#[test]
fn nested_ctx_config_is_loaded_as_domain_anchor() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("domains/replay/.ctx.yml"),
        r#"version: 1
domain:
  id: replay
concepts:
  replay.session:
    role: runtime_state
    files:
      - src/replay-session.ts
task_routes:
  playback_session:
    match:
      - seek
    read_first:
      - src/replay-session.ts
    verify:
      - cargo test -p replay
"#,
    );
    write(
        &repo.path().join("domains/replay/src/replay-session.ts"),
        "export const frame = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix seek frame", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["domain"]["path"], "domains/replay");
    assert_eq!(json["confidence"], "high");
    assert_eq!(
        json["read_first"][0]["path"],
        "domains/replay/src/replay-session.ts"
    );
}

#[test]
fn verify_uses_impact_traversal_for_recommended_checks() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"echo test ok","typecheck":"echo typecheck ok"}}"#,
    );
    write(
        &repo.path().join("src/token.ts"),
        "export const token = 'old';\n",
    );
    write(
        &repo.path().join("src/session.ts"),
        "import { token } from './token';\nexport { token };\n",
    );
    write(
        &repo.path().join("src/index.ts"),
        "import { token } from './session';\nexport { token };\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "verify",
            "--files",
            "src/token.ts",
            "--depth",
            "2",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx verify should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["kind"], "verification_plan");
    let recommended = json["verification"]["recommended"].as_array().unwrap();
    assert!(
        recommended
            .iter()
            .any(|cmd| cmd.as_str() == Some("npm run typecheck")),
        "verify should recommend typecheck because impact traversal reaches public src/index.ts"
    );
}

#[test]
fn global_instruction_does_not_advertise_fake_agent_mode_flag() {
    let output = ctx()
        .args(["bootstrap", "--global-instruction"])
        .output()
        .expect("ctx bootstrap should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!stdout.contains("--for-agent"));
    assert!(stdout.contains("ctx start --task"));
}

#[test]
fn json_schemas_are_present_and_parse() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "schemas/capsule.schema.json",
        "schemas/impact.schema.json",
        "schemas/verify.schema.json",
    ] {
        let text = fs::read_to_string(root.join(rel)).expect("schema should exist");
        let json: Value = serde_json::from_str(&text).expect("schema should be valid json");
        assert_eq!(
            json["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(json["required"].as_array().unwrap().len() >= 8);
    }

    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"echo test ok"}}"#,
    );
    write(
        &repo.path().join("src/save.ts"),
        "export function saveGame(x: string) { return x }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let capsule = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix broken save", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(capsule.status.success());
    let capsule_json: Value = serde_json::from_slice(&capsule.stdout).expect("valid capsule json");
    assert_schema_accepts("schemas/capsule.schema.json", &capsule_json);

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["impact", "--files", "src/save.ts", "--format", "json"])
        .output()
        .expect("ctx impact should run");
    assert!(impact.status.success());
    let impact_json: Value = serde_json::from_slice(&impact.stdout).expect("valid impact json");
    assert_schema_accepts("schemas/impact.schema.json", &impact_json);

    let verify = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["verify", "--files", "src/save.ts", "--format", "json"])
        .output()
        .expect("ctx verify should run");
    assert!(verify.status.success());
    let verify_json: Value = serde_json::from_slice(&verify.stdout).expect("valid verify json");
    assert_schema_accepts("schemas/verify.schema.json", &verify_json);
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
fn task_keywords_without_matching_files_do_not_claim_high_confidence() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("src/model.rs"), "pub struct Model;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
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
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_ne!(json["confidence"], "high");
    assert!(json["read_first"].as_array().unwrap().is_empty());
}

#[test]
fn absolute_start_path_selects_target_repo_from_any_cwd() {
    let repo = TempDir::new().expect("repo tempdir");
    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"echo test ok"}}"#,
    );
    write(
        &repo.path().join("src/save.ts"),
        "export function saveGame(x: string) { return x }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--task",
            "fix broken save",
            "--path",
            repo.path().to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("src/save.ts"))
    );
    assert_eq!(json["domain"]["path"], ".");
}

#[test]
fn absolute_file_args_are_normalized_to_repo_relative_paths() {
    let repo = TempDir::new().expect("repo tempdir");
    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("src/save.ts"),
        "export function saveGame(x: string) { return x }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);
    let absolute = repo.path().join("src/save.ts");

    let output = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            absolute.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["changed"][0], "src/save.ts");
}

#[test]
fn init_write_minimal_refuses_absolute_path_outside_repo() {
    let repo = TempDir::new().expect("repo tempdir");
    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(&repo.path().join("main.py"), "print('x')\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "init",
            "--write-minimal",
            "--path",
            outside.path().to_str().unwrap(),
        ])
        .output()
        .expect("ctx init should run");
    assert!(!output.status.success());
    assert!(!outside.path().join(".ctx.yml").exists());
}

#[test]
fn init_write_minimal_creates_domain_directory_when_explicit() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(&repo.path().join("main.py"), "print('x')\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["init", "--write-minimal", "--path", "domains/replay"])
        .output()
        .expect("ctx init should run");
    assert!(output.status.success());
    let written = fs::read_to_string(repo.path().join("domains/replay/.ctx.yml"))
        .expect("ctx config should be written");
    assert!(written.contains("id: replay"));

    let validate = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["anchors", "validate", "--format", "json"])
        .output()
        .expect("ctx anchors validate should run");
    assert!(validate.status.success());
    let json: Value = serde_json::from_slice(&validate.stdout).expect("valid json");
    assert_eq!(json["ok"], true);
}
