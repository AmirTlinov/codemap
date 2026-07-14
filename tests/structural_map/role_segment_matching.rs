// Responsibility: whole-segment path-role matching truth
#[test]
fn path_role_hints_match_whole_segments_and_skip_docs() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("src/model/cone_reports.rs"),
        "pub struct ConeReport;\n",
    );
    write(
        &repo.path().join("docs/LANG_SUPPORT.md"),
        "# language support: parser, adapter, cache, client tables\n",
    );
    write(
        &repo.path().join("src/ports/redis_port.ts"),
        "export const redisPort = {};\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "segment role fixture"]);

    let code_roles = [
        "adapter",
        "parser",
        "persistence",
        "repo_discovery",
        "cache",
        "cli_surface",
        "state_model",
        "runtime_state",
    ];

    let reports = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/model/cone_reports.rs", "--format", "json"],
    );
    let report_roles = reports["anchor"]["roles"].as_array().expect("roles");
    assert!(
        !report_roles.iter().any(|role| role == "adapter"),
        "`reports` must not match the `port` needle as a substring: {reports:#}"
    );

    let docs = run_json(
        repo.path(),
        cache.path(),
        &["ls", "docs/LANG_SUPPORT.md", "--format", "json"],
    );
    let docs_roles = docs["anchor"]["roles"].as_array().expect("docs roles");
    assert!(
        docs_roles.iter().any(|role| role == "docs"),
        "markdown surface should keep its docs role: {docs:#}"
    );
    for role in code_roles {
        assert!(
            !docs_roles.iter().any(|found| found == role),
            "docs surface must not receive code role `{role}`: {docs:#}"
        );
    }

    let port = run_json(
        repo.path(),
        cache.path(),
        &["ls", "src/ports/redis_port.ts", "--format", "json"],
    );
    assert!(
        port["anchor"]["roles"]
            .as_array()
            .expect("port roles")
            .iter()
            .any(|role| role == "adapter"),
        "a real ports/ surface must keep its adapter role under segment matching: {port:#}"
    );
}
