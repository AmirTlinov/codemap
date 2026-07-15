// Responsibility: lexical-consumer-coverage-boundary-regressions

#[test]
fn a_semicolonless_bare_alias_does_not_disappear_from_consumer_coverage() {
    let repo = TempDir::new().expect("bare alias consumer repo");
    let cache = TempDir::new().expect("bare alias consumer cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"bare-alias-consumer","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/use.ts"),
        "import { target } from './target';\nexport function observed() { target(); }\nexport const alias = target\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "semicolonless bare alias"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    let count = &json["definitions"][0]["consumers_total"];
    assert_eq!(count["observed"], 1, "{json:#}");
    assert_eq!(
        count["closure"], "open",
        "a second bare value reference cannot be hidden by EOF: {json:#}"
    );
}
