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
        let file_name = entry.file_name();
        let target_name = if file_name == "Cargo.toml.fixture" {
            "Cargo.toml".into()
        } else {
            file_name
        };
        let target = to.join(target_name);
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
fn mixed_monorepo_root_path_still_uses_task_routing() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "start",
            "--path",
            repo.path().to_str().unwrap(),
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
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("domains/replay/src/replay-session.ts"))
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

#[test]
fn rust_workspace_routes_replay_task_to_replay_crate() {
    let repo = fixture_copy("rust-workspace");
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
    assert_eq!(json["domain"]["path"], "crates/replay");
    assert_eq!(json["task_kind"], "playback_session");
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("crates/replay/src/session.rs"))
    );
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("crates/replay/src/timeline.rs"))
    );
    assert!(
        json["related_tests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("crates/replay/tests/session_test.rs"))
    );
    assert!(
        json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("crates/renderer/**"))
    );
}

#[test]
fn rust_workspace_cargo_table_dependencies_feed_impact_and_boundaries() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "crates/replay/Cargo.toml",
            "--depth",
            "2",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(impact.status.success());
    let impact_json: Value = serde_json::from_slice(&impact.stdout).expect("valid impact json");
    assert!(
        impact_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("crates/renderer/Cargo.toml")),
        "renderer consumes replay through a Cargo table dependency"
    );
    assert!(
        impact_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("crates/app/Cargo.toml")),
        "app consumes renderer and should be reached at depth 2"
    );
    assert!(
        impact_json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("package consumers affected"))
    );

    fs::write(
        repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: crates/renderer/src/**
      to: crates/replay/src/**
      reason: renderer must not depend on replay in this fixture policy
"#,
    )
    .expect("write ctx config");
    let boundaries = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["boundaries", "--format", "json"])
        .output()
        .expect("ctx boundaries should run");
    assert!(!boundaries.status.success());
    let boundaries_json: Value =
        serde_json::from_slice(&boundaries.stdout).expect("valid boundaries json");
    assert!(
        boundaries_json["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["from"].as_str() == Some("crates/renderer/Cargo.toml")
                    && finding["to"].as_str() == Some("crates/replay/Cargo.toml")
                    && finding["provenance"].as_str() == Some("package_manifest+ctx_anchor")
                    && finding["reason"]
                        .as_str()
                        .unwrap_or("")
                        .contains("Cargo.toml path dependency")
            })
    );
}
