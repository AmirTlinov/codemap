use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn ctx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ctx"))
}

#[test]
fn schema_manifest_is_the_exported_contract_guard() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest_text =
        fs::read_to_string(root.join("schemas/manifest.json")).expect("manifest should exist");
    let manifest: Value = serde_json::from_str(&manifest_text).expect("manifest should be json");
    assert_eq!(manifest["version"], 1);
    let route_schema_version = manifest["route_schema_version"]
        .as_str()
        .expect("route_schema_version should be a string");
    let anchor_config_version = manifest["anchor_config_version"]
        .as_i64()
        .expect("anchor_config_version should be a number");
    let schemas = manifest["schemas"]
        .as_array()
        .expect("manifest schemas should be an array");

    let actual_schema_files = fs::read_dir(root.join("schemas"))
        .expect("schemas dir should exist")
        .map(|entry| entry.expect("schema dir entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("manifest.json"))
        .map(|path| format!("schemas/{}", path.file_name().unwrap().to_string_lossy()))
        .collect::<BTreeSet<_>>();

    let manifest_schema_files = schemas
        .iter()
        .map(|entry| {
            entry["file"]
                .as_str()
                .expect("schema manifest entry should have file")
                .to_string()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        manifest_schema_files, actual_schema_files,
        "schemas/manifest.json must list every bundled schema and no missing schema"
    );

    let outside = TempDir::new().expect("outside tempdir");
    let cache = TempDir::new().expect("cache tempdir");

    let manifest_output = ctx()
        .current_dir(outside.path())
        .env("CTX_CACHE_DIR", cache.path())
        .args(["schema", "manifest"])
        .output()
        .expect("ctx schema manifest should run");
    assert!(
        manifest_output.status.success(),
        "ctx schema manifest should succeed: {}",
        String::from_utf8_lossy(&manifest_output.stderr)
    );
    let printed_manifest: Value =
        serde_json::from_slice(&manifest_output.stdout).expect("printed manifest should be json");
    assert_eq!(
        printed_manifest, manifest,
        "ctx schema manifest drifted from bundled manifest"
    );

    for entry in schemas {
        let kind = entry["kind"]
            .as_str()
            .expect("schema manifest entry should have kind");
        let rel = entry["file"]
            .as_str()
            .expect("schema manifest entry should have file");
        let contract = entry["contract"]
            .as_str()
            .expect("schema manifest entry should have contract");
        let schema_text = fs::read_to_string(root.join(rel)).expect("schema file should exist");
        let schema_json: Value =
            serde_json::from_str(&schema_text).expect("schema file should be json");

        assert_eq!(
            schema_json["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema_json["$id"],
            format!("https://github.com/AmirTlinov/ctx/{rel}")
        );
        assert_eq!(schema_json["additionalProperties"], false);

        let output = ctx()
            .current_dir(outside.path())
            .env("CTX_CACHE_DIR", cache.path())
            .args(["schema", kind])
            .output()
            .expect("ctx schema should run");
        assert!(
            output.status.success(),
            "ctx schema {kind} should succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let printed_json: Value =
            serde_json::from_slice(&output.stdout).expect("printed schema should be json");
        assert_eq!(printed_json, schema_json, "ctx schema {kind} drifted");

        match contract {
            "route_output" => {
                assert_route_schema_policy(kind, entry, &schema_json, route_schema_version)
            }
            "semantic_anchor_config" => {
                assert_anchor_schema_policy(&schema_json, anchor_config_version)
            }
            other => panic!("unsupported schema contract `{other}` for {kind}"),
        }
    }

    assert_eq!(
        fs::read_dir(cache.path()).expect("cache dir").count(),
        0,
        "ctx schema commands must not load a project or write cache"
    );
}

fn assert_route_schema_policy(
    kind: &str,
    manifest_entry: &Value,
    schema_json: &Value,
    route_schema_version: &str,
) {
    let required = schema_json["required"]
        .as_array()
        .expect("route schema required should be an array");
    assert!(
        required
            .iter()
            .any(|value| value.as_str() == Some("schema_version")),
        "{kind} schema must require schema_version"
    );
    assert_eq!(
        schema_json["properties"]["schema_version"]["const"],
        route_schema_version
    );

    if let Some(json_kind) = manifest_entry["json_kind"].as_str() {
        assert_eq!(schema_json["properties"]["kind"]["const"], json_kind);
    } else {
        let expected = manifest_entry["json_kind_values"]
            .as_array()
            .expect("dynamic kind schemas should declare json_kind_values")
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<BTreeSet<_>>();
        let actual = schema_json["properties"]["kind"]["enum"]
            .as_array()
            .expect("dynamic kind schema should have kind enum")
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
    }
}

fn assert_anchor_schema_policy(schema_json: &Value, anchor_config_version: i64) {
    let required = schema_json["required"]
        .as_array()
        .expect("anchor schema required should be an array");
    assert!(
        required
            .iter()
            .any(|value| value.as_str() == Some("version")),
        "anchor schema must require version"
    );
    assert_eq!(
        schema_json["properties"]["version"]["const"],
        anchor_config_version
    );
}
