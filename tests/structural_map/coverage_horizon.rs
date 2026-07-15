#[test]
fn coverage_horizon_owns_truncation_while_json_keeps_all_observed_edges() {
    let repo = TempDir::new().expect("repo tempdir");
    let readable_cache = TempDir::new().expect("readable cache");
    let json_cache = TempDir::new().expect("json cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"coverage-mass","private":true}"#,
    );
    write(
        &repo.path().join("src/target.ts"),
        "export function target(): number { return 1; }\n",
    );
    for index in 0..8 {
        write(
            &repo.path().join(format!("src/consumer-{index}.ts")),
            &format!(
                "import {{ target }} from './target';\nexport const value{index} = target();\n"
            ),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "coverage mass fixture"]);

    let markdown = run_markdown(
        repo.path(),
        readable_cache.path(),
        &["cone", "src/target.ts#target", "--limit", "2"],
    );
    assert!(
        markdown.contains("incoming: counted-at-least(8,") && markdown.contains("shown=2 hidden=6"),
        "readable projection must expose its exact truncation: {markdown}"
    );
    let artifact = cached_lens_artifact_json(readable_cache.path(), "cone-current.json");
    let bounded_ledger = &artifact["report"]["observations"];
    let bounded = horizon(bounded_ledger, "incoming");
    assert_eq!(bounded["count"]["observed"], 8);
    assert_eq!(bounded["shown"], 2);
    assert_eq!(bounded["hidden"], 6);
    assert!(
        bounded["expand"]
            .as_str()
            .is_some_and(|value| value.ends_with(" --all"))
    );
    assert_horizon_certificate_resolves(bounded_ledger, bounded);

    let json = run_json(
        repo.path(),
        json_cache.path(),
        &[
            "cone",
            "src/target.ts#target",
            "--limit",
            "2",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &json);
    assert_eq!(json["incoming"].as_array().expect("incoming").len(), 8);
    let full = horizon(&json["observations"], "incoming");
    assert_eq!(full["count"]["observed"], 8);
    assert_eq!(full["shown"], 8);
    assert_eq!(full["hidden"], 0);
    assert!(full["expand"].is_null());
    assert_eq!(
        bounded["count"]["certificate_id"], full["count"]["certificate_id"],
        "certificate identity must not depend on display limit"
    );

    let all = run_markdown(
        repo.path(),
        readable_cache.path(),
        &["cone", "src/target.ts#target", "--all", "--limit", "2"],
    );
    assert!(
        all.contains("incoming: counted-at-least(8,") && all.contains("shown=8 hidden=0"),
        "--all must make the horizon explicit rather than suppress it: {all}"
    );
}

#[test]
fn where_limit_one_never_forges_a_unique_definition() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"definition-limit","private":true}"#,
    );
    write(
        &repo.path().join("src/a.ts"),
        "export function Twin() { return 'a'; }\n",
    );
    write(
        &repo.path().join("src/b.ts"),
        "export function Twin() { return 'b'; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "two definitions"]);

    let markdown = run_markdown(
        repo.path(),
        cache.path(),
        &["where", "Twin", "--limit", "1"],
    );
    assert!(markdown.contains("Matches: `2`"), "{markdown}");
    assert!(markdown.contains("\n## Definitions\n"), "{markdown}");
    assert!(!markdown.contains("\n## Definition\n"), "{markdown}");
    assert!(!markdown.contains("## X-Ray Card"), "{markdown}");
    assert!(
        markdown.contains("definition_matches: counted(2); shown=1 hidden=1"),
        "bounded definitions need a truthful horizon: {markdown}"
    );
    assert!(
        markdown.contains("  - consumers:")
            && markdown.contains("  - incoming:")
            && markdown.contains("  - verification:"),
        "every shown definition needs all S03.a horizons in readable output: {markdown}"
    );
}

#[test]
fn warm_symbol_cone_preserves_the_complete_observation_ledger() {
    let (repo, cache) = fixture();
    let args = [
        "cone",
        "packages/replay/src/session.ts#seek",
        "--format",
        "json",
    ];
    let cold = run_json(repo.path(), cache.path(), &args);
    let warm = run_json(repo.path(), cache.path(), &args);
    assert_schema("schemas/cone.schema.json", &cold);
    assert_eq!(
        cold["observations"], warm["observations"],
        "warm fast path must not drop horizons or certificates"
    );
    for (group, field) in [("incoming", "incoming"), ("verification", "proof")] {
        assert_eq!(
            horizon(&cold["observations"], group)["count"]["observed"]
                .as_u64()
                .expect("observed"),
            cold[field].as_array().expect("serialized cone facts").len() as u64,
            "{group} count must resolve to the serialized cone facts: {cold:#}"
        );
    }
    for horizon in cold["observations"]["horizons"]
        .as_array()
        .expect("horizons")
    {
        assert_horizon_certificate_resolves(&cold["observations"], horizon);
    }
    let artifact = cached_lens_artifact_json(cache.path(), "cone-current.json");
    assert_eq!(
        artifact["report"]["observations"], cold["observations"],
        "cache artifact must carry the same complete ledger"
    );
}

#[test]
fn unique_where_serializes_the_facts_named_by_every_count() {
    let (repo, cache) = fixture();
    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "seek", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    let definition = &json["definitions"][0];
    let observations = &definition["observations"];
    for (group, list) in [
        ("consumers", &definition["consumers"]),
        ("incoming", &definition["incoming"]),
        ("verification", &definition["verification"]),
    ] {
        let horizon = horizon(observations, group);
        assert_eq!(
            horizon["count"]["observed"].as_u64().expect("observed"),
            list.as_array().expect("serialized fact list").len() as u64,
            "{group} count must resolve to the facts serialized beside it: {json:#}"
        );
        assert_horizon_certificate_resolves(observations, horizon);
    }
    let definitions = horizon(&json["observations"], "definition_matches");
    assert_eq!(
        definitions["count"]["observed"].as_u64(),
        Some(json["definitions"].as_array().expect("definitions").len() as u64)
    );
}

#[test]
fn multiple_where_definitions_still_own_all_required_horizons() {
    let repo = TempDir::new().expect("multiple definition repo");
    let cache = TempDir::new().expect("multiple definition cache");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"multiple-definition-horizons","private":true}"#,
    );
    write(
        &repo.path().join("src/a.ts"),
        "export function target() { return 'a'; }\n",
    );
    write(
        &repo.path().join("src/b.ts"),
        "export function target() { return 'b'; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "multiple exact definitions"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "target", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    assert_eq!(json["definitions"].as_array().expect("definitions").len(), 2);
    for definition in json["definitions"].as_array().expect("definitions") {
        let ledger = &definition["observations"];
        for (group, field) in [
            ("consumers", "consumers"),
            ("incoming", "incoming"),
            ("verification", "verification"),
        ] {
            let horizon = horizon(ledger, group);
            assert_eq!(
                horizon["shown"].as_u64(),
                Some(definition[field].as_array().expect("fact list").len() as u64),
                "{group} must be explicit for every definition: {json:#}"
            );
            assert_horizon_certificate_resolves(ledger, horizon);
        }
    }
}

#[test]
fn missing_definition_is_a_certificate_backed_proven_zero() {
    let (repo, cache) = fixture();
    let json = run_json(
        repo.path(),
        cache.path(),
        &["where", "NoSuchExactSymbol", "--format", "json"],
    );
    assert_schema("schemas/where.schema.json", &json);
    let definitions = horizon(&json["observations"], "definition_matches");
    assert_eq!(definitions["count"]["observed"], 0);
    assert_eq!(definitions["count"]["closure"], "closed");
    assert_horizon_certificate_resolves(&json["observations"], definitions);
}

#[test]
fn self_map_indexes_the_observation_ledger_owner() {
    let cache = TempDir::new().expect("self map cache");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let json = run_json(
        root,
        cache.path(),
        &["where", "ObservationLedger", "--format", "json"],
    );
    assert_eq!(json["total_matches"], 1, "{json:#}");
    assert_eq!(
        json["definitions"][0]["anchor"]["path"],
        "src/model/coverage_ledger.rs#ObservationLedger",
        "the evidence owner must not live under a globally ignored directory: {json:#}"
    );
}

fn horizon<'a>(ledger: &'a Value, group: &str) -> &'a Value {
    ledger["horizons"]
        .as_array()
        .expect("horizons")
        .iter()
        .find(|horizon| horizon["group"] == group)
        .unwrap_or_else(|| panic!("missing {group} horizon: {ledger:#}"))
}

fn assert_horizon_certificate_resolves(ledger: &Value, horizon: &Value) {
    let id = horizon["count"]["certificate_id"]
        .as_str()
        .expect("certificate id");
    assert_eq!(
        ledger["certificates"][id]["id"], id,
        "horizon has a dangling certificate: {ledger:#}"
    );
    assert_eq!(
        ledger["certificates"][id]["closure"], horizon["count"]["closure"],
        "horizon and certificate closure diverged: {ledger:#}"
    );
}
