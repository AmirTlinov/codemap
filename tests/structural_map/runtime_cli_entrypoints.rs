#[test]
fn runtime_lens_exposes_manifest_cli_entrypoints() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "private": true,
  "bin": {
    "fixture-app": "./src/cli.ts",
    "bad-escape": "../../../src/main.rs"
  },
  "dependencies": { "@fixture/replay": "workspace:*" },
  "scripts": { "test": "vitest run", "test:e2e": "playwright test" }
}
"#,
    );
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
    write(
        &repo.path().join("packages/app/src/cli.ts"),
        "export function main() { return true; }\n",
    );
    write(
        &repo.path().join("tools/pycli/pyproject.toml"),
        "[project]\nname = \"fixture-pycli\"\n[project.scripts]\nfixture-pycli = \"fixture_pycli.cli:main\"\n",
    );
    write(
        &repo.path().join("tools/pycli/src/fixture_pycli/cli.py"),
        "def main():\n    return True\n",
    );
    write(
        &repo.path().join("crates/worker/Cargo.toml"),
        "[package]\nname = \"fixture-worker\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"fixture-worker\"\npath = \"src/bin/worker.rs\"\n",
    );
    write(
        &repo.path().join("crates/worker/src/bin/worker.rs"),
        "fn main() {}\n",
    );
    write(
        &repo.path().join("crates/default-bin/Cargo.toml"),
        "[package]\nname = \"fixture-default-bin\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("crates/default-bin/src/main.rs"), "fn main() {}\n");
    write(
        &repo.path().join("crates/explicit-default/Cargo.toml"),
        "[package]\nname = \"fixture-explicit-default\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"fixture-explicit-default\"\npath = \"src/main.rs\"\n",
    );
    write(
        &repo.path().join("crates/explicit-default/src/main.rs"),
        "fn main() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "cli runtime fixture"]);

    let runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
    assert_schema("schemas/runtime.schema.json", &runtime);
    for (command, path, evidence) in [
        (
            "fixture-app",
            "packages/app/src/cli.ts",
            "package_json_bin",
        ),
        (
            "fixture-pycli",
            "tools/pycli/src/fixture_pycli/cli.py",
            "pyproject_project_scripts",
        ),
        (
            "fixture-worker",
            "crates/worker/src/bin/worker.rs",
            "cargo_bin_target",
        ),
        (
            "fixture-default-bin",
            "crates/default-bin/src/main.rs",
            "cargo_default_bin_convention",
        ),
    ] {
        assert!(
            runtime["entrypoints"]
                .as_array()
                .expect("runtime entrypoints")
                .iter()
                .any(|surface| surface["kind"] == "cli_entrypoint"
                    && surface["path"] == path
                    && surface["evidence"] == evidence
                    && surface["examples"]
                        .as_array()
                        .is_some_and(|examples| examples.iter().any(|example| example
                            .as_str()
                            .is_some_and(|value| value.contains(command))))),
            "runtime lens should expose deterministic CLI entrypoint {command} -> {path}: {runtime:#}"
        );
    }
    let explicit_default = runtime["entrypoints"]
        .as_array()
        .expect("runtime entrypoints")
        .iter()
        .filter(|surface| {
            surface["kind"] == "cli_entrypoint"
                && surface["path"] == "crates/explicit-default/src/main.rs"
        })
        .collect::<Vec<_>>();
    assert_eq!(
        explicit_default.len(),
        1,
        "explicit Cargo bin matching the default src/main.rs must not duplicate the same CLI surface: {runtime:#}"
    );
    assert!(
        runtime["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .any(|surface| surface["kind"] == "cli_entrypoint"
                && surface["path"] == "packages/app/package.json"
                && surface["examples"]
                    .as_array()
                    .is_some_and(|examples| examples.iter().any(|example| example
                        .as_str()
                        .is_some_and(|value| value.contains("bad-escape -> ../../../src/main.rs"))))),
        "escaped package bin targets must stay manifest declarations, not exact file paths: {runtime:#}"
    );
    assert!(
        !runtime["entrypoints"]
            .as_array()
            .expect("runtime entrypoints")
            .iter()
            .any(|surface| surface["kind"] == "cli_entrypoint"
                && surface["path"] == "src/main.rs"
                && surface["examples"]
                    .as_array()
                    .is_some_and(|examples| examples.iter().any(|example| example
                        .as_str()
                        .is_some_and(|value| value.contains("bad-escape"))))),
        "escaped package bin target must not be normalized into a false exact src/main.rs path: {runtime:#}"
    );
}
