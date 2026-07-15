// Responsibility: scanner-boundary coverage certificates

#[cfg(unix)]
#[test]
fn tracked_source_symlink_is_a_typed_cold_and_warm_coverage_boundary() {
    use std::os::unix::fs::symlink;

    let workspace = TempDir::new().expect("symlink coverage workspace");
    let repo = workspace.path().join("repo");
    let cache = TempDir::new().expect("symlink coverage cache");
    fs::create_dir_all(repo.join("src")).expect("source directory");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "a@example.com"]);
    git(&repo, &["config", "user.name", "a"]);
    write(
        &repo.join("package.json"),
        r#"{"name":"symlink-coverage","private":true}"#,
    );
    write(
        &repo.join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    let external = workspace.path().join("external.ts");
    write(
        &external,
        "import { target } from './repo/src/target';\nconst late = require(selectModule());\nexport function HiddenViaLink() { return target() + late; }\n",
    );
    symlink(&external, repo.join("src/linked.ts")).expect("source symlink");
    git(&repo, &["add", "package.json", "src"]);
    git(&repo, &["commit", "-qm", "tracked source symlink"]);

    for _ in 0..2 {
        let missing = run_json(
            &repo,
            cache.path(),
            &["where", "HiddenViaLink", "--format", "json"],
        );
        assert_eq!(
            missing["total_matches"], 0,
            "symlink contents were followed: {missing:#}"
        );
        let definitions = horizon(&missing["observations"], "definition_matches");
        assert_eq!(definitions["count"]["closure"], "open", "{missing:#}");
        assert_unsupported_file(definitions, "src/linked.ts", &missing);

        let consumer = run_json(
            &repo,
            cache.path(),
            &["where", "target", "--format", "json"],
        );
        let definition = &consumer["definitions"][0];
        assert_eq!(definition["consumers_total"]["observed"], 0, "{consumer:#}");
        assert_eq!(
            definition["consumers_total"]["closure"], "open",
            "{consumer:#}"
        );
        let consumers = horizon(&definition["observations"], "consumers");
        assert_unsupported_file(consumers, "src/linked.ts", &consumer);
        assert!(
            consumers["dynamic"]
                .as_array()
                .expect("dynamic stops")
                .is_empty(),
            "external symlink contents must not be inspected: {consumer:#}"
        );
    }
}

fn assert_unsupported_file(horizon: &Value, expected: &str, report: &Value) {
    assert!(
        horizon["unsupported"]
            .as_array()
            .expect("unsupported coverage files")
            .iter()
            .any(|item| item["file"] == expected),
        "`{expected}` must remain a typed coverage boundary: {report:#}"
    );
}

#[test]
fn node_typescript_module_extensions_are_typed_placeholders_not_false_zeroes() {
    let repo = TempDir::new().expect("Node TypeScript module repo");
    let cache = TempDir::new().expect("Node TypeScript module cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"node-typescript-modules","private":true,"type":"module"}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/owner.mts"),
        "export function ModernMtsNeedle() { return 1; }\n",
    );
    write(
        &repo.path().join("src/owner.cts"),
        "export function ModernCtsNeedle() { return 1; }\n",
    );
    write(
        &repo.path().join("src/consumer.mts"),
        "import alias from './target';\nexport const value = alias();\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "Node TypeScript module extensions"],
    );

    for (query, owner) in [
        ("ModernMtsNeedle", "src/owner.mts"),
        ("ModernCtsNeedle", "src/owner.cts"),
    ] {
        let json = run_json(
            repo.path(),
            cache.path(),
            &["where", query, "--format", "json"],
        );
        assert_eq!(
            json["total_matches"], 0,
            "full parsing was not added: {json:#}"
        );
        let definitions = horizon(&json["observations"], "definition_matches");
        assert_eq!(definitions["count"]["closure"], "open", "{json:#}");
        assert_unsupported_file(definitions, owner, &json);
    }

    let target = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    assert_eq!(target["definitions"][0]["consumers_total"]["observed"], 0);
    assert_eq!(
        target["definitions"][0]["consumers_total"]["closure"],
        "open"
    );
}
