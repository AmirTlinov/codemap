use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

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

fn migration_repo() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    init_repo(repo.path());
    write(
        &repo.path().join("package.json"),
        r#"{"name":"legacy-migration","scripts":{"test":"vitest run"}}"#,
    );
    write(
        &repo.path().join("src/session.ts"),
        "export function seekFrame(cursor: number) {\n  return cursor;\n}\n",
    );
    write(
        &repo.path().join("tests/session.test.ts"),
        "import { seekFrame } from '../src/session';\n\ntest('seek frame', () => expect(seekFrame(2)).toBe(2));\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    (repo, cache)
}

#[test]
fn locate_markdown_is_a_compat_wrapper_over_find_not_start() {
    let (repo, cache) = migration_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["locate", "--task", "seekFrame"])
        .output()
        .expect("ctx locate should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("# Locate Compatibility"));
    assert!(stdout.contains("ctx find seekFrame"));
    assert!(stdout.contains("# Anchor Candidates"));
    assert!(!stdout.contains("ctx start"));
}

#[test]
fn explain_file_markdown_uses_structural_ls_bridge() {
    let (repo, cache) = migration_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["explain", "src/session.ts"])
        .output()
        .expect("ctx explain should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("# Explain Compatibility"));
    assert!(stdout.contains("# Structural LS"));
    assert!(stdout.contains("ctx ls src/session.ts"));
    assert!(stdout.contains("ctx cone src/session.ts"));
}

#[test]
fn verify_markdown_bridges_to_structural_proof_without_running() {
    let (repo, cache) = migration_repo();
    write(
        &repo.path().join("src/session.ts"),
        "export function seekFrame(cursor: number) {\n  return cursor + 1;\n}\n",
    );
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["verify", "--changed"])
        .output()
        .expect("ctx verify should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("# Verify Compatibility"));
    assert!(stdout.contains("ctx proof --changed"));
    assert!(stdout.contains("# Proof Plan"));
    assert!(stdout.contains("npm test"));
    assert!(stdout.contains("does not run commands unless `--run` is explicit"));
}

#[test]
fn verify_files_markdown_points_to_proof_files_not_changed() {
    let (repo, cache) = migration_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["verify", "--files", "src/session.ts"])
        .output()
        .expect("ctx verify should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("ctx proof --files src/session.ts"));
    assert!(!stdout.contains("ctx proof --changed"));
}

#[test]
fn verify_markdown_preserves_non_default_depth_and_limit_in_proof_hint() {
    let (repo, cache) = migration_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "verify",
            "--files",
            "src/session.ts",
            "--depth",
            "2",
            "--limit",
            "3",
        ])
        .output()
        .expect("ctx verify should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("ctx proof --files src/session.ts --depth 2 --limit 3"));
}

#[test]
fn widen_with_exact_path_markdown_maps_to_cone_depth_two() {
    let (repo, cache) = migration_repo();
    let output = ctx()
        .current_dir(repo.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args([
            "widen",
            "--path",
            "src/session.ts",
            "--reason",
            "anchor did not explain the failure",
        ])
        .output()
        .expect("ctx widen should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("# Widen Compatibility"));
    assert!(stdout.contains("ctx cone src/session.ts --depth 2"));
    assert!(stdout.contains("# Structural Cone"));
}
