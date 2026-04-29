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

fn fixture() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "map-fixture",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": { "test": "pnpm test", "typecheck": "tsc -b" }
}
"#,
    );
    write(
        &repo.path().join("packages/replay/package.json"),
        r#"{
  "name": "@fixture/replay",
  "version": "1.0.0",
  "main": "src/index.ts",
  "exports": { ".": "./src/index.ts" },
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "private": true,
  "dependencies": { "@fixture/replay": "workspace:*" },
  "scripts": { "test": "vitest run" }
}
"#,
    );
    write(
        &repo.path().join(".ctx.yml"),
        r#"version: 1

boundaries:
  forbidden:
    - from: packages/app/src/**
      to: packages/replay/src/internal.ts
      reason: app must consume replay public exports
"#,
    );
    write(
        &repo.path().join("packages/replay/src/index.ts"),
        "export { seek } from './session';\nexport type { FrameDto } from './types';\n",
    );
    write(
        &repo.path().join("packages/replay/src/types.ts"),
        "export interface FrameDto {\n  frame: number;\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/timeline.ts"),
        "export class Timeline {\n  frameAt(cursor: number) {\n    return cursor;\n  }\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/session.ts"),
        "import { Timeline } from './timeline';\nimport type { FrameDto } from './types';\n\nexport function seek(cursor: number): FrameDto {\n  return { frame: new Timeline().frameAt(cursor) };\n}\n",
    );
    write(
        &repo.path().join("packages/replay/src/internal.ts"),
        "export const internalValue = 1;\n",
    );
    write(
        &repo.path().join("packages/replay/tests/session.test.ts"),
        "import { seek } from '../src/session';\n\ntest('seek maps frame', () => {\n  expect(seek(2).frame).toBe(2);\n});\n",
    );
    write(
        &repo.path().join("packages/replay/tests/e2e/seek.e2e.ts"),
        "import { seek } from '../../src/session';\n\ntest('e2e seek maps frame', () => {\n  expect(seek(3).frame).toBe(3);\n});\n",
    );
    write(
        &repo.path().join("packages/replay/tests/support/setup.ts"),
        "export const setup = true;\n",
    );
    write(
        &repo.path().join("packages/app/src/useReplay.ts"),
        "import { seek } from '@fixture/replay';\n\nexport const frame = seek(1).frame;\n",
    );
    write(
        &repo.path().join("packages/app/src/badInternal.ts"),
        "import { internalValue } from '../../replay/src/internal';\n\nexport const bad = internalValue;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);
    (repo, cache)
}

fn run_json(repo: &Path, cache: &Path, args: &[&str]) -> Value {
    let output = codemap()
        .current_dir(repo)
        .env("CODEMAP_CACHE_DIR", cache)
        .args(args)
        .output()
        .expect("codemap should run");
    assert!(
        output.status.success(),
        "codemap {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid json")
}

#[test]
fn help_exposes_only_map_first_commands() {
    let output = codemap().arg("--help").output().expect("help should run");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("help utf8");
    for command in ["ls", "cone", "impact", "proof", "graph", "boundaries"] {
        assert!(stdout.contains(command), "help should expose {command}");
    }
    for forbidden in ["start", "locate", "find", "verify", "widen", "read_first"] {
        assert!(
            !stdout.contains(forbidden),
            "help must not expose removed surface {forbidden}"
        );
    }
}

#[test]
fn root_ls_is_a_bounded_domain_and_package_map() {
    let (repo, cache) = fixture();
    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    assert_eq!(json["kind"], "ls_report");
    assert_eq!(json["schema_version"], "2");
    assert_eq!(json["mode"], "directory");
    let surfaces = json["directory"].as_array().expect("directory surfaces");
    assert!(surfaces.iter().any(|surface| surface["kind"] == "domain"));
    assert!(
        surfaces
            .iter()
            .any(|surface| surface["kind"] == "package:javascript")
    );
    assert!(surfaces.iter().any(|surface| surface["kind"] == "dir"));
    assert!(
        json["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(|edge| edge["type"] == "package_internal"
                && edge["from"] == "packages/app/"
                && edge["to"] == "packages/replay/")
    );
    assert_eq!(json.get("read_first"), None);
    assert_eq!(json.get("confidence"), None);

    let tests_scope = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/replay/tests", "--format", "json"],
    );
    let test_surfaces = tests_scope["directory"].as_array().expect("test surfaces");
    assert!(
        test_surfaces
            .iter()
            .any(|surface| surface["kind"] == "e2e_test")
    );
    assert!(
        test_surfaces
            .iter()
            .any(|surface| surface["kind"] == "test_support")
    );
}

#[test]
fn file_ls_and_cone_show_symbols_edges_proof_and_boundary() {
    let (repo, cache) = fixture();
    let ls = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/replay/src/session.ts", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["anchor"]["path"], "packages/replay/src/session.ts");
    assert!(
        ls["anchor"]["symbols"]
            .as_array()
            .expect("symbols")
            .iter()
            .any(|symbol| symbol["name"] == "seek" && symbol["kind"] == "function")
    );
    assert!(
        ls["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .any(
                |edge| edge["from"] == "packages/replay/tests/session.test.ts"
                    && edge["type"] == "tests"
            )
    );

    let cone = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "packages/app/src/badInternal.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone);
    assert!(
        cone["boundary"]
            .as_array()
            .expect("boundary")
            .iter()
            .any(|edge| edge["from"] == "packages/app/src/badInternal.ts"
                && edge["to"] == "packages/replay/src/internal.ts"
                && edge["strength"] == "hard")
    );
}

#[test]
fn impact_and_proof_are_structural_without_structural_flag() {
    let (repo, cache) = fixture();
    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "packages/replay/src/types.ts",
            "--depth",
            "2",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    assert_eq!(impact["kind"], "impact_report");
    assert_eq!(impact["schema_version"], "2");
    let cluster = &impact["clusters"][0];
    assert_eq!(cluster["risk"], "high");
    assert!(
        cluster["direct_consumers"]
            .as_array()
            .expect("direct consumers")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/src/session.ts")
    );
    assert!(
        cluster["proof"]
            .as_array()
            .expect("proof")
            .iter()
            .any(|edge| edge["from"] == "packages/replay/tests/session.test.ts")
    );

    let proof = run_json(
        repo.path(),
        cache.path(),
        &[
            "proof",
            "packages/replay/src/session.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/proof.schema.json", &proof);
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .any(
                |proof| proof["path"] == "packages/replay/tests/session.test.ts"
                    && proof["command"]
                        .as_str()
                        .unwrap_or_default()
                        .contains("vitest run")
            )
    );
    assert!(
        proof["proofs"]
            .as_array()
            .expect("proofs")
            .iter()
            .all(|proof| proof["path"] != "packages/replay/tests/support/setup.ts"),
        "test support files are map surfaces, not runnable proof"
    );
}

#[test]
fn schema_manifest_has_no_removed_router_contracts_and_schema_command_is_side_effect_free() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest_text =
        fs::read_to_string(root.join("schemas/manifest.json")).expect("manifest should exist");
    let manifest: Value = serde_json::from_str(&manifest_text).expect("manifest json");
    assert_eq!(manifest["version"], 2);
    let schemas = manifest["schemas"].as_array().expect("schemas");
    let kinds = schemas
        .iter()
        .map(|entry| entry["kind"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    for forbidden in [
        "capsule",
        "find",
        "verify",
        "locate",
        "explain",
        "widen",
        "impact-v2",
    ] {
        assert!(!kinds.iter().any(|kind| kind == forbidden));
    }

    let actual_schema_files = fs::read_dir(root.join("schemas"))
        .expect("schemas dir")
        .map(|entry| entry.expect("schema dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("manifest.json"))
        .map(|path| format!("schemas/{}", path.file_name().unwrap().to_string_lossy()))
        .collect::<std::collections::BTreeSet<_>>();
    let manifest_schema_files = schemas
        .iter()
        .map(|entry| entry["file"].as_str().unwrap().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(manifest_schema_files, actual_schema_files);

    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    for entry in schemas {
        let kind = entry["kind"].as_str().unwrap();
        let rel = entry["file"].as_str().unwrap();
        let schema_json: Value =
            serde_json::from_str(&fs::read_to_string(root.join(rel)).expect("schema"))
                .expect("schema json");
        assert_eq!(
            schema_json["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema_json["$id"],
            format!("https://github.com/AmirTlinov/codemap/{rel}")
        );
        let output = codemap()
            .current_dir(outside.path())
            .env("CODEMAP_CACHE_DIR", cache.path())
            .args(["schema", kind])
            .output()
            .expect("schema command should run");
        assert!(output.status.success());
        let printed: Value = serde_json::from_slice(&output.stdout).expect("printed schema json");
        assert_eq!(printed, schema_json);
    }
    assert_eq!(fs::read_dir(cache.path()).expect("cache dir").count(), 0);
}

#[test]
fn removed_graph_lens_aliases_fail_closed() {
    let (repo, cache) = fixture();
    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["graph", "--lens", "verify", "--format", "json"])
        .output()
        .expect("codemap should run");
    assert!(
        !output.status.success(),
        "removed verify lens alias must not silently fall back"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown graph lens"));
}

#[test]
fn removed_router_commands_and_flags_fail_closed() {
    let (repo, cache) = fixture();
    let removed_command = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["explain", "packages/replay/src/session.ts"])
        .output()
        .expect("codemap should run");
    assert!(
        !removed_command.status.success(),
        "removed explain command must not be accepted"
    );

    let removed_flag = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "impact",
            "--files",
            "packages/replay/src/session.ts",
            "--structural",
        ])
        .output()
        .expect("codemap should run");
    assert!(
        !removed_flag.status.success(),
        "removed --structural flag must not be accepted"
    );
}
