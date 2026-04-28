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
fn schema_command_outputs_bundled_contract_without_repo_load() {
    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    let capsule = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["schema", "capsule"])
        .output()
        .expect("ctx schema should run");

    assert!(capsule.status.success());
    let json: Value = serde_json::from_slice(&capsule.stdout).expect("schema should be json");
    assert_eq!(json["properties"]["kind"]["const"], "task_context_capsule");
    assert_eq!(json["properties"]["schema_version"]["const"], "1");

    let anchors = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["schema", "anchors"])
        .output()
        .expect("ctx schema anchors should run");
    assert!(anchors.status.success());
    let anchors_json: Value =
        serde_json::from_slice(&anchors.stdout).expect("anchors schema should be json");
    assert_eq!(anchors_json["title"], "ctx semantic anchors");
    assert_eq!(anchors_json["properties"]["version"]["const"], 1);

    let graph = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["schema", "graph"])
        .output()
        .expect("ctx schema graph should run");
    assert!(graph.status.success());
    let graph_json: Value = serde_json::from_slice(&graph.stdout).expect("graph schema json");
    assert_eq!(graph_json["properties"]["kind"]["const"], "graph_lens");
    assert_eq!(graph_json["properties"]["schema_version"]["const"], "1");

    assert_eq!(
        fs::read_dir(cache.path()).expect("cache dir").count(),
        0,
        "ctx schema must not load a project or write cache"
    );
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
    let project_cache = fs::read_dir(cache.path())
        .expect("cache dir")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .expect("project cache dir");
    for artifact in [
        "status.json",
        "inventory.json",
        "graph.json",
        "fingerprints.json",
    ] {
        assert!(
            project_cache.join(artifact).exists(),
            "cache artifact {artifact} should be written outside the project"
        );
    }
    let inventory: Value = serde_json::from_slice(
        &fs::read(project_cache.join("inventory.json")).expect("read inventory cache"),
    )
    .expect("inventory cache json");
    assert!(
        inventory["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("src/save.ts"))
    );
    let graph: Value =
        serde_json::from_slice(&fs::read(project_cache.join("graph.json")).expect("read graph"))
            .expect("graph cache json");
    assert!(graph["edges"].as_array().unwrap().iter().any(|edge| {
        edge["from"].as_str() == Some("src/reopen.ts")
            && edge["to"].as_str() == Some("src/save.ts")
            && edge["kind"].as_str() == Some("imports")
    }));
}

#[test]
fn status_reports_external_cache_state_without_self_warming() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"echo test ok"}}"#,
    );
    write(&repo.path().join("src/lib.ts"), "export const value = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let cold = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["status", "--format", "json"])
        .output()
        .expect("ctx status should run");
    assert!(cold.status.success());
    let cold_json: Value = serde_json::from_slice(&cold.stdout).expect("valid status json");
    assert_eq!(cold_json["cache_state"], "cold");
    assert!(
        cold_json["cache_artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["exists"] == false),
        "fresh status should observe a cold cache instead of self-warming it"
    );

    let start = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "inspect lib value"])
        .output()
        .expect("ctx start should run");
    assert!(start.status.success());

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["status", "--format", "json"])
        .output()
        .expect("ctx status should run");
    assert!(output.status.success());
    assert!(
        git_status(repo.path()).is_empty(),
        "ctx status must not write project files"
    );
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid status json");
    assert_eq!(json["kind"], "status_report");
    assert_eq!(json["schema_version"], "1");
    assert_eq!(json["cache_state"], "warm");
    let artifacts = json["cache_artifacts"]
        .as_array()
        .expect("cache artifacts array");
    for artifact in [
        "status.json",
        "inventory.json",
        "graph.json",
        "fingerprints.json",
    ] {
        let status = artifacts
            .iter()
            .find(|item| item["name"].as_str() == Some(artifact))
            .unwrap_or_else(|| panic!("missing cache artifact status for {artifact}"));
        assert_eq!(status["exists"], true);
        assert_eq!(status["fingerprint_match"], true);
        assert!(
            status["bytes"].as_u64().unwrap_or(0) > 0,
            "cache artifact {artifact} should be non-empty"
        );
        let path = Path::new(status["path"].as_str().expect("artifact path"));
        assert!(
            path.starts_with(cache.path()),
            "cache artifact should live under CTX_CACHE_DIR"
        );
    }

    let status_artifact = artifacts
        .iter()
        .find(|item| item["name"].as_str() == Some("status.json"))
        .expect("status artifact");
    let status_path = Path::new(status_artifact["path"].as_str().expect("artifact path"));
    write(status_path, r#"{"fingerprint":"stale"}"#);

    let stale = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["status", "--format", "json"])
        .output()
        .expect("ctx status should run");
    assert!(stale.status.success());
    let stale_json: Value = serde_json::from_slice(&stale.stdout).expect("valid status json");
    assert_eq!(stale_json["cache_state"], "stale");
    assert!(
        stale_json["cache_artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["name"].as_str() == Some("status.json")
                && item["fingerprint_match"] == false)
    );

    let disabled = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .env("CTX_NO_CACHE", "1")
        .args(["status", "--format", "json"])
        .output()
        .expect("ctx status should run");
    assert!(disabled.status.success());
    let disabled_json: Value = serde_json::from_slice(&disabled.stdout).expect("valid status json");
    assert_eq!(disabled_json["cache_state"], "disabled");
}

#[test]
fn context_routing_tasks_prefer_implementation_over_output_schemas() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"ctx-like\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &repo.path().join("src/route.rs"),
        "pub fn start_capsule() {}\n",
    );
    write(&repo.path().join("src/repo.rs"), "pub fn discover() {}\n");
    write(&repo.path().join("src/cli.rs"), "pub fn command() {}\n");
    write(&repo.path().join("src/cache.rs"), "pub fn cache() {}\n");
    write(
        &repo.path().join("schemas/capsule.schema.json"),
        r#"{"title":"capsule"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    for task in [
        "improve ctx routing quality",
        "continue ctx end-to-end implementation",
    ] {
        let output = ctx()
            .current_dir(repo.path())
            .env("CTX_CACHE_DIR", cache.path())
            .args([
                "start",
                "--path",
                repo.path().to_str().unwrap(),
                "--task",
                task,
                "--format",
                "json",
            ])
            .output()
            .expect("ctx start should run");
        assert!(output.status.success(), "{task}");
        let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
        assert_eq!(json["task_kind"], "context_routing", "{task}");
        assert_eq!(json["read_first"][0]["path"], "src/route.rs", "{task}");
        assert!(
            json["read_first"]
                .as_array()
                .unwrap()
                .iter()
                .take(3)
                .all(|item| item["path"].as_str().unwrap_or("").starts_with("src/")),
            "{task}"
        );
    }
}

#[test]
fn task_keyword_matching_does_not_match_inside_other_words() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"keyword-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("src/render.rs"), "pub fn render() {}\n");
    write(&repo.path().join("src/cli.rs"), "pub fn command() {}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let build = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix build error", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(build.status.success());
    let build_json: Value = serde_json::from_slice(&build.stdout).expect("valid json");
    assert_eq!(build_json["task_kind"], "build_ci");

    let ui = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix ui button", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(ui.status.success());
    let ui_json: Value = serde_json::from_slice(&ui.stdout).expect("valid json");
    assert_eq!(ui_json["task_kind"], "ui_rendering");

    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"domain-keyword-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &repo.path().join("domains/guide/src/model.rs"),
        "pub fn guide_model() {}\n",
    );
    write(
        &repo.path().join("domains/ui/src/view.rs"),
        "pub fn render_button() {}\n",
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed/domains/guide/package.json"),
        r#"{"name":"@fixture/guide"}"#,
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed/domains/guide/src/model.ts"),
        "export const guideModel = 1;\n",
    );
    write(
        &repo.path().join("fixtures/mixed/domains/ui/package.json"),
        r#"{"name":"@fixture/ui"}"#,
    );
    write(
        &repo.path().join("fixtures/mixed/domains/ui/src/view.ts"),
        "export const buttonView = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let ui = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix ui button", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(ui.status.success());
    let ui_json: Value = serde_json::from_slice(&ui.stdout).expect("valid json");
    assert_eq!(ui_json["domain"]["path"], "domains/ui");

    let guide = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix guide docs", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(guide.status.success());
    let guide_json: Value = serde_json::from_slice(&guide.stdout).expect("valid json");
    assert_eq!(guide_json["domain"]["path"], "domains/guide");
    assert!(
        guide_json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/ui/**"))
    );

    let scoped_ui = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--path",
            "fixtures/mixed",
            "--task",
            "fix ui button",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start should run");
    assert!(scoped_ui.status.success());
    let scoped_ui_json: Value = serde_json::from_slice(&scoped_ui.stdout).expect("valid json");
    assert_eq!(
        scoped_ui_json["domain"]["path"],
        "fixtures/mixed/domains/ui"
    );
    assert!(
        scoped_ui_json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("fixtures/mixed/domains/guide/**"))
    );

    let scoped_guide = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--path",
            "fixtures/mixed",
            "--task",
            "fix guide docs",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start should run");
    assert!(scoped_guide.status.success());
    let scoped_guide_json: Value =
        serde_json::from_slice(&scoped_guide.stdout).expect("valid json");
    assert_eq!(
        scoped_guide_json["domain"]["path"],
        "fixtures/mixed/domains/guide"
    );
    assert!(
        scoped_guide_json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("fixtures/mixed/domains/ui/**"))
    );
}

#[test]
fn build_ci_tasks_read_build_surfaces_instead_of_empty_context() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"build-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps: []\n",
    );
    write(
        &repo.path().join("src/model.rs"),
        "pub struct DomainModel;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix build error", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["task_kind"], "build_ci");
    let read_first = json["read_first"].as_array().expect("read_first array");
    let read_paths: Vec<_> = read_first
        .iter()
        .filter_map(|item| item["path"].as_str())
        .collect();
    assert!(read_paths.contains(&".github/workflows/ci.yml"));
    assert!(read_paths.contains(&"Cargo.toml"));
    assert!(!read_paths.contains(&"src/model.rs"));
}

#[test]
fn build_ci_tasks_recognize_common_non_github_build_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"vitest","build":"vite build"}}"#,
    );
    write(
        &repo.path().join(".circleci/config.yml"),
        "version: 2.1\njobs:\n  test:\n    docker: []\n",
    );
    write(
        &repo.path().join("Jenkinsfile"),
        "pipeline { agent any; stages { stage('test') { steps { sh 'npm test' } } } }\n",
    );
    write(
        &repo.path().join("Taskfile.yml"),
        "version: '3'\ntasks: {}\n",
    );
    write(&repo.path().join("Taskfile"), "version: '3'\ntasks: {}\n");
    write(&repo.path().join("Dockerfile"), "FROM scratch\n");
    write(
        &repo.path().join("src/model.ts"),
        "export const model = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix ci pipeline", "--format", "json"])
        .output()
        .expect("ctx start should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["task_kind"], "build_ci");
    let read_first = json["read_first"].as_array().expect("read_first array");
    let read_paths: Vec<_> = read_first
        .iter()
        .filter_map(|item| item["path"].as_str())
        .collect();
    assert!(read_paths.contains(&".circleci/config.yml"));
    assert!(read_paths.contains(&"Jenkinsfile"));
    assert!(read_paths.contains(&"Taskfile"));
    assert!(read_paths.contains(&"Taskfile.yml"));
    assert!(read_paths.contains(&"Dockerfile"));
    assert!(read_paths.contains(&"package.json"));
    assert!(!read_paths.contains(&"src/model.ts"));
}

#[test]
fn pnpm_workspace_globs_create_domains_outside_builtin_dirs() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("pnpm-workspace.yaml"),
        "packages:\n  - workstreams/*\n",
    );
    write(
        &repo.path().join("workstreams/ledger/package.json"),
        r#"{"name":"@fixture/ledger"}"#,
    );
    write(
        &repo.path().join("workstreams/ledger/src/balance.ts"),
        "export function balance() { return 1; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["locate", "--task", "fix ledger balance", "--format", "json"])
        .output()
        .expect("ctx locate should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(
        json["candidates"][0]["domain"]["path"],
        "workstreams/ledger"
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
fn impact_names_public_schema_and_source_truth_expansion_triggers() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"echo test ok","typecheck":"echo typecheck ok"}}"#,
    );
    write(
        &repo.path().join("src/types.ts"),
        "export type ReplayDto = { frame: number };\n",
    );
    write(
        &repo.path().join("src/index.ts"),
        "export type { ReplayDto } from './types';\n",
    );
    write(
        &repo.path().join("src/timeline.ts"),
        "export const timeline = 1;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let schema_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "src/types.ts",
            "--depth",
            "1",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(schema_output.status.success());
    let schema_json: Value =
        serde_json::from_slice(&schema_output.stdout).expect("valid impact json");
    let triggers = schema_json["expansion_triggers"].as_array().unwrap();
    assert!(
        triggers
            .iter()
            .any(|trigger| trigger.as_str() == Some("DTO/schema contract changed"))
    );
    assert!(
        triggers
            .iter()
            .any(|trigger| trigger.as_str() == Some("impact reaches public boundary"))
    );

    let truth_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["impact", "--files", "src/timeline.ts", "--format", "json"])
        .output()
        .expect("ctx impact should run");
    assert!(truth_output.status.success());
    let truth_json: Value =
        serde_json::from_slice(&truth_output.stdout).expect("valid impact json");
    assert!(
        truth_json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|trigger| trigger.as_str() == Some("source of truth changed"))
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
fn unsupported_ctx_config_version_fails_closed_before_routing() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 2
concepts:
  replay.timeline:
    role: source_of_truth
    files:
      - src/replay-timeline.ts
"#,
    );
    write(
        &repo.path().join("src/replay-timeline.ts"),
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
    let json: Value = serde_json::from_slice(&validate.stdout).expect("valid json");
    assert_eq!(json["ok"], false);
    assert!(json["problems"].as_array().unwrap().iter().any(|problem| {
        problem
            .as_str()
            .unwrap()
            .contains("unsupported .ctx version")
    }));

    let start = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix replay timeline"])
        .output()
        .expect("ctx start should run");
    assert!(!start.status.success());
    let stderr = String::from_utf8(start.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("unsupported .ctx version"));
}

#[test]
fn unknown_ctx_config_fields_fail_closed_before_routing() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
task_routez:
  typo:
    match:
      - seek
"#,
    );
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
            .any(|problem| problem.as_str().unwrap().contains("task_routez"))
    );

    let start = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix demo"])
        .output()
        .expect("ctx start should run");
    assert!(!start.status.success());
}

#[test]
fn invalid_semantic_anchors_fail_closed_before_routing() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
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
      - domains/replay/src/missing-timeline.ts
boundaries:
  forbidden:
    - from: domains/replay/src/**
      to: domains/renderer/src/**
task_routes:
  playback_session:
    match:
      - seek
    read_first:
      - domains/replay/src/missing-session.ts
"#,
    );
    write(
        &repo.path().join("domains/replay/src/replay-session.ts"),
        "export const session = 1;\n",
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
    let json: Value = serde_json::from_slice(&validate.stdout).expect("valid json");
    assert_eq!(json["ok"], false);
    let problems = json["problems"].as_array().unwrap();
    assert!(
        problems
            .iter()
            .any(|problem| problem.as_str().unwrap().contains("missing-timeline.ts"))
    );
    assert!(
        problems
            .iter()
            .any(|problem| problem.as_str().unwrap().contains("missing `reason`"))
    );
    assert!(
        problems
            .iter()
            .any(|problem| problem.as_str().unwrap().contains("missing-session.ts"))
    );

    let start = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["start", "--task", "fix seek frame"])
        .output()
        .expect("ctx start should run");
    assert!(!start.status.success());
    let stderr = String::from_utf8(start.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("missing-timeline.ts"));
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
fn transitive_package_manifest_boundary_edge_fails_closed_when_changed() {
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
    "@fixture/timeline": "workspace:*"
  }
}"#,
    );
    write(
        &repo.path().join("domains/timeline/package.json"),
        r#"{"name":"@fixture/timeline"}"#,
    );
    write(
        &repo.path().join("domains/renderer/package.json"),
        r#"{"name":"@fixture/renderer"}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let clean = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["boundaries", "--changed"])
        .output()
        .expect("ctx boundaries should run");
    assert!(clean.status.success());

    write(
        &repo.path().join("domains/timeline/package.json"),
        r#"{
  "name": "@fixture/timeline",
  "dependencies": {
    "@fixture/renderer": "workspace:*"
  }
}"#,
    );
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["boundaries", "--changed"])
        .output()
        .expect("ctx boundaries should run");
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("transitive package manifest dependency path"));
    assert!(stdout.contains("@fixture/timeline -> @fixture/renderer"));
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
        "schemas/status.schema.json",
        "schemas/files.schema.json",
        "schemas/capsule.schema.json",
        "schemas/impact.schema.json",
        "schemas/verify.schema.json",
        "schemas/anchors.schema.json",
        "schemas/locate.schema.json",
        "schemas/explain.schema.json",
        "schemas/widen.schema.json",
        "schemas/graph.schema.json",
        "schemas/boundaries.schema.json",
    ] {
        let text = fs::read_to_string(root.join(rel)).expect("schema should exist");
        let json: Value = serde_json::from_str(&text).expect("schema should be valid json");
        assert_eq!(
            json["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(!json["required"].as_array().unwrap().is_empty());
    }
    let anchors_instance = serde_json::json!({
        "version": 1,
        "domain": {
            "id": "replay",
            "path": "domains/replay"
        },
        "concepts": {
            "replay.timeline": {
                "role": "source_of_truth",
                "files": ["src/replay-timeline.ts"]
            }
        },
        "boundaries": {
            "forbidden": [{
                "from": "domains/replay/src/**",
                "to": "domains/renderer/src/**",
                "reason": "replay emits DTOs; renderer consumes DTOs"
            }]
        },
        "task_routes": {
            "playback": {
                "match": ["frame", "seek"],
                "read_first": ["src/replay-session.ts"],
                "verify": ["pnpm test domains/replay -- session"]
            }
        }
    });
    assert_schema_accepts("schemas/anchors.schema.json", &anchors_instance);

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

    let status = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["status", "--format", "json"])
        .output()
        .expect("ctx status should run");
    assert!(status.status.success());
    let status_json: Value = serde_json::from_slice(&status.stdout).expect("valid status json");
    assert_schema_accepts("schemas/status.schema.json", &status_json);

    let files = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["files", "--limit", "5", "--format", "json"])
        .output()
        .expect("ctx files should run");
    assert!(files.status.success());
    let files_json: Value = serde_json::from_slice(&files.stdout).expect("valid files json");
    assert_schema_accepts("schemas/files.schema.json", &files_json);

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

    let locate = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["locate", "--task", "fix broken save", "--format", "json"])
        .output()
        .expect("ctx locate should run");
    assert!(locate.status.success());
    let locate_json: Value = serde_json::from_slice(&locate.stdout).expect("valid locate json");
    assert_schema_accepts("schemas/locate.schema.json", &locate_json);

    let explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["explain", "src/save.ts", "--format", "json"])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    assert_schema_accepts("schemas/explain.schema.json", &explain_json);

    let widen = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "widen",
            "--task",
            "fix broken save",
            "--reason",
            "read-first set did not contain the cause",
            "--already",
            "src/save.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx widen should run");
    assert!(widen.status.success());
    let widen_json: Value = serde_json::from_slice(&widen.stdout).expect("valid widen json");
    assert_schema_accepts("schemas/widen.schema.json", &widen_json);

    let graph = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["graph", "--lens", "causal", "--format", "json"])
        .output()
        .expect("ctx graph should run");
    assert!(graph.status.success());
    let graph_json: Value = serde_json::from_slice(&graph.stdout).expect("valid graph json");
    assert_schema_accepts("schemas/graph.schema.json", &graph_json);

    let boundaries = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["boundaries", "--format", "json"])
        .output()
        .expect("ctx boundaries should run");
    assert!(boundaries.status.success());
    let boundaries_json: Value =
        serde_json::from_slice(&boundaries.stdout).expect("valid boundaries json");
    assert_schema_accepts("schemas/boundaries.schema.json", &boundaries_json);
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
fn general_low_confidence_task_gets_bounded_orientation_route() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"ctx-demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("src/model.rs"), "pub struct Model;\n");
    write(&repo.path().join("src/cli.rs"), "pub fn run() {}\n");
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--task",
            "continue implementation until complete",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["task_kind"], "general");
    assert_eq!(json["confidence"], "low");
    let read_first = json["read_first"].as_array().expect("read_first array");
    assert!(
        !read_first.is_empty(),
        "general low-confidence tasks still need a bounded orientation route"
    );
    assert!(read_first.len() <= 7);
    assert!(
        read_first
            .iter()
            .any(|item| item["path"].as_str() == Some("src/model.rs"))
    );
    assert!(
        json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|trigger| trigger.as_str() == Some("context confidence is medium/low"))
    );
}

#[test]
fn start_does_not_route_into_top_level_fixtures_by_default() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("fixtures/replay/src/replay-session.ts"),
        "export const replaySession = 1;\n",
    );
    write(
        &repo.path().join("fixtures/replay/src/replay-timeline.ts"),
        "export const replayTimeline = 1;\n",
    );
    write(
        &repo.path().join("examples/replay/src/replay-session.ts"),
        "export const replayExample = 1;\n",
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed/domains/replay/package.json"),
        r#"{"name":"@fixture/replay"}"#,
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed/domains/replay/src/replay-session.ts"),
        "export const replaySession = 1;\n",
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed/domains/replay/tests/replay-session.test.ts"),
        "import { replaySession } from '../src/replay-session';\nconsole.log(replaySession);\n",
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed/services/auth/package.json"),
        r#"{"name":"@fixture/auth"}"#,
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed/services/auth/src/session.ts"),
        "export const authSession = 1;\n",
    );
    write(
        &repo
            .path()
            .join("fixtures/mixed/services/auth/tests/session.test.ts"),
        "import { authSession } from '../src/session';\nconsole.log(authSession);\n",
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
    assert!(json["read_first"].as_array().unwrap().iter().all(|item| {
        let path = item["path"].as_str().unwrap_or("");
        !path.starts_with("fixtures/") && !path.starts_with("examples/")
    }));
    assert!(
        json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("fixtures/**"))
    );
    assert!(
        json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("examples/**"))
    );

    let graph_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["graph", "--lens", "causal", "--format", "json"])
        .output()
        .expect("ctx graph should run");
    assert!(graph_output.status.success());
    let graph_json: Value = serde_json::from_slice(&graph_output.stdout).expect("valid graph json");
    assert!(graph_json["nodes"].as_array().unwrap().iter().all(|item| {
        let path = item.as_str().unwrap_or("");
        !path.starts_with("fixtures/") && !path.starts_with("examples/")
    }));

    let locate = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "locate",
            "--task",
            "fix replay jumping to wrong frame after seek",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx locate should run");
    assert!(locate.status.success());
    let locate_json: Value = serde_json::from_slice(&locate.stdout).expect("valid locate json");
    let first_candidate = &locate_json["candidates"][0];
    assert_eq!(first_candidate["task_kind"], "playback_session");
    assert_ne!(first_candidate["confidence"], "high");
    assert!(
        first_candidate["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str() == Some("no task-specific file evidence found"))
    );

    let fixture_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--task",
            "fix replay fixture seek behavior",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start fixture should run");
    assert!(fixture_output.status.success());
    let fixture_json: Value =
        serde_json::from_slice(&fixture_output.stdout).expect("valid fixture json");
    assert!(
        fixture_json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"]
                .as_str()
                .unwrap_or("")
                .starts_with("fixtures/replay/"))
    );
    assert!(
        fixture_json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["path"].as_str() != Some("fixtures/**"))
    );

    let nested_fixture_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--path",
            "fixtures/mixed",
            "--task",
            "fix replay fixture seek behavior",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start nested fixture should run");
    assert!(nested_fixture_output.status.success());
    let nested_fixture_json: Value =
        serde_json::from_slice(&nested_fixture_output.stdout).expect("valid nested fixture json");
    assert_eq!(
        nested_fixture_json["domain"]["path"],
        "fixtures/mixed/domains/replay"
    );
    assert!(
        nested_fixture_json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| {
                item["path"]
                    .as_str()
                    .unwrap_or("")
                    .starts_with("fixtures/mixed/domains/replay/")
            })
    );
    assert!(
        nested_fixture_json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("fixtures/mixed/services/auth/**"))
    );
    assert!(
        nested_fixture_json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["path"].as_str() != Some("."))
    );

    let nested_file_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--path",
            "fixtures/mixed/domains/replay/src/replay-session.ts",
            "--task",
            "fix replay fixture seek behavior",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start nested fixture file should run");
    assert!(nested_file_output.status.success());
    let nested_file_json: Value =
        serde_json::from_slice(&nested_file_output.stdout).expect("valid nested file json");
    assert_eq!(
        nested_file_json["domain"]["path"],
        "fixtures/mixed/domains/replay"
    );

    let nested_auth_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--path",
            "fixtures/mixed",
            "--task",
            "fix auth fixture session behavior",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start nested auth fixture should run");
    assert!(nested_auth_output.status.success());
    let nested_auth_json: Value =
        serde_json::from_slice(&nested_auth_output.stdout).expect("valid nested auth json");
    assert_eq!(
        nested_auth_json["domain"]["path"],
        "fixtures/mixed/services/auth"
    );
    assert!(
        nested_auth_json["related_tests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("fixtures/mixed/services/auth/tests/session.test.ts"))
    );
    assert!(
        nested_auth_json["related_tests"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| !item
                .as_str()
                .unwrap_or("")
                .starts_with("fixtures/mixed/domains/replay/"))
    );

    let example_output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--task",
            "fix replay example seek behavior",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx start example should run");
    assert!(example_output.status.success());
    let example_json: Value =
        serde_json::from_slice(&example_output.stdout).expect("valid example json");
    assert!(
        example_json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"]
                .as_str()
                .unwrap_or("")
                .starts_with("examples/replay/"))
    );
    assert!(
        example_json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["path"].as_str() != Some("examples/**"))
    );
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
fn absolute_path_commands_select_target_repo_from_any_cwd() {
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
    let absolute_src = repo.path().join("src");
    let absolute_file = repo.path().join("src/save.ts");

    let files = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "files",
            "--path",
            absolute_src.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("ctx files should run");
    assert!(files.status.success());
    let files_json: Value = serde_json::from_slice(&files.stdout).expect("valid files json");
    assert_eq!(files_json["path"], "src");
    assert!(
        files_json["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item.as_str() == Some("src/save.ts") })
    );

    let explain = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "explain",
            absolute_file.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    assert_eq!(explain_json["path"], "src/save.ts");

    let graph = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "graph",
            "--path",
            absolute_src.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("ctx graph should run");
    assert!(graph.status.success());
    let graph_json: Value = serde_json::from_slice(&graph.stdout).expect("valid graph json");
    assert!(
        graph_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| { item.as_str() == Some("src/save.ts") })
    );

    let absolute_domain = repo.path().join("domains/replay");
    let init = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "init",
            "--write-minimal",
            "--path",
            absolute_domain.to_str().unwrap(),
        ])
        .output()
        .expect("ctx init should run");
    assert!(init.status.success());
    assert!(repo.path().join("domains/replay/.ctx.yml").exists());
    assert!(!outside.path().join(".ctx.yml").exists());
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
