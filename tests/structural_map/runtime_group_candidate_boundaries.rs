// Responsibility: runtime-group-candidate-result-boundaries
fn runtime_candidate_fixture() -> (TempDir, TempDir) {
    let repo = TempDir::new().expect("runtime candidate repo");
    let cache = TempDir::new().expect("runtime candidate cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    (repo, cache)
}

fn commit_runtime_candidate_fixture(repo: &TempDir, message: &str) {
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", message]);
}

fn runtime_group_certificate<'a>(json: &'a Value, group: &str) -> &'a Value {
    let item = horizon(&json["observations"], group);
    let id = item["count"]["certificate_id"]
        .as_str()
        .expect("runtime certificate id");
    &json["observations"]["certificates"][id]
}

#[test]
fn runtime_group_nested_nonempty_scope_keeps_root_only_script_catalog_open() {
    let (repo, cache) = runtime_candidate_fixture();
    write(
        &repo.path().join("src/app.ts"),
        "export const app = true;\n",
    );
    commit_runtime_candidate_fixture(&repo, "nested script boundary");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src", "--format", "json"],
    );
    let item = horizon(&json["observations"], "scripts");
    assert_eq!(item["count"]["observed"], 0, "{json:#}");
    assert_eq!(item["count"]["closure"], "open", "{json:#}");
    assert!(
        runtime_group_certificate(&json, "scripts")["unresolved_stops"]
            .as_array()
            .expect("script stops")
            .iter()
            .any(|stop| stop["kind"] == "unsupported_construct"),
        "a root-only catalog cannot close a nonempty nested scope: {json:#}"
    );
}

#[test]
fn runtime_group_ignored_root_manifest_does_not_leak_script_or_entrypoint_facts() {
    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join(".gitignore"), "package.json\n");
    write(&repo.path().join("README.md"), "tracked owner\n");
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"echo leaked"},"bin":{"leak":"leak.js"}}"#,
    );
    commit_runtime_candidate_fixture(&repo, "ignored manifest boundary");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert!(
        json["scripts"].as_array().expect("scripts").is_empty(),
        "{json:#}"
    );
    assert!(
        json["entrypoints"]
            .as_array()
            .expect("entrypoints")
            .is_empty(),
        "ignored filesystem manifests are outside indexed runtime truth: {json:#}"
    );
}

#[cfg(unix)]
#[test]
fn runtime_group_external_makefile_symlink_is_not_a_fact_and_keeps_coverage_open() {
    use std::os::unix::fs::symlink;

    let (repo, cache) = runtime_candidate_fixture();
    let external = TempDir::new().expect("external makefile owner");
    write(&external.path().join("GNUmakefile"), "pwn:\n\t@true\n");
    write(&repo.path().join("README.md"), "tracked owner\n");
    write(&repo.path().join("Makefile"), "lower:\n\t@true\n");
    symlink(
        external.path().join("GNUmakefile"),
        repo.path().join("GNUmakefile"),
    )
    .expect("tracked makefile symlink");
    commit_runtime_candidate_fixture(&repo, "script symlink boundary");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert!(
        json["scripts"].as_array().expect("scripts").is_empty(),
        "{json:#}"
    );
    let item = horizon(&json["observations"], "scripts");
    assert_eq!(item["count"]["closure"], "open", "{json:#}");
    assert_runtime_external_exclusion(&json, "scripts", "GNUmakefile");
}

#[cfg(unix)]
#[test]
fn runtime_group_ci_symlink_keeps_indexed_path_truth_without_following_content() {
    use std::os::unix::fs::symlink;

    let (repo, cache) = runtime_candidate_fixture();
    let external = TempDir::new().expect("external workflow owner");
    write(
        &external.path().join("ci.yml"),
        "jobs:\n  leak:\n    env:\n      EXTERNAL_SECRET: leaked\n",
    );
    fs::create_dir_all(repo.path().join(".github/workflows")).expect("workflow directory");
    symlink(
        external.path().join("ci.yml"),
        repo.path().join(".github/workflows/ci.yml"),
    )
    .expect("tracked workflow symlink");
    commit_runtime_candidate_fixture(&repo, "ci symlink boundary");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert!(
        json["ci"].as_array().expect("ci").iter().any(|surface| {
            surface["path"] == ".github/workflows/ci.yml" && surface["evidence"] == "role:build_ci"
        }),
        "the indexed workflow path is a fact even when external content is not: {json:#}"
    );
    assert!(
        json["env"].as_array().expect("env").is_empty(),
        "external workflow contents must not leak into runtime facts: {json:#}"
    );
    let certificate = runtime_group_certificate(&json, "ci");
    assert_eq!(certificate["eligible_files"], 1, "{json:#}");
    assert_eq!(certificate["visited_files"], 0, "{json:#}");
    assert_eq!(
        horizon(&json["observations"], "ci")["count"]["closure"],
        "open",
        "{json:#}"
    );
    assert_runtime_external_exclusion(&json, "ci", ".github/workflows/ci.yml");
}

#[test]
fn runtime_group_non_source_text_cannot_create_facts_outside_its_universe() {
    let (repo, cache) = runtime_candidate_fixture();
    write(
        &repo.path().join("README.md"),
        "db.query(\"SELECT * FROM users\")\nprocess.env.README_SECRET\n",
    );
    write(
        &repo.path().join("Dockerfile"),
        "RUN node -e process.env.DOCKER_SECRET\n",
    );
    commit_runtime_candidate_fixture(&repo, "non-source runtime noise");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    for group in ["entrypoints", "env", "unknowns"] {
        assert!(
            json[group].as_array().expect("runtime group").is_empty(),
            "{json:#}"
        );
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
        assert!(
            !runtime_group_certificate(&json, group)["unresolved_stops"]
                .as_array()
                .expect("partial detector stop")
                .is_empty(),
            "a nonempty unsupported carrier cannot prove {group} zero: {json:#}"
        );
    }
}

#[test]
fn runtime_group_noncanonical_manifest_cannot_emit_cli_facts_or_enter_candidate_universe() {
    let (repo, cache) = runtime_candidate_fixture();
    write(
        &repo.path().join("PACKAGE.JSON"),
        r#"{"bin":{"tool":"missing.js"}}"#,
    );
    commit_runtime_candidate_fixture(&repo, "uppercase manifest boundary");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let item = horizon(&json["observations"], "entrypoints");
    let certificate = runtime_group_certificate(&json, "entrypoints");
    assert!(
        json["entrypoints"]
            .as_array()
            .expect("entrypoints")
            .is_empty(),
        "a noncanonical manifest name cannot mint CLI facts: {json:#}"
    );
    assert_eq!(item["count"]["observed"], 0, "{json:#}");
    assert_eq!(item["count"]["closure"], "open", "{json:#}");
    assert_eq!(certificate["eligible_files"], 0, "{json:#}");
    assert_eq!(certificate["visited_files"], 0, "{json:#}");
    assert!(
        !certificate["unresolved_stops"]
            .as_array()
            .expect("partial detector stop")
            .is_empty(),
        "the nonempty scope must remain an honest lower bound: {json:#}"
    );
}

#[test]
fn runtime_group_shell_only_scope_is_an_open_entrypoint_lower_bound() {
    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join("run.sh"), "#!/bin/sh\nexec true\n");
    commit_runtime_candidate_fixture(&repo, "shell entrypoint boundary");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let item = horizon(&json["observations"], "entrypoints");
    assert_eq!(item["count"]["observed"], 0, "{json:#}");
    assert_eq!(item["count"]["closure"], "open", "{json:#}");
    assert_unsupported_file(item, "run.sh", &json);
}

#[test]
fn runtime_group_unsupported_static_env_syntax_cannot_become_proven_zero() {
    let (repo, cache) = runtime_candidate_fixture();
    write(
        &repo.path().join("app.ts"),
        "const { API_KEY } = process.env;\n",
    );
    commit_runtime_candidate_fixture(&repo, "env syntax boundary");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let item = horizon(&json["observations"], "env");
    assert_eq!(item["count"]["observed"], 0, "{json:#}");
    assert_eq!(item["count"]["closure"], "open", "{json:#}");
    assert!(
        runtime_group_certificate(&json, "env")["unresolved_stops"]
            .as_array()
            .expect("env stops")
            .iter()
            .any(|stop| stop["kind"] == "unsupported_construct"),
        "supported-language syntax outside declared access forms needs an explicit stop: {json:#}"
    );
}

#[test]
fn runtime_group_gnumakefile_uses_the_same_script_fact_and_candidate_owner() {
    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join("GNUmakefile"), "check:\n\t@true\n");
    commit_runtime_candidate_fixture(&repo, "gnu make script owner");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert!(
        json["scripts"]
            .as_array()
            .expect("scripts")
            .iter()
            .any(|script| script["path"] == "GNUmakefile"),
        "GNUmakefile must be read by the owner that declares it eligible: {json:#}"
    );
    let certificate = runtime_group_certificate(&json, "scripts");
    assert_eq!(certificate["eligible_files"], 1, "{json:#}");
    assert_eq!(certificate["visited_files"], 1, "{json:#}");
}

#[test]
fn runtime_group_make_catalog_uses_gnu_precedence_and_one_active_carrier() {
    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join("Makefile"), "legacy:\n\t@true\n");
    write(&repo.path().join("GNUmakefile"), "actual:\n\t@true\n");
    commit_runtime_candidate_fixture(&repo, "make carrier precedence");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let examples = json["scripts"]
        .as_array()
        .expect("scripts")
        .iter()
        .flat_map(|script| script["examples"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert!(
        examples
            .iter()
            .any(|example| example.starts_with("actual:")),
        "{json:#}"
    );
    assert!(
        examples
            .iter()
            .all(|example| !example.starts_with("legacy:")),
        "{json:#}"
    );
    let certificate = runtime_group_certificate(&json, "scripts");
    assert_eq!(certificate["eligible_files"], 1, "{json:#}");
    assert_eq!(certificate["visited_files"], 1, "{json:#}");
}

#[test]
fn runtime_limit_zero_uses_the_existing_minimum_one_projection() {
    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join("src/main.rs"), "fn main() {}\n");
    write(
        &repo.path().join("src/main.go"),
        "package main\nfunc main() {}\n",
    );
    commit_runtime_candidate_fixture(&repo, "runtime zero limit");

    let readable = run_markdown(
        repo.path(),
        cache.path(),
        &["runtime", "src", "--limit", "0"],
    );
    assert!(
        readable.contains("entrypoints: counted-at-least(2,")
            && readable.contains("shown=1 hidden=1"),
        "runtime must clamp zero to its established one-row minimum: {readable}"
    );
}

#[test]
fn runtime_group_script_runner_ignores_unindexed_package_manager_locks() {
    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join(".gitignore"), "pnpm-lock.yaml\n");
    write(
        &repo.path().join("package.json"),
        r#"{"scripts":{"test":"node test.js"}}"#,
    );
    write(&repo.path().join("pnpm-lock.yaml"), "lockfileVersion: 9\n");
    commit_runtime_candidate_fixture(&repo, "ignored package manager lock");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    let scripts = json["scripts"].as_array().expect("scripts");
    assert_eq!(scripts.len(), 1, "{json:#}");
    assert_eq!(scripts[0]["examples"][0], "test: npm test", "{json:#}");
    assert_eq!(
        runtime_group_certificate(&json, "scripts")["eligible_files"],
        1,
        "the script fact and runner must be bound to the indexed package.json: {json:#}"
    );
}
