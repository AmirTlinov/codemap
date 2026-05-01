#[test]
fn runtime_groups_repeated_env_references_by_file_and_name() {
    let (repo, cache) = fixture();
    write(
        &repo.path().join("packages/app/src/repeated-env.ts"),
        "export const first = process.env.CI;\nexport const second = process.env.CI;\nexport const third = process.env.CI;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "repeated env fixture"]);

    let runtime = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "packages/app/src", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &runtime);
    let repeated = runtime["env"]
        .as_array()
        .expect("env surfaces")
        .iter()
        .filter(|surface| {
            surface["name"] == "CI" && surface["used_by"] == "packages/app/src/repeated-env.ts"
        })
        .collect::<Vec<_>>();
    assert_eq!(
        repeated.len(),
        1,
        "runtime env map should show one surface per file/name, not one row per repeated line: {runtime:#}"
    );
    assert_eq!(
        repeated[0]["locations"]
            .as_array()
            .expect("locations")
            .len(),
        3,
        "grouped env surface should preserve every exact line as locations: {runtime:#}"
    );
}
