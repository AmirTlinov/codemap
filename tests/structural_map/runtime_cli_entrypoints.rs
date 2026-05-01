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

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--include-hidden", "--format", "json"],
    );
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

#[test]
fn runtime_lens_exposes_clap_subcommands_for_cli_scope() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("Cargo.toml"),
        "[package]\nname = \"clap-command-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
    write(
        &repo.path().join("src/cli/args.rs"),
        r#"use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
pub struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    #[command(about = "Show structural map changes")]
    DiffMap(DiffMapArgs),
    #[command(alias = "check-boundaries")]
    #[command(about = "Check explicit forbidden boundaries")]
    Boundaries(BoundariesArgs),
}
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "clap command fixture"]);

    let root_runtime = run_json(repo.path(), cache.path(), &["runtime", ".", "--format", "json"]);
    assert_schema("schemas/runtime.schema.json", &root_runtime);
    assert!(
        root_runtime["entrypoints"]
            .as_array()
            .expect("root runtime entrypoints")
            .iter()
            .all(|surface| surface["kind"] != "cli_command"),
        "root runtime should stay current-level and not recursively dump CLI commands: {root_runtime:#}"
    );

    let cli_runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/cli", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &cli_runtime);
    let entrypoints = cli_runtime["entrypoints"]
        .as_array()
        .expect("cli runtime entrypoints");
    assert!(
        entrypoints.iter().any(|surface| {
            surface["kind"] == "cli_command"
                && surface["path"] == "src/cli/args.rs#CommandKind::DiffMap"
                && surface["evidence"] == "clap_subcommand_enum"
                && surface["examples"].as_array().is_some_and(|examples| {
                    examples.iter().any(|example| {
                        example.as_str().is_some_and(|value| {
                            value.contains("diff-map -> src/cli/args.rs:")
                                && value.contains("Show structural map changes")
                        })
                    })
                })
        }),
        "runtime src/cli should expose deterministic Clap subcommand surfaces: {cli_runtime:#}"
    );
    assert!(
        entrypoints.iter().any(|surface| {
            surface["kind"] == "cli_command"
                && surface["path"] == "src/cli/args.rs#CommandKind::Boundaries"
                && surface["examples"].as_array().is_some_and(|examples| {
                    examples.iter().any(|example| {
                        example.as_str().is_some_and(|value| {
                            value.contains("boundaries -> src/cli/args.rs:")
                                && value.contains("alias: check-boundaries")
                                && value.contains("Check explicit forbidden boundaries")
                        })
                    })
                })
        }),
        "runtime src/cli should preserve Clap aliases and about text as evidence, not prose guessing: {cli_runtime:#}"
    );
}
