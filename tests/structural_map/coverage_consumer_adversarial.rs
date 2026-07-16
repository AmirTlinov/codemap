#[test]
fn same_file_incoming_and_external_consumers_have_distinct_horizons() {
    let repo = TempDir::new().expect("local incoming repo");
    let cache = TempDir::new().expect("local incoming cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"local-incoming","private":true}"#,
    );
    write(
        &repo.path().join("src/owner.ts"),
        "export function target() { return 1; } export function caller() { return target(); }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "same file consumer"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    let definition = &json["definitions"][0];
    let consumers = horizon(&definition["observations"], "consumers");
    let incoming = horizon(&definition["observations"], "incoming");
    assert_eq!(consumers["count"]["closure"], "closed", "{json:#}");
    assert_eq!(incoming["count"]["closure"], "open", "{json:#}");
    assert_ne!(
        consumers["count"]["certificate_id"], incoming["count"]["certificate_id"],
        "local incoming and external consumers are different universes: {json:#}"
    );
    let incoming_id = incoming["count"]["certificate_id"]
        .as_str()
        .expect("incoming certificate");
    let certificate = &definition["observations"]["certificates"][incoming_id];
    assert_eq!(certificate["query_kind"], "symbol_incoming_relations");
    assert_eq!(
        certificate["excluded_files_by_reason"]["unsupported_construct"]
            .as_array()
            .expect("typed exclusions"),
        &[serde_json::json!("src/owner.ts")],
        "every eligible-but-unvisited file needs a typed exclusion: {json:#}"
    );
    assert!(
        certificate["unsupported"]
            .as_array()
            .expect("unsupported")
            .iter()
            .any(|item| item["construct"] == "same_file_symbol_reference_closure"),
        "the unclosed same-file grammar must be explicit: {json:#}"
    );
}

#[test]
fn unresolved_alias_and_amd_candidates_cannot_prove_consumer_zero() {
    let repo = TempDir::new().expect("module gap repo");
    let cache = TempDir::new().expect("module gap cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r##"{"name":"module-gaps","private":true,"imports":{"#owner":"./src/target.ts"}}"##,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/alias.ts"),
        "import { target } from '#owner';\nexport const value = target();\n",
    );
    write(
        &repo.path().join("src/amd.ts"),
        "define(['./target'], function(api) { return api.target(); });\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "unsupported module systems"],
    );

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    let definition = &json["definitions"][0];
    assert_eq!(definition["consumers_total"]["observed"], 0, "{json:#}");
    assert_eq!(definition["consumers_total"]["closure"], "open", "{json:#}");
    let id = definition["consumers_total"]["certificate_id"]
        .as_str()
        .expect("consumer certificate");
    let constructs = definition["observations"]["certificates"][id]["unsupported"]
        .as_array()
        .expect("unsupported")
        .iter()
        .filter_map(|item| item["construct"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        constructs.contains("unresolved_static_import_target"),
        "{json:#}"
    );
    assert!(
        constructs.contains("unsupported_static_module_system"),
        "{json:#}"
    );
}

#[test]
fn computed_commonjs_require_is_a_typed_dynamic_stop() {
    let repo = TempDir::new().expect("dynamic require repo");
    let cache = TempDir::new().expect("dynamic require cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"dynamic-require","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/dynamic.ts"),
        "const api = require('./' + 'target'); export const value = api.target();\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "computed require"]);

    for _ in 0..2 {
        let json = run_json(
            repo.path(),
            cache.path(),
            &["where", "target", "--format", "json"],
        );
        let definition = &json["definitions"][0];
        let id = definition["consumers_total"]["certificate_id"]
            .as_str()
            .expect("consumer certificate");
        let certificate = &definition["observations"]["certificates"][id];
        assert_eq!(definition["consumers_total"]["closure"], "open", "{json:#}");
        assert!(
            certificate["dynamic_stops"]
                .as_array()
                .expect("dynamic stops")
                .iter()
                .any(|stop| stop["kind"] == "dynamic_import_flow"),
            "computed require must survive cold and warm indexes: {json:#}"
        );
    }
}

#[test]
fn runtime_generated_javascript_cannot_prove_a_consumer_zero() {
    let repo = TempDir::new().expect("generated code repo");
    let cache = TempDir::new().expect("generated code cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"runtime-generated","private":true,"type":"module"}"#,
    );
    write(
        &repo.path().join("src/target.js"),
        "export function needle() { return 1; }\n",
    );
    write(
        &repo.path().join("src/eval.js"),
        r#"eval("import('./target.js').then(api => api.needle())");"#,
    );
    write(
        &repo.path().join("src/indirect-eval.js"),
        r#"(0, eval)("import('./target.js').then(api => api.needle())");"#,
    );
    write(
        &repo.path().join("src/optional-eval.js"),
        r#"eval?.("import('./target.js').then(api => api.needle())");"#,
    );
    write(
        &repo.path().join("src/computed-eval.js"),
        r#"globalThis["eval"] ("import('./target.js').then(api => api.needle())");"#,
    );
    write(
        &repo.path().join("src/function.js"),
        r#"new Function("return import('./target.js').then(api => api.needle())")();"#,
    );
    write(
        &repo.path().join("src/timer.js"),
        r#"setTimeout("import('./target.js').then(api => api.needle())", 0);"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "opaque runtime consumers"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "needle", "--format", "json"],
    );
    let definition = &json["definitions"][0];
    assert_eq!(definition["consumers_total"]["observed"], 0, "{json:#}");
    assert_eq!(
        definition["consumers_total"]["closure"], "open",
        "runtime-generated source is an unknown boundary, never a zero: {json:#}"
    );
    let id = definition["consumers_total"]["certificate_id"]
        .as_str()
        .expect("consumer certificate");
    let constructs = definition["observations"]["certificates"][id]["unsupported"]
        .as_array()
        .expect("unsupported runtime constructs")
        .iter()
        .filter_map(|item| item["construct"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        constructs,
        std::collections::BTreeSet::from([
            "eval_generated_code",
            "function_constructor_generated_code",
            "string_timer_generated_code",
        ]),
        "{json:#}"
    );
}

#[test]
fn an_observed_symbol_body_does_not_hide_an_unscoped_use_in_the_same_file() {
    let repo = TempDir::new().expect("mixed scope consumer repo");
    let cache = TempDir::new().expect("mixed scope consumer cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"mixed-consumer-scope","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/use.ts"),
        "import { target } from './target';\nexport function observed() { target(); }\ntarget();\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "mixed scope consumer"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    let count = &json["definitions"][0]["consumers_total"];
    assert_eq!(count["observed"], 1, "the body use remains observed: {json:#}");
    assert_eq!(
        count["closure"], "open",
        "a file-level edge cannot close an unscoped occurrence: {json:#}"
    );
}

#[test]
fn an_observed_symbol_body_does_not_hide_a_same_line_unscoped_use() {
    let repo = TempDir::new().expect("same-line mixed scope repo");
    let cache = TempDir::new().expect("same-line mixed scope cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"same-line-mixed-scope","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/use.ts"),
        "import { target } from './target';\nexport function observed() { target(); } target();\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "same-line mixed scope"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    let count = &json["definitions"][0]["consumers_total"];
    assert_eq!(count["observed"], 1, "{json:#}");
    assert_eq!(
        count["closure"], "open",
        "a line-based symbol range cannot close a trailing top-level use: {json:#}"
    );
}

#[test]
fn a_same_line_variable_symbol_does_not_hide_a_second_statement() {
    let repo = TempDir::new().expect("same-line variable scope repo");
    let cache = TempDir::new().expect("same-line variable scope cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"same-line-variable-scope","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    write(
        &repo.path().join("src/use.ts"),
        "import { target } from './target';\nexport const observed = target(); target();\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "same-line variable boundary"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    let count = &json["definitions"][0]["consumers_total"];
    assert_eq!(count["observed"], 1, "{json:#}");
    assert_eq!(
        count["closure"], "open",
        "one line-range edge cannot prove the second statement was accounted: {json:#}"
    );
}

#[cfg(unix)]
#[test]
fn unreadable_consumer_candidate_is_excluded_from_visited_coverage() {
    use std::os::unix::fs::PermissionsExt;

    let repo = TempDir::new().expect("unreadable consumer repo");
    let cache = TempDir::new().expect("unreadable consumer cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"unreadable"}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target() { return 1; }\n",
    );
    let unreadable = repo.path().join("src/candidate.ts");
    write(&unreadable, "export const candidate = 1;\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "unreadable candidate"]);
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o0))
        .expect("remove read permission");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o644))
        .expect("restore read permission");
    let definition = &json["definitions"][0];
    let id = definition["consumers_total"]["certificate_id"]
        .as_str()
        .expect("consumer certificate");
    let certificate = &definition["observations"]["certificates"][id];
    assert!(
        certificate["visited_files"].as_u64() < certificate["eligible_files"].as_u64(),
        "unreadable candidates cannot count as visited: {json:#}"
    );
    assert!(
        certificate["excluded_files_by_reason"]["unsupported_construct"]
            .as_array()
            .expect("excluded unreadable files")
            .iter()
            .any(|path| path == "src/candidate.ts"),
        "the excluded file must be named: {json:#}"
    );
}
