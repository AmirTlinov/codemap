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
        let target_name = match file_name.to_str() {
            Some("Cargo.toml.fixture") => "Cargo.toml".into(),
            Some("go.mod.fixture") => "go.mod".into(),
            Some("go.work.fixture") => "go.work".into(),
            _ => file_name,
        };
        let target = to.join(target_name);
        if source.is_dir() {
            copy_dir(&source, &target);
        } else {
            fs::copy(&source, &target).expect("copy fixture file");
        }
    }
}

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write file");
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should run");
    assert!(status.success(), "git {:?} failed", args);
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
fn mixed_monorepo_resolves_js_workspace_package_imports_in_file_graph() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");

    let explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["explain", "apps/web/src/app.ts", "--format", "json"])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    let imports = explain_json["imports"].as_array().unwrap();
    assert!(
        imports
            .iter()
            .any(|item| item.as_str() == Some("domains/renderer/src/replay-renderer.ts")),
        "package import @ctx-fixture/renderer should resolve to its exported file"
    );
    assert!(
        imports
            .iter()
            .any(|item| item.as_str() == Some("services/auth/src/index.ts")),
        "package import @ctx-fixture/auth should resolve to src/index.ts"
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "domains/renderer/src/replay-renderer.ts",
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
            .any(|item| item.as_str() == Some("apps/web/src/app.ts")),
        "file-level impact should reach workspace package consumers"
    );
}

#[test]
fn mixed_monorepo_resolves_tsconfig_path_aliases_in_file_graph() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");

    let explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["explain", "apps/web/src/auth-alias.ts", "--format", "json"])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    assert!(
        explain_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("services/auth/src/session.ts"))
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "services/auth/src/session.ts",
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
            .any(|item| item.as_str() == Some("apps/web/src/auth-alias.ts"))
    );
}

#[test]
fn tsconfig_path_aliases_are_scoped_to_config_directory() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("apps/web/tsconfig.json"),
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@local/*": ["src/*"]
    }
  }
}"#,
    );
    write(
        &repo.path().join("apps/web/src/local-util.ts"),
        "export const localUtil = 1;\n",
    );
    write(
        &repo.path().join("services/auth/src/local-consumer.ts"),
        "import { localUtil } from '@local/local-util';\nexport const value = localUtil;\n",
    );

    let explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "explain",
            "services/auth/src/local-consumer.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    assert!(
        explain_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("apps/web/src/local-util.ts")),
        "aliases from apps/web/tsconfig.json must not resolve imports in services/auth"
    );
}

#[test]
fn nested_tsconfig_path_aliases_override_root_aliases_for_local_importers() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("package.json"),
        r#"{"workspaces":["z/*"]}"#,
    );
    write(
        &repo.path().join("tsconfig.json"),
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@shared/*": ["shared/root/src/*"]
    }
  }
}"#,
    );
    write(
        &repo.path().join("z/app/tsconfig.json"),
        r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@shared/*": ["local/src/*"]
    }
  }
}"#,
    );
    write(
        &repo.path().join("shared/root/src/value.ts"),
        "export const value = 'root';\n",
    );
    write(
        &repo.path().join("z/app/local/src/value.ts"),
        "export const value = 'local';\n",
    );
    write(
        &repo.path().join("z/app/src/consumer.ts"),
        "import { value } from '@shared/value';\nexport const consumer = value;\n",
    );

    let explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["explain", "z/app/src/consumer.ts", "--format", "json"])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    assert!(
        explain_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("z/app/local/src/value.ts"))
    );
    assert!(
        explain_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("shared/root/src/value.ts"))
    );
}

#[test]
fn package_exports_subpaths_resolve_and_block_unexported_root_fallback() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("package.json"),
        r#"{"workspaces":["packages/*","apps/*"]}"#,
    );
    write(
        &repo.path().join("packages/core/package.json"),
        r#"{
  "name": "@demo/core",
  "exports": {
    "./foo": "./src/public/foo.ts"
  }
}"#,
    );
    write(
        &repo.path().join("packages/core/src/public/foo.ts"),
        "export const foo = 1;\n",
    );
    write(
        &repo.path().join("packages/core/src/index.ts"),
        "export const hidden = 1;\n",
    );
    write(
        &repo.path().join("apps/web/package.json"),
        r#"{"name":"web","dependencies":{"@demo/core":"workspace:*"}}"#,
    );
    write(
        &repo.path().join("apps/web/src/app.ts"),
        "import { foo } from '@demo/core/foo';\nexport const value = foo;\n",
    );
    write(
        &repo.path().join("apps/web/src/root-import.ts"),
        "import { hidden } from '@demo/core';\nexport const value = hidden;\n",
    );

    let subpath = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["explain", "apps/web/src/app.ts", "--format", "json"])
        .output()
        .expect("ctx explain should run");
    assert!(subpath.status.success());
    let subpath_json: Value = serde_json::from_slice(&subpath.stdout).expect("valid explain json");
    assert!(
        subpath_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("packages/core/src/public/foo.ts"))
    );

    let root_import = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["explain", "apps/web/src/root-import.ts", "--format", "json"])
        .output()
        .expect("ctx explain should run");
    assert!(root_import.status.success());
    let root_json: Value = serde_json::from_slice(&root_import.stdout).expect("valid explain json");
    assert!(
        root_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("packages/core/src/index.ts")),
        "package exports without `.` must not fall back to src/index.ts"
    );
}

#[test]
fn graph_boundaries_lens_renders_forbidden_package_edges() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: apps/web/package.json
      to: domains/renderer/package.json
      reason: app must not package-depend on renderer in this fixture policy
"#,
    );

    let graph = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["graph", "--lens", "boundaries", "--format", "json"])
        .output()
        .expect("ctx graph should run");
    assert!(graph.status.success());
    let graph_json: Value = serde_json::from_slice(&graph.stdout).expect("valid graph json");
    assert!(
        graph_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("apps/web/package.json"))
    );
    assert!(
        graph_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/renderer/package.json"))
    );
    assert!(
        graph_json["edges"].as_array().unwrap().iter().any(|edge| {
            edge["from"].as_str() == Some("apps/web/package.json")
                && edge["to"].as_str() == Some("domains/renderer/package.json")
                && edge["type"].as_str() == Some("forbidden")
        }),
        "boundaries lens should render explicit/package findings as graph edges"
    );
}

#[test]
fn graph_boundaries_changed_lens_respects_changed_scope() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: apps/web/package.json
      to: domains/renderer/package.json
      reason: app must not package-depend on renderer in this fixture policy
"#,
    );
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);
    write(
        &repo.path().join("services/auth/src/token.ts"),
        "export function token() { return 'changed'; }\n",
    );

    let unrelated = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "graph",
            "--lens",
            "boundaries",
            "--changed",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx graph should run");
    assert!(unrelated.status.success());
    let unrelated_json: Value =
        serde_json::from_slice(&unrelated.stdout).expect("valid graph json");
    assert_eq!(
        unrelated_json["nodes"].as_array().unwrap().len(),
        0,
        "changed-only boundary graph must not show unrelated committed findings"
    );
    assert_eq!(unrelated_json["edges"].as_array().unwrap().len(), 0);

    write(
        &repo.path().join("domains/renderer/package.json"),
        r#"{
  "name": "@ctx-fixture/renderer",
  "private": true,
  "exports": "./src/replay-renderer.ts",
  "dependencies": {
    "@ctx-fixture/replay": "workspace:*"
  },
  "description": "changed target manifest"
}
"#,
    );

    let target_changed = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "graph",
            "--lens",
            "boundaries",
            "--changed",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx graph should run");
    assert!(target_changed.status.success());
    let graph_json: Value =
        serde_json::from_slice(&target_changed.stdout).expect("valid graph json");
    assert!(
        graph_json["edges"].as_array().unwrap().iter().any(|edge| {
            edge["from"].as_str() == Some("apps/web/package.json")
                && edge["to"].as_str() == Some("domains/renderer/package.json")
                && edge["type"].as_str() == Some("forbidden")
        }),
        "changed target manifests should seed changed-only boundary graph findings"
    );
}

#[test]
fn graph_verification_lens_connects_changed_files_tests_and_commands() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);
    write(
        &repo.path().join("domains/replay/src/replay-timeline.ts"),
        "export function frameAt(timeMs: number): number {\n  return Math.max(0, Math.floor(timeMs / 16));\n}\n",
    );

    let graph = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "graph",
            "--lens",
            "verification",
            "--changed",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx graph should run");
    assert!(graph.status.success());
    let graph_json: Value = serde_json::from_slice(&graph.stdout).expect("valid graph json");
    assert_eq!(graph_json["domain"]["path"], "domains/replay");
    assert!(
        graph_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/src/replay-timeline.ts"))
    );
    assert!(
        graph_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/tests/replay-session.test.ts"))
    );
    assert!(
        graph_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("$ pnpm test domains/replay"))
    );
    assert!(graph_json["edges"].as_array().unwrap().iter().any(|edge| {
        edge["from"].as_str() == Some("domains/replay/src/replay-session.ts")
            && edge["to"].as_str() == Some("domains/replay/src/replay-timeline.ts")
            && edge["type"].as_str() == Some("imports")
    }));
    assert!(graph_json["edges"].as_array().unwrap().iter().any(|edge| {
        edge["from"].as_str() == Some("domains/replay/src/replay-session.ts")
            && edge["to"].as_str() == Some("domains/replay/tests/replay-session.test.ts")
            && edge["type"].as_str() == Some("tested_by")
    }));
    assert!(graph_json["edges"].as_array().unwrap().iter().any(|edge| {
        edge["from"].as_str() == Some("domains/replay/tests/replay-session.test.ts")
            && edge["to"].as_str() == Some("$ pnpm test domains/replay")
            && edge["type"].as_str() == Some("verified_by")
    }));
    assert!(
        graph_json["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|edge| edge["from"] != edge["to"]),
        "verification graph must not emit self-loop edges"
    );
}

#[test]
fn graph_file_path_seeds_impact_and_verification_lenses() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "graph",
            "--lens",
            "impact",
            "--path",
            "domains/replay/src/replay-timeline.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx graph impact should run");
    assert!(impact.status.success());
    let impact_json: Value = serde_json::from_slice(&impact.stdout).expect("valid impact graph");
    assert!(
        impact_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/src/replay-timeline.ts")),
        "file-level impact graph should include the exact requested file"
    );
    assert!(
        impact_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/src/replay-session.ts")),
        "file-level impact graph should include file importers"
    );

    let verification = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "graph",
            "--lens",
            "verification",
            "--path",
            "domains/replay/src/replay-timeline.ts",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx graph verification should run");
    assert!(verification.status.success());
    let verification_json: Value =
        serde_json::from_slice(&verification.stdout).expect("valid verification graph");
    assert!(
        verification_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/src/replay-timeline.ts")),
        "file-level verification graph should include the exact requested file"
    );
    assert!(
        verification_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("domains/replay/tests/replay-session.test.ts"))
    );
    assert!(
        verification_json["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("$ pnpm test domains/replay"))
    );
}

#[test]
fn graph_changed_lenses_do_not_invent_context_when_changed_set_is_empty() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "init"]);

    for lens in ["verification", "impact"] {
        let graph = ctx()
            .current_dir(repo.path())
            .env("CTX_CACHE_DIR", cache.path())
            .args(["graph", "--lens", lens, "--changed", "--format", "json"])
            .output()
            .unwrap_or_else(|error| panic!("ctx graph {lens} should run: {error}"));
        assert!(graph.status.success(), "ctx graph {lens} should succeed");
        let graph_json: Value = serde_json::from_slice(&graph.stdout).expect("valid graph json");
        assert_eq!(
            graph_json["nodes"].as_array().unwrap().len(),
            0,
            "{lens} graph should stay empty for an explicitly empty changed set"
        );
        assert_eq!(
            graph_json["edges"].as_array().unwrap().len(),
            0,
            "{lens} graph should not synthesize edges for an empty changed set"
        );
    }
}

#[test]
fn graph_impact_lens_without_changed_input_stays_empty() {
    let repo = fixture_copy("mixed-monorepo");
    let cache = TempDir::new().expect("cache tempdir");

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["graph", "--lens", "impact", "--format", "json"])
        .output()
        .expect("ctx graph impact should run");
    assert!(impact.status.success());
    let impact_json: Value = serde_json::from_slice(&impact.stdout).expect("valid impact graph");
    assert_eq!(
        impact_json["nodes"].as_array().unwrap().len(),
        0,
        "impact lens should not fall back to causal context without changed input"
    );
    assert_eq!(impact_json["edges"].as_array().unwrap().len(), 0);

    let verification = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["graph", "--lens", "verification", "--format", "json"])
        .output()
        .expect("ctx graph verification should run");
    assert!(verification.status.success());
    let verification_json: Value =
        serde_json::from_slice(&verification.stdout).expect("valid verification graph");
    assert!(
        !verification_json["nodes"].as_array().unwrap().is_empty(),
        "verification lens keeps its no-changed general orientation graph"
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

#[test]
fn rust_workspace_dependencies_feed_package_impact_and_boundaries() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = [
  "crates/app",
  "crates/renderer",
  "crates/replay",
]
resolver = "3"

[workspace.dependencies.ctx_fixture_replay]
path = "crates/replay" # valid TOML comment
"#,
    );
    write(
        &repo.path().join("crates/renderer/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_renderer"
version = "0.1.0"
edition = "2024"

[dependencies.ctx_fixture_replay]
workspace = true # valid TOML comment
"#,
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "crates/replay/Cargo.toml",
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
        "renderer consumes replay through a workspace dependency"
    );
    assert!(
        impact_json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("package consumers affected"))
    );
    let workspace_impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["impact", "--files", "Cargo.toml", "--format", "json"])
        .output()
        .expect("ctx impact should run");
    assert!(workspace_impact.status.success());
    let workspace_json: Value =
        serde_json::from_slice(&workspace_impact.stdout).expect("valid impact json");
    assert!(
        workspace_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("crates/renderer/Cargo.toml")),
        "workspace dependency declaration changes should reach workspace=true consumers"
    );
    assert!(
        workspace_json["expansion_triggers"]
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
    git(repo.path(), &["init"]);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-m", "baseline"]);
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = [
  "crates/app",
  "crates/renderer",
  "crates/replay",
]
resolver = "3"

[workspace.dependencies.ctx_fixture_replay]
path = "crates/replay"
"#,
    );
    let changed_boundaries = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["boundaries", "--changed", "--format", "json"])
        .output()
        .expect("ctx boundaries should run");
    assert!(!changed_boundaries.status.success());
    let changed_boundaries_json: Value =
        serde_json::from_slice(&changed_boundaries.stdout).expect("valid boundaries json");
    assert!(
        changed_boundaries_json["findings"]
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
                        .contains("Cargo.toml workspace dependency")
            }),
        "changed-scoped boundaries should treat workspace manifest changes as touching the edge"
    );

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
                        .contains("Cargo.toml workspace dependency")
            })
    );
}

#[test]
fn rust_workspace_metadata_dependencies_are_not_package_edges() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = [
  "crates/app",
  "crates/renderer",
  "crates/replay",
]
resolver = "3"

[workspace.dependencies.ctx_fixture_replay]
path = "crates/replay"
"#,
    );
    write(
        &repo.path().join("crates/renderer/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_renderer"
version = "0.1.0"
edition = "2024"

[package.metadata.fake.dependencies.ctx_fixture_replay]
workspace = true

[target.'cfg(unix)'.package.metadata.fake.dependencies.ctx_fixture_replay]
workspace = true
"#,
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "crates/replay/Cargo.toml",
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
            .all(|item| item.as_str() != Some("crates/renderer/Cargo.toml")),
        "Cargo package metadata tables must not create package dependency edges"
    );
    assert!(
        impact_json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("package consumers affected"))
    );
}

#[test]
fn rust_workspace_dependencies_do_not_escape_repo_root() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = [
  "crates/app",
  "external",
]
resolver = "3"

[workspace.dependencies.ctx_fixture_external]
path = "../external"
"#,
    );
    write(
        &repo.path().join("crates/app/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_app"
version = "0.1.0"
edition = "2024"

[dependencies.ctx_fixture_external]
workspace = true
"#,
    );
    write(
        &repo.path().join("external/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_external"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(
        &repo.path().join("external/src/lib.rs"),
        "pub fn value() -> usize { 1 }\n",
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "external/Cargo.toml",
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
            .all(|item| item.as_str() != Some("crates/app/Cargo.toml")),
        "Cargo workspace dependency paths that escape the repo root must not be remapped inside the repo"
    );
    assert!(
        impact_json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("package consumers affected"))
    );

    fs::write(
        repo.path().join(".ctx.yml"),
        r#"version: 1
boundaries:
  forbidden:
    - from: crates/app/src/**
      to: external/src/**
      reason: app must not depend on external in this fixture policy
"#,
    )
    .expect("write ctx config");
    let boundaries = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["boundaries", "--format", "json"])
        .output()
        .expect("ctx boundaries should run");
    assert!(
        boundaries.status.success(),
        "outside-repo Cargo paths must not create false package boundary findings: {}",
        String::from_utf8_lossy(&boundaries.stdout)
    );
}

#[test]
fn rust_workspace_dependencies_are_scoped_to_nearest_workspace() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = [
  "crates/app",
  "crates/renderer",
  "crates/replay",
]
resolver = "3"

[workspace.dependencies.ctx_fixture_replay]
path = "crates/replay"
"#,
    );
    write(
        &repo.path().join("crates/renderer/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_renderer"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(
        &repo.path().join("crates/app/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_app"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(
        &repo.path().join("nested/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_nested"
version = "0.1.0"
edition = "2024"

[workspace]
members = ["deps/replay"]
resolver = "3"

[workspace.dependencies.ctx_fixture_replay]
path = "deps/replay"

[dependencies.ctx_fixture_replay]
workspace = true
"#,
    );
    write(
        &repo.path().join("nested/src/lib.rs"),
        "pub fn nested() -> usize { ctx_fixture_replay::value() }\n",
    );
    write(
        &repo.path().join("nested/deps/replay/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_replay"
version = "0.1.0"
edition = "2024"
"#,
    );
    write(
        &repo.path().join("nested/deps/replay/src/lib.rs"),
        "pub fn value() -> usize { 1 }\n",
    );

    let root_impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "crates/replay/Cargo.toml",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(root_impact.status.success());
    let root_json: Value = serde_json::from_slice(&root_impact.stdout).expect("valid json");
    assert!(
        root_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("nested/Cargo.toml")),
        "nested package must not consume the root workspace dependency"
    );
    assert!(
        root_json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("package consumers affected"))
    );

    let nested_impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "nested/deps/replay/Cargo.toml",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(nested_impact.status.success());
    let nested_json: Value = serde_json::from_slice(&nested_impact.stdout).expect("valid json");
    assert!(
        nested_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("nested/Cargo.toml")),
        "nested package should consume its own nearest workspace dependency"
    );
    assert!(
        nested_json["expansion_triggers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("package consumers affected"))
    );
}

#[test]
fn rust_workspace_member_globs_feed_package_impact() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = [
  "crates/*/app",
  "crates/replay",
]
resolver = "3"

[workspace.dependencies.ctx_fixture_replay]
path = "crates/replay"
"#,
    );
    write(
        &repo.path().join("crates/group/app/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_group_app"
version = "0.1.0"
edition = "2024"

[dependencies.ctx_fixture_replay]
workspace = true
"#,
    );
    write(
        &repo.path().join("crates/group/app/src/lib.rs"),
        "pub fn value() -> usize { ctx_fixture_replay::value() }\n",
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "crates/replay/Cargo.toml",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(impact.status.success());
    let json: Value = serde_json::from_slice(&impact.stdout).expect("valid json");
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("crates/group/app/Cargo.toml")),
        "Cargo workspace member globs should inherit workspace dependencies"
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
fn rust_workspace_path_dependencies_become_members() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = ["crates/app"]
resolver = "3"

[workspace.dependencies.ctx_fixture_replay]
path = "crates/replay"
"#,
    );
    write(
        &repo.path().join("crates/app/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_app"
version = "0.1.0"
edition = "2024"

[dependencies.ctx_fixture_renderer]
path = "../renderer"
"#,
    );
    write(
        &repo.path().join("crates/renderer/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_renderer"
version = "0.1.0"
edition = "2024"

[dependencies.ctx_fixture_replay]
workspace = true
"#,
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "crates/replay/Cargo.toml",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(impact.status.success());
    let json: Value = serde_json::from_slice(&impact.stdout).expect("valid json");
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("crates/renderer/Cargo.toml")),
        "Cargo path dependencies inside a workspace should become workspace members"
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
fn rust_workspace_dotted_cargo_syntax_feeds_package_impact() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("Cargo.toml"),
        r#"workspace.members = ["crates/app", "crates/replay"]
workspace.dependencies.ctx_fixture_replay = { path = "crates/replay" }
"#,
    );
    write(
        &repo.path().join("crates/app/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_app"
version = "0.1.0"
edition = "2024"

[dependencies]
ctx_fixture_replay.workspace = true
"#,
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "crates/replay/Cargo.toml",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(impact.status.success());
    let json: Value = serde_json::from_slice(&impact.stdout).expect("valid json");
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("crates/app/Cargo.toml")),
        "Cargo dotted key syntax should create the same package edge"
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
fn rust_workspace_table_local_dotted_cargo_syntax_feeds_package_impact() {
    let repo = fixture_copy("rust-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    write(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = ["crates/app", "crates/replay"]
dependencies.ctx_fixture_replay.path = "crates/replay"
"#,
    );
    write(
        &repo.path().join("crates/app/Cargo.toml"),
        r#"[package]
name = "ctx_fixture_app"
version = "0.1.0"
edition = "2024"

[dependencies]
ctx_fixture_replay.workspace = true
"#,
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "crates/replay/Cargo.toml",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx impact should run");
    assert!(impact.status.success());
    let json: Value = serde_json::from_slice(&impact.stdout).expect("valid json");
    assert!(
        json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("crates/app/Cargo.toml")),
        "Cargo table-local dotted workspace dependencies should create package edges"
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
fn go_workspace_routes_replay_task_to_replay_module() {
    let repo = fixture_copy("go-workspace");
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
    assert_eq!(json["domain"]["path"], "services/replay");
    assert_eq!(json["task_kind"], "playback_session");
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("services/replay/session/session.go"))
    );
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("services/replay/timeline/timeline.go"))
    );
    assert!(
        json["related_tests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("services/replay/session/session_test.go"))
    );
    assert!(
        json["do_not_read_yet"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("services/renderer/**"))
    );
}

#[test]
fn go_workspace_module_imports_feed_file_impact() {
    let repo = fixture_copy("go-workspace");
    let cache = TempDir::new().expect("cache tempdir");

    let explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "explain",
            "services/renderer/render/render.go",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    assert!(
        explain_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("services/replay/session/session.go")),
        "Go module import should resolve to the imported package source file"
    );

    let non_import_string = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "explain",
            "services/renderer/render/doc.go",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx explain should run");
    assert!(non_import_string.status.success());
    let non_import_json: Value =
        serde_json::from_slice(&non_import_string.stdout).expect("valid explain json");
    assert!(
        non_import_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("services/replay/session/session.go")),
        "Go string literals outside import declarations must not become graph imports"
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "services/replay/session/label.go",
            "--depth",
            "3",
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
            .any(|item| item.as_str() == Some("services/renderer/render/render.go")),
        "changes to any non-test file in an imported Go package should reach package importers"
    );
    assert!(
        impact_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("apps/api/main.go"))
    );
}

#[test]
fn go_work_only_repo_infers_go_verification_plan() {
    let repo = fixture_copy("go-workspace");
    let cache = TempDir::new().expect("cache tempdir");
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "verify",
            "--files",
            "services/replay/go.mod",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx verify should run");
    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert!(
        json["verification"]["minimal"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("go test ./...")),
        "go.work-only repositories should be detected as Go projects for verification"
    );
}

#[test]
fn go_workspace_mod_replace_edges_feed_impact_and_boundaries() {
    let repo = fixture_copy("go-workspace");
    let cache = TempDir::new().expect("cache tempdir");

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "services/replay/go.mod",
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
            .any(|item| item.as_str() == Some("services/renderer/go.mod")),
        "renderer consumes replay through go.mod require/replace"
    );
    assert!(
        impact_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("apps/api/go.mod")),
        "api consumes renderer and should be reached at depth 2"
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
    - from: services/renderer/go.mod
      to: services/replay/go.mod
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
                finding["from"].as_str() == Some("services/renderer/go.mod")
                    && finding["to"].as_str() == Some("services/replay/go.mod")
                    && finding["provenance"].as_str() == Some("package_manifest+ctx_anchor")
                    && finding["reason"]
                        .as_str()
                        .unwrap_or("")
                        .contains("go.mod local replace")
            })
    );
}

#[test]
fn python_workspace_routes_replay_task_to_replay_package() {
    let repo = fixture_copy("python-workspace");
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
    assert_eq!(json["domain"]["path"], "services/replay");
    assert_eq!(json["task_kind"], "playback_session");
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("services/replay/replay/session.py"))
    );
    assert!(
        json["read_first"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["path"].as_str() == Some("services/replay/replay/timeline.py"))
    );
    assert!(
        json["related_tests"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("services/replay/tests/test_session.py"))
    );
}

#[test]
fn python_workspace_imports_feed_file_impact() {
    let repo = fixture_copy("python-workspace");
    let cache = TempDir::new().expect("cache tempdir");

    let explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "explain",
            "services/renderer/renderer/render.py",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx explain should run");
    assert!(explain.status.success());
    let explain_json: Value = serde_json::from_slice(&explain.stdout).expect("valid explain json");
    assert!(
        explain_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("services/replay/replay/session.py")),
        "Python src-layout imports should resolve through workspace package roots"
    );

    let non_import_string = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "explain",
            "services/renderer/renderer/doc.py",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx explain should run");
    assert!(non_import_string.status.success());
    let non_import_json: Value =
        serde_json::from_slice(&non_import_string.stdout).expect("valid explain json");
    assert!(
        non_import_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.as_str() != Some("services/replay/replay/session.py")),
        "Python string literals must not become imports"
    );

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "services/replay/replay/timeline.py",
            "--depth",
            "3",
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
            .any(|item| item.as_str() == Some("services/replay/replay/session.py"))
    );
    assert!(
        impact_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("services/renderer/renderer/render.py"))
    );
    assert!(
        impact_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("apps/api/api/main.py"))
    );

    let relative_explain = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "explain",
            "services/replay/replay/session.py",
            "--format",
            "json",
        ])
        .output()
        .expect("ctx explain should run");
    assert!(relative_explain.status.success());
    let relative_json: Value =
        serde_json::from_slice(&relative_explain.stdout).expect("valid explain json");
    assert!(
        relative_json["imports"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("services/replay/replay/timeline.py")),
        "Python relative imports should resolve through the Python resolver"
    );
}

#[test]
fn python_workspace_pyproject_edges_feed_impact_and_boundaries() {
    let repo = fixture_copy("python-workspace");
    let cache = TempDir::new().expect("cache tempdir");

    let impact = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "services/replay/pyproject.toml",
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
            .any(|item| item.as_str() == Some("services/renderer/pyproject.toml")),
        "renderer consumes replay through pyproject local source"
    );
    assert!(
        impact_json["impacted"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("apps/api/pyproject.toml")),
        "api consumes renderer and should be reached at depth 2"
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
    - from: services/renderer/pyproject.toml
      to: services/replay/pyproject.toml
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
                finding["from"].as_str() == Some("services/renderer/pyproject.toml")
                    && finding["to"].as_str() == Some("services/replay/pyproject.toml")
                    && finding["provenance"].as_str() == Some("package_manifest+ctx_anchor")
                    && finding["reason"]
                        .as_str()
                        .unwrap_or("")
                        .contains("pyproject local path dependency")
            })
    );
    assert!(
        boundaries_json["findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| {
                !finding["reason"]
                    .as_str()
                    .unwrap_or("")
                    .contains("pyproject dependency")
            }),
        "plain project dependencies must not be overclaimed as local package edges"
    );
}
