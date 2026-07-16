#[test]
fn proof_exact_workflow_compacts_many_distinct_commands_with_exact_detail_expand() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    let steps = (0..12)
        .map(|index| format!("      - run: ./scripts/check-{index}.sh"))
        .collect::<Vec<_>>()
        .join("\n");
    write(
        &repo.path().join(".github/workflows/ci.yml"),
        &format!("name: ci\non: [push]\njobs:\n  verify:\n    runs-on: ubuntu-latest\n    steps:\n{steps}\n"),
    );
    for index in 0..12 {
        write(
            &repo.path().join(format!("scripts/check-{index}.sh")),
            "#!/usr/bin/env sh\nexit 0\n",
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "wide workflow proof fixture"]);

    let output = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args(["proof", ".github/workflows/ci.yml"])
        .output()
        .expect("proof workflow should run");
    assert!(output.status.success());
    let markdown = String::from_utf8(output.stdout).expect("markdown utf8");
    assert!(
        markdown.lines().count() <= 140,
        "wide exact proof must fit the owner-cone budget: {markdown}"
    );
    assert!(
        markdown.contains("[sensors=1; evidence=ci_run_step:1; strength=hard:1]")
            && markdown.contains("codemap proof .github/workflows/ci.yml --section proof"),
        "compact command groups must keep their source basis and exact detail expand: {markdown}"
    );
    assert!(
        !markdown.contains("\n### `./scripts/check-0.sh`"),
        "default proof should reserve repeated command cards for --section proof: {markdown}"
    );

    let detail = codemap()
        .current_dir(repo.path())
        .env("CODEMAP_CACHE_DIR", cache.path())
        .args([
            "proof",
            ".github/workflows/ci.yml",
            "--section",
            "proof",
        ])
        .output()
        .expect("proof detail should run");
    let detail = String::from_utf8(detail.stdout).expect("detail utf8");
    assert!(
        detail.contains("\n### `./scripts/check-0.sh`")
            && detail.contains("\n### `./scripts/check-11.sh`"),
        "exact proof section should retain full command cards: {detail}"
    );
}
