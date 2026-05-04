#[test]
fn ls_root_and_changed_render_boundary_facts_without_policy_verdicts() {
    let (repo, cache) = fixture();
    write(&repo.path().join("AGENTS.md"), "# local instructions\n");
    write(&repo.path().join("SECURITY.md"), "# security\n");
    write(&repo.path().join(".agents/rules.md"), "# agent rules\n");
    write(&repo.path().join("Makefile"), "doctor:\n\ttrue\n");
    write(&repo.path().join("tools/doctor.py"), "print('ok')\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        "name: ci\non: [push]\n",
    );
    write(&repo.path().join("models/tiny.gguf"), "not a real model\n");

    let ls = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &ls);
    assert_eq!(ls["schema_version"], "5");
    assert!(
        ls["boundary_facts"]["instruction_files"]
            .as_array()
            .expect("instruction files")
            .iter()
            .any(|fact| fact["path"] == "AGENTS.md"),
        "ls root should expose instruction files as boundary facts: {ls:#}"
    );
    assert!(
        ls["boundary_facts"]["repo_local_guard_files"]
            .as_array()
            .expect("guard files")
            .iter()
            .any(|fact| fact["path"] == "tools/doctor.py"),
        "ls root should expose repo-local guard files as facts: {ls:#}"
    );
    assert!(
        ls["boundary_facts"]["protected_looking_paths"]
            .as_array()
            .expect("protected-looking paths")
            .iter()
            .any(|fact| fact["path"] == "models/tiny.gguf"),
        "ls root should expose protected-looking paths as facts: {ls:#}"
    );

    let ls_markdown = run_lens_stdout(repo.path(), cache.path(), &["ls", "."]);
    assert!(
        ls_markdown.contains("\n## Boundary Facts\n")
            && ls_markdown.contains("instruction files:")
            && ls_markdown.contains("repo-local guard files:")
            && ls_markdown.contains("protected-looking paths:"),
        "ls markdown should render boundary facts: {ls_markdown}"
    );
    for forbidden in ["recommended", "best", "safe", "unsafe", "policy verdict"] {
        assert!(
            !ls_markdown.to_ascii_lowercase().contains(forbidden),
            "boundary facts must not become advice/verdict wording `{forbidden}`: {ls_markdown}"
        );
    }

    let changed = run_json(repo.path(), cache.path(), &["changed", "--format", "json"]);
    assert_schema("schemas/changed.schema.json", &changed);
    assert_eq!(changed["schema_version"], "8");
    assert!(
        changed["boundary_facts"]["instruction_files"]
            .as_array()
            .expect("changed instruction files")
            .iter()
            .any(|fact| fact["path"] == "AGENTS.md"),
        "changed should expose changed instruction files: {changed:#}"
    );
    assert!(
        changed["boundary_facts"]["protected_looking_paths"]
            .as_array()
            .expect("changed protected-looking paths")
            .iter()
            .any(|fact| fact["path"] == "models/tiny.gguf"),
        "changed should expose protected-looking changed paths: {changed:#}"
    );
}
