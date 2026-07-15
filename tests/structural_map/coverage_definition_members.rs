// Responsibility: definition-member-coverage-regressions

#[test]
fn unindexed_javascript_member_definitions_keep_the_horizon_open() {
    let repo = TempDir::new().expect("member definition repo");
    let cache = TempDir::new().expect("member definition cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"member-definitions","private":true}"#,
    );
    write(
        &repo.path().join("src/api.ts"),
        r#"
export class Api {
  target() { return 1; }
  async AsyncTarget() { return 1; }
  *GeneratorTarget() { yield 1; }
  get GetterTarget() { return 1; }
  set SetterTarget(value: number) {}
  FieldTarget = 1;
}
export const object = { ObjectTarget() {}, ArrowTarget: () => 1 };
"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "member definition forms"]);

    for query in [
        "target",
        "AsyncTarget",
        "GeneratorTarget",
        "GetterTarget",
        "SetterTarget",
        "FieldTarget",
        "ObjectTarget",
        "ArrowTarget",
    ] {
        let json = run_json(
            repo.path(),
            cache.path(),
            &["where", query, "--format", "json"],
        );
        let definitions = horizon(&json["observations"], "definition_matches");
        assert_eq!(definitions["count"]["observed"], 0, "{query}: {json:#}");
        assert_eq!(
            definitions["count"]["closure"], "open",
            "an unindexed member declaration cannot prove zero for {query}: {json:#}"
        );
        assert_unsupported_file(definitions, "src/api.ts", &json);
    }
}
