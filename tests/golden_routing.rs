use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn ctx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ctx"))
}

fn fixture(name: &str) -> std::path::PathBuf {
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

#[test]
fn mixed_monorepo_routes_replay_task_to_replay_domain() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
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
    assert_eq!(json["domain"]["path"], "domains/replay");
    assert_eq!(json["task_kind"], "playback_session");
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/replay/src/replay-session.ts"))
    );
    assert!(
        !json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/replay/package.json"))
    );
    assert!(
        json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/renderer/**"))
    );
}

#[test]
fn mixed_monorepo_locates_auth_token_task() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "locate",
            "--task",
            "fix auth token refresh",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx locate should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(json["candidates"][0]["domain"]["path"], "services/auth");
    assert_eq!(json["candidates"][0]["task_kind"], "auth");
}

#[test]
fn mixed_monorepo_impact_keeps_replay_scope_bounded() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "domains/replay/src/replay-timeline.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/src/replay-session.ts"))
    );
    assert!(
        json["related_tests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/tests/replay-session.test.ts"))
    );
    assert!(
        !json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap_or("").starts_with("apps/web/"))
    );
}

#[test]
fn mixed_monorepo_impact_expands_package_consumers_for_public_boundary_change() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "domains/replay/package.json",
            "--depth",
            "2",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/renderer/package.json"))
    );
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("apps/web/package.json"))
    );
    assert!(
        json["external_domains"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/renderer"))
    );
    assert!(
        json["external_domains"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("apps/web"))
    );
    assert!(
        json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("package consumers affected"))
    );
}

#[test]
fn mixed_monorepo_impact_expands_package_consumers_when_internal_change_reaches_public_boundary() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "services/auth/src/token.ts",
            "--depth",
            "2",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("services/auth/src/index.ts"))
    );
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("apps/web/package.json"))
    );
    assert!(
        json["external_domains"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("apps/web"))
    );
    assert!(
        !json["related_tests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/tests/replay-session.test.ts"))
    );
}
