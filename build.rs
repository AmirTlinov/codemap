// Responsibility: compile-time-build-provenance
use std::env;
use std::path::Path;
use std::process::Command;

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn main() {
    println!("cargo:rerun-if-env-changed=CODEMAP_SOURCE_COMMIT");
    println!("cargo:rerun-if-env-changed=CODEMAP_DIRTY_BUILD");
    println!("cargo:rerun-if-changed=src");
    // Every public schema is embedded into the binary by the schema command.
    // Watching the directory keeps compile-time dirty provenance aligned with
    // schema-only edits instead of serving the previous build-script result.
    println!("cargo:rerun-if-changed=schemas");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");

    let root = env::var("CARGO_MANIFEST_DIR").expect("Cargo manifest dir");
    let root = Path::new(&root);
    let has_git_metadata = root.join(".git").exists();
    if has_git_metadata {
        for git_path in ["HEAD", "index"] {
            if let Some(path) = git_output(root, &["rev-parse", "--git-path", git_path]) {
                println!("cargo:rerun-if-changed={path}");
            }
        }
    }
    let source_commit = env::var("CODEMAP_SOURCE_COMMIT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            has_git_metadata
                .then(|| git_output(root, &["rev-parse", "HEAD"]))
                .flatten()
        });
    if let Some(source_commit) = source_commit {
        println!("cargo:rustc-env=CODEMAP_SOURCE_COMMIT={source_commit}");
    }

    let dirty_build = env::var("CODEMAP_DIRTY_BUILD")
        .ok()
        .filter(|value| matches!(value.as_str(), "true" | "false"))
        .or_else(|| {
            if !has_git_metadata {
                return None;
            }
            let output = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["status", "--porcelain", "--untracked-files=all"])
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| (!output.stdout.is_empty()).to_string())
        });
    if let Some(dirty_build) = dirty_build {
        println!("cargo:rustc-env=CODEMAP_DIRTY_BUILD={dirty_build}");
    }
}
