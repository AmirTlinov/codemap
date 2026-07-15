#[test]
fn unsupported_javascript_binding_forms_keep_consumer_coverage_open() {
    let repo = TempDir::new().expect("consumer gap repo");
    let cache = TempDir::new().expect("consumer gap cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"consumer-gaps","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/namespace.ts"),
        "import * as api from './target';\nexport const namespaceValue = api.target();\n",
    );
    write(
        &repo.path().join("src/one-line.ts"),
        "import { target } from './target'; export const oneLineValue = target();\n",
    );
    write(
        &repo.path().join("src/commonjs.ts"),
        "const { target } = require('./target'); export const commonValue = target();\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "unsupported consumer bindings"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    let definition = &json["definitions"][0];
    let count = &definition["consumers_total"];
    assert_eq!(count["observed"], 0, "S04 resolution must not be forged: {json:#}");
    assert_eq!(count["closure"], "open", "blind bindings cannot prove zero: {json:#}");
    assert!(
        count["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|reason| reason == "unsupported_construct"),
        "the gap needs a typed reason: {json:#}"
    );
    let id = count["certificate_id"].as_str().expect("certificate id");
    let certificate = &definition["observations"]["certificates"][id];
    let constructs = certificate["unsupported"]
        .as_array()
        .expect("unsupported")
        .iter()
        .filter_map(|item| item["construct"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for expected in [
        "namespace_import_member_binding",
        "unscoped_static_import_reference",
        "commonjs_or_unbound_static_import",
    ] {
        assert!(
            constructs.contains(expected),
            "missing `{expected}` in certificate: {json:#}"
        );
    }
    assert_horizon_certificate_resolves(&definition["observations"], horizon(&definition["observations"], "consumers"));
}

#[test]
fn an_observed_consumer_stays_a_lower_bound_when_another_binding_is_unresolved() {
    let repo = TempDir::new().expect("consumer lower-bound repo");
    let cache = TempDir::new().expect("consumer lower-bound cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"consumer-lower-bound","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/direct.ts"),
        "import { target } from './target';\nexport function direct() { return target(); }\n",
    );
    write(
        &repo.path().join("src/namespace.ts"),
        "import * as api from './target';\nexport function mediated() { return api.target(); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "partial consumer observation"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    let count = &json["definitions"][0]["consumers_total"];
    assert_eq!(count["observed"], 1, "direct fact must survive: {json:#}");
    assert_eq!(count["closure"], "open", "namespace path stays unresolved: {json:#}");
}

#[test]
fn partially_supported_language_does_not_claim_symbol_consumer_closure() {
    let repo = TempDir::new().expect("partial language repo");
    let cache = TempDir::new().expect("partial language cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("pyproject.toml"),
        "[project]\nname = \"partial-language\"\nversion = \"0.1.0\"\n",
    );
    write(
        &repo.path().join("src/owner.py"),
        "def orphan_helper():\n    return 1\n",
    );
    write(
        &repo.path().join("src/other.py"),
        "def unrelated():\n    return 2\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "partial language fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "orphan_helper", "--format", "json"],
    );
    let definition = &json["definitions"][0];
    assert_eq!(definition["consumers_total"]["observed"], 0, "{json:#}");
    assert_eq!(definition["consumers_total"]["closure"], "open", "{json:#}");
    let id = definition["consumers_total"]["certificate_id"]
        .as_str()
        .expect("certificate id");
    assert!(
        definition["observations"]["certificates"][id]["unsupported"]
            .as_array()
            .expect("unsupported")
            .iter()
            .any(|item| item["construct"] == "partial_python_symbol_consumer_closure"),
        "the certificate must name the unclosed language surface: {json:#}"
    );
}
