// Responsibility: runtime-external-ci-container-path-truth
#[cfg(unix)]
#[test]
fn runtime_external_ci_directory_symlinks_are_boundaries_not_ci_facts() {
    use std::os::unix::fs::symlink;

    let (repo, cache) = runtime_candidate_fixture();
    let external = TempDir::new().expect("external CI trees");
    for root in [".circleci", ".buildkite", ".teamcity"] {
        let target = external.path().join(root.trim_start_matches('.'));
        write(
            &target.join("config.yml"),
            "jobs:\n  leaked:\n    runs-on: ubuntu-latest\n",
        );
        symlink(&target, repo.path().join(root)).expect("CI directory symlink");
    }
    commit_runtime_candidate_fixture(&repo, "external CI directory symlinks");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_external_ci_containers(&json);
}

#[test]
fn runtime_regular_ci_container_name_is_not_a_ci_file_fact() {
    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join(".circleci"), "not a CI config file\n");
    commit_runtime_candidate_fixture(&repo, "regular CI container name");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert!(
        json["ci"].as_array().expect("CI facts").is_empty(),
        "a container name without a concrete child is not a CI file convention: {json:#}"
    );
    assert_eq!(
        horizon(&json["observations"], "ci")["count"]["observed"],
        0,
        "{json:#}"
    );
}

#[cfg(unix)]
#[test]
fn runtime_external_ci_gitlinks_are_boundaries_not_ci_facts() {
    let target = TempDir::new().expect("CI submodule source");
    git(target.path(), &["init", "-q"]);
    git(target.path(), &["config", "user.email", "a@example.com"]);
    git(target.path(), &["config", "user.name", "a"]);
    write(
        &target.path().join("config.yml"),
        "jobs:\n  leaked:\n    runs-on: ubuntu-latest\n",
    );
    git(target.path(), &["add", "."]);
    git(target.path(), &["commit", "-qm", "external CI tree"]);

    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join("README.md"), "superproject\n");
    commit_runtime_candidate_fixture(&repo, "superproject root");
    for root in [".circleci", ".buildkite", ".teamcity"] {
        add_local_submodule(repo.path(), target.path(), root);
    }
    git(repo.path(), &["commit", "-qm", "external CI gitlinks"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert_external_ci_containers(&json);
}

#[cfg(unix)]
#[test]
fn runtime_file_looking_ci_gitlink_is_still_a_container_boundary() {
    let target = TempDir::new().expect("file-looking CI submodule source");
    git(target.path(), &["init", "-q"]);
    git(target.path(), &["config", "user.email", "a@example.com"]);
    git(target.path(), &["config", "user.name", "a"]);
    write(
        &target.path().join("leaked.yml"),
        "jobs:\n  leaked:\n    runs-on: ubuntu-latest\n",
    );
    git(target.path(), &["add", "."]);
    git(
        target.path(),
        &["commit", "-qm", "external file-looking CI tree"],
    );

    let (repo, cache) = runtime_candidate_fixture();
    write(&repo.path().join("README.md"), "superproject\n");
    commit_runtime_candidate_fixture(&repo, "superproject root");
    add_local_submodule(repo.path(), target.path(), "ci.yml");
    git(repo.path(), &["commit", "-qm", "file-looking CI gitlink"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    assert!(
        json["ci"].as_array().expect("CI facts").is_empty(),
        "Git mode 160000 is a container even when its path looks like a file: {json:#}"
    );
    let ci = horizon(&json["observations"], "ci");
    assert_eq!(ci["count"]["observed"], 0, "{json:#}");
    assert_eq!(ci["count"]["closure"], "open", "{json:#}");
    assert_boundary_exclusions(&json, "ci", &["ci.yml"], true);
}

fn assert_external_ci_containers(json: &Value) {
    let roots = [".circleci", ".buildkite", ".teamcity"];
    assert!(
        json["ci"].as_array().expect("CI facts").is_empty(),
        "external containers cannot become exact CI file facts: {json:#}"
    );
    let horizon = horizon(&json["observations"], "ci");
    assert_eq!(horizon["count"]["observed"], 0, "{json:#}");
    assert_eq!(horizon["count"]["closure"], "open", "{json:#}");
    assert_boundary_exclusions(json, "ci", &roots, true);
}
