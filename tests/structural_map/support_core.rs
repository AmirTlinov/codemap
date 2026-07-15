use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn codemap() -> Command {
    Command::new(env!("CARGO_BIN_EXE_codemap"))
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "init.defaultBranch=main"])
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

fn assert_schema(schema_rel: &str, instance: &Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema_text = fs::read_to_string(root.join(schema_rel)).expect("schema should exist");
    let schema: Value = serde_json::from_str(&schema_text).expect("schema json");
    let validator = jsonschema::validator_for(&schema).expect("schema should compile");
    validator
        .validate(instance)
        .unwrap_or_else(|error| panic!("{schema_rel} rejected instance: {error}"));
}
