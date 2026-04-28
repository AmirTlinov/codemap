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
