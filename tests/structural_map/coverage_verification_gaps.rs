#[test]
fn verification_certificate_uses_real_test_candidates_and_names_unsupported_files() {
    let repo = TempDir::new().expect("verification gap repo");
    let cache = TempDir::new().expect("verification gap cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"verification-gap","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("tests/target.kt"),
        "fun targetContract() { check(true) }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "verification gap fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["cone", "src/target.ts#target", "--format", "json"],
    );
    assert_schema("schemas/cone.schema.json", &json);
    let ledger = &json["observations"];
    let verification = horizon(ledger, "verification");
    let id = verification["count"]["certificate_id"]
        .as_str()
        .expect("certificate id");
    let certificate = &ledger["certificates"][id];
    assert_eq!(certificate["eligible_files"], 1, "only test candidates are eligible: {json:#}");
    assert_eq!(certificate["visited_files"], 0, "unsupported Kotlin was not traversed: {json:#}");
    let reasons = certificate["reasons"].as_array().expect("typed reasons");
    assert!(reasons.iter().any(|reason| reason == "unsupported_language"), "{json:#}");
    assert!(
        !reasons.iter().any(|reason| reason == "unsupported_construct"),
        "a language exclusion must not be relabeled as a construct gap: {json:#}"
    );
    assert!(
        certificate["unsupported"]
            .as_array()
            .expect("unsupported")
            .iter()
            .any(|item| item["file"] == "tests/target.kt"),
        "unsupported verification file must be explicit: {json:#}"
    );
    assert!(
        certificate["extractor_capabilities"]
            .as_array()
            .expect("capabilities")
            .is_empty(),
        "package.json/config must not mint a verification capability: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, verification);
}

#[test]
fn test_support_static_use_is_a_visible_verification_fact() {
    let repo = TempDir::new().expect("test support repo");
    let cache = TempDir::new().expect("test support cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"test-support-use","type":"module"}"#,
    );
    write(
        &repo.path().join("src/target.js"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("tests/helpers/setup.js"),
        "import { target } from '../../src/target.js';\nexport const fixture = target();\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "test support use fixture"]);

    let where_json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &where_json);
    let definition = &where_json["definitions"][0];
    let verification_facts = definition["verification"]
        .as_array()
        .expect("where verification facts");
    assert_eq!(verification_facts.len(), 1, "{where_json:#}");
    assert_eq!(
        verification_facts[0]["from"],
        "tests/helpers/setup.js",
        "the static support import must not disappear between count classes: {where_json:#}"
    );
    assert_eq!(verification_facts[0]["evidence"], "test_support_import");
    assert_eq!(
        verification_facts[0]["type"],
        "setup_support_surface",
        "support code is visible without being presented as a runnable test: {where_json:#}"
    );
    let verification = horizon(&definition["observations"], "verification");
    assert_eq!(verification["count"]["observed"], 1, "{where_json:#}");
    assert_eq!(verification["shown"], 1, "{where_json:#}");
    let certificate_id = verification["count"]["certificate_id"]
        .as_str()
        .expect("verification certificate");
    let certificate = &definition["observations"]["certificates"][certificate_id];
    assert_eq!(certificate["eligible_files"], 1, "{where_json:#}");
    assert_eq!(certificate["visited_files"], 1, "{where_json:#}");
    assert_horizon_certificate_resolves(&definition["observations"], verification);

    let cone_json = run_json(
        repo.path(),
        cache.path(),
        &[
            "cone",
            "src/target.js#target",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &cone_json);
    assert_eq!(cone_json["proof"], definition["verification"]);
    assert_eq!(
        horizon(&cone_json["observations"], "verification")["count"]["certificate_id"],
        certificate_id,
        "where and cone must share the same support-aware verification basis"
    );
}
