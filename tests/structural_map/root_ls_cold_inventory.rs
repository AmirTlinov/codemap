#[test]
fn cold_large_root_ls_uses_bounded_inventory_map() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "cold-root-inventory-fixture",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": {
    "test": "vitest run",
    "build": "tsc -b",
    "lint": "eslint ."
  }
}
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{"name":"@fixture/app","private":true}"#,
    );
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"cold-root-inventory\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("Cargo.lock"), "# lock\n");
    write(&repo.path().join(".env.example"), "DATABASE_URL=\n");
    write(&repo.path().join("README.md"), "# Cold Root Inventory\n");
    write(
        &repo.path().join("apps/api/prisma/schema.prisma"),
        "datasource db { provider = \"postgresql\" url = env(\"DATABASE_URL\") }\n",
    );
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - run: pnpm test\n",
    );
    write(
        &repo.path().join(".github/workflows/release.yml"),
        "name: release\non: [push]\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - run: |\n          pnpm build\n",
    );
    for index in 0..820 {
        write(
            &repo.path().join(format!("src/bulk/file_{index:03}.ts")),
            &format!("export const value{index} = {index};\n"),
        );
    }

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    assert_eq!(json["mode"], "directory");
    assert!(
        json["directory"].as_array().expect("directory").iter().any(|surface| {
            surface["kind"] == "package:javascript"
                && surface["examples"]
                    .as_array()
                    .expect("examples")
                    .iter()
                    .any(|example| example == "package.json")
        }),
        "cold root inventory should still expose root package manifests: {json:#}"
    );
    for (kind, example) in [
        ("manifest", "Cargo.toml"),
        ("lockfile", "Cargo.lock"),
        ("env_config", ".env.example"),
        ("docs", "README.md"),
        ("schema_contract", "apps/api/prisma/schema.prisma"),
        ("build_ci", ".github/workflows/ci.yml"),
    ] {
        assert!(
            json["directory"].as_array().expect("directory").iter().any(|surface| {
                surface["kind"] == kind
                    && surface["role"] == kind
                    && surface["examples"]
                        .as_array()
                        .expect("examples")
                        .iter()
                        .any(|item| item == example)
            }),
            "cold root inventory should expose `{kind}` surface `{example}`: {json:#}"
        );
    }
    assert!(
        json["edges"].as_array().expect("edges").iter().any(|edge| {
            edge["type"] == "declares_script"
                && edge["evidence"] == "root_inventory_script"
                && edge["from"] == "package.json"
        }),
        "cold root inventory should keep source-backed root script edges: {json:#}"
    );
    assert!(
        json["edges"].as_array().expect("edges").iter().any(|edge| {
            edge["type"] == "runs_command"
                && edge["evidence"] == "ci_run_step"
                && edge["from"] == ".github/workflows/ci.yml"
        }),
        "cold root inventory should keep source-backed CI run edges: {json:#}"
    );
    assert!(
        json["edges"].as_array().expect("edges").iter().all(|edge| {
            edge["to"] != "command:|"
        }) && json["edges"].as_array().expect("edges").iter().any(|edge| {
            edge["type"] == "declares_run_block"
                && edge["from"] == ".github/workflows/release.yml"
        }),
        "multiline CI blocks should not be rendered as fake `command:|`: {json:#}"
    );
    assert!(
        json["edges"].as_array().expect("edges").iter().any(|edge| {
            edge["type"] == "workspace_member"
                && edge["evidence"] == "root_inventory_workspace_pattern"
                && edge["to"] == "packages/app/package.json"
        }),
        "cold root inventory should keep manifest workspace edges: {json:#}"
    );
    assert!(
        json["hidden"].as_array().expect("hidden").iter().any(|hidden| {
            hidden["reason"] == "full-index source edges hidden by bounded root inventory"
        }),
        "cold root inventory must say when source import edges require the full index: {json:#}"
    );

    let markdown = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["ls", "."])
        .output()
        .expect("cold root ls markdown should run");
    assert!(markdown.status.success());
    let markdown = String::from_utf8(markdown.stdout).expect("markdown utf8");
    assert!(
        markdown.contains("Map Snapshot: root=`")
            && markdown.contains("fingerprint=`")
            && markdown.contains("cache=`")
            && markdown.contains("strategy=`inventory_fast_path`")
            && markdown.contains("location=`")
            && markdown.contains("repo_footprint=`zero`")
            && !markdown.contains("fingerprint=`unknown`")
            && !markdown.contains("cache=`unknown`")
            && !markdown.contains("location=`unknown`"),
        "cold root inventory should expose a deterministic bounded snapshot fingerprint: {markdown}"
    );
}
