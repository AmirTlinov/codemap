// Responsibility: runtime-group-horizon-pilot-matrix
#[test]
fn runtime_group_horizons_certify_every_remaining_group() {
    let repo = runtime_group_horizon_fixture();
    let cache = TempDir::new().expect("runtime group horizon cache");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &json);
    let ledger = &json["observations"];
    assert_eq!(
        ledger["horizons"].as_array().expect("horizons").len(),
        8,
        "the runtime ledger must carry exactly one horizon per fact group: {json:#}"
    );

    let entrypoints = horizon(ledger, "entrypoints");
    assert_eq!(entrypoints["count"]["observed"], 2, "{json:#}");
    assert_eq!(entrypoints["count"]["closure"], "open", "{json:#}");
    assert!(
        entrypoints["count"]["reasons"]
            .as_array()
            .expect("entrypoint reasons")
            .iter()
            .any(|reason| reason == "unsupported_construct"),
        "incomplete entrypoint extractor capability must stay a typed open: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, entrypoints);

    let scripts = horizon(ledger, "scripts");
    assert_eq!(scripts["count"]["observed"], 0, "{json:#}");
    assert_eq!(scripts["count"]["closure"], "open", "{json:#}");
    assert!(
        scripts["count"]["reasons"]
            .as_array()
            .expect("script reasons")
            .iter()
            .any(|reason| reason == "incomplete_traversal"),
        "a root-only script catalog cannot prove zero for a nested scope: {json:#}"
    );
    let scripts_certificate =
        &ledger["certificates"][scripts["count"]["certificate_id"].as_str().expect("id")];
    assert_eq!(scripts_certificate["eligible_files"], 1, "{json:#}");
    assert_eq!(scripts_certificate["visited_files"], 0, "{json:#}");
    assert!(
        scripts_certificate["excluded_files_by_reason"]["incomplete_traversal"]
            .as_array()
            .expect("unvisited script manifests")
            .iter()
            .any(|file| file == "src/nested/package.json"),
        "the unvisited nested manifest must be an exact exclusion: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, scripts);

    let env = horizon(ledger, "env");
    assert_eq!(env["count"]["observed"], 1, "{json:#}");
    assert_eq!(env["count"]["closure"], "open", "{json:#}");
    for reason in ["dynamic_env_lookup", "unsupported_language"] {
        assert!(
            env["count"]["reasons"]
                .as_array()
                .expect("env reasons")
                .iter()
                .any(|value| value == reason),
            "env horizon needs typed reason {reason}: {json:#}"
        );
    }
    assert_eq!(
        env["dynamic"][0]["kind"], "dynamic_env_lookup",
        "the computed env key must become an exact dynamic stop: {json:#}"
    );
    assert_eq!(env["dynamic"][0]["location"]["path"], "src/env.ts", "{json:#}");
    assert!(
        env["unsupported"]
            .as_array()
            .expect("unsupported env files")
            .iter()
            .any(|observation| observation["file"] == "src/api/main.go"),
        "a language outside the env grammar must stay an exact unsupported surface: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, env);

    for group in ["workers", "ci"] {
        let inventory = horizon(ledger, group);
        assert_eq!(inventory["count"]["closure"], "closed", "{group}: {json:#}");
        assert_eq!(
            inventory["count"]["reasons"],
            serde_json::json!([]),
            "{group}: {json:#}"
        );
        let certificate =
            &ledger["certificates"][inventory["count"]["certificate_id"].as_str().expect("id")];
        assert_eq!(
            certificate["eligible_files"], certificate["visited_files"],
            "{group} closes only with exact candidate inventory accounting: {json:#}"
        );
        assert!(
            certificate["dynamic_stops"].as_array().expect("stops").is_empty()
                && certificate["unresolved_stops"]
                    .as_array()
                    .expect("stops")
                    .is_empty()
                && certificate["external_stops"]
                    .as_array()
                    .expect("stops")
                    .is_empty(),
            "{group} closure forbids unresolved/dynamic/external stops: {json:#}"
        );
        assert_horizon_certificate_resolves(ledger, inventory);
    }
    assert_eq!(horizon(ledger, "workers")["count"]["observed"], 1, "{json:#}");
    assert_eq!(horizon(ledger, "ci")["count"]["observed"], 0, "{json:#}");

    let proof = horizon(ledger, "proof");
    assert_eq!(proof["count"]["observed"], 0, "{json:#}");
    assert_eq!(proof["count"]["closure"], "open", "{json:#}");
    assert_eq!(
        proof["count"]["reasons"],
        serde_json::json!(["verification_relation_flow"]),
        "unresolved route/verification relations keep the proof zero open: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, proof);

    let unknowns = horizon(ledger, "unknowns");
    assert_eq!(unknowns["count"]["observed"], 1, "{json:#}");
    assert_eq!(unknowns["count"]["closure"], "open", "{json:#}");
    assert!(
        unknowns["count"]["reasons"]
            .as_array()
            .expect("unknown reasons")
            .iter()
            .any(|reason| reason == "unsupported_construct"),
        "the unknown detector surface itself has unsupported boundaries: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, unknowns);
}

#[test]
fn bounded_readable_runtime_groups_share_the_machine_certificates() {
    let repo = runtime_group_horizon_fixture();
    let readable_cache = TempDir::new().expect("readable runtime group cache");
    let json_cache = TempDir::new().expect("json runtime group cache");

    let markdown = run_markdown(
        repo.path(),
        readable_cache.path(),
        &["runtime", "src", "--limit", "1"],
    );
    assert!(
        markdown.contains("- entrypoints: counted-at-least(2, unsupported construct); shown=1 hidden=1")
            && markdown.contains("codemap runtime src --all --limit 2"),
        "bounded readable entrypoints must expose their horizon and expansion: {markdown}"
    );
    assert!(
        markdown.contains("- workers: counted(1); shown=1 hidden=0")
            && markdown.contains("- ci: proven-zero; shown=0 hidden=0")
            && markdown.contains("- scripts: unknown lower bound: 0")
            && markdown.contains("- proof: unknown lower bound: 0 (verification relations partial)"),
        "every runtime group must print its own certified visibility row: {markdown}"
    );
    for legacy in [
        "runtime entrypoints hidden by limit",
        "runtime scripts hidden by limit",
        "environment surfaces hidden by limit",
        "worker/job surfaces hidden by limit",
        "ci surfaces hidden by limit",
        "runtime verification edges hidden by limit",
        "runtime unknowns hidden by limit",
    ] {
        assert!(
            !markdown.contains(legacy),
            "legacy detached hidden group `{legacy}` must not duplicate the horizon: {markdown}"
        );
    }

    let json = run_json(
        repo.path(),
        json_cache.path(),
        &["runtime", "src", "--format", "json"],
    );
    let readable_certificates = markdown
        .lines()
        .filter_map(|line| line.split_once(": ").map(|(group, _)| (group, line)))
        .filter_map(|(group, line)| {
            let certificate = line.split_once("cert=`")?.1.split_once('`')?.0;
            Some((group.trim_start_matches("- ").to_string(), certificate))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        readable_certificates.len(),
        8,
        "readable runtime output must show all eight group certificates: {markdown}"
    );
    for horizon in json["observations"]["horizons"]
        .as_array()
        .expect("runtime horizons")
    {
        let group = horizon["group"].as_str().expect("group");
        let readable_digest = readable_certificates[group]
            .strip_prefix("v1:")
            .expect("compact certificate");
        assert!(
            horizon["count"]["certificate_id"]
                .as_str()
                .expect("certificate id")
                .strip_prefix("coverage-v1:")
                .is_some_and(|digest| digest.starts_with(readable_digest)),
            "readable and JSON projections must resolve the same {group} certificate: {json:#}"
        );
    }
}

#[test]
fn unindexed_scope_leaves_every_runtime_group_unavailable() {
    let repo = runtime_group_horizon_fixture();
    let cache = TempDir::new().expect("unindexed runtime group cache");
    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "not-an-indexed-scope", "--format", "json"],
    );
    let ledger = &json["observations"];
    for group in [
        "entrypoints",
        "routes",
        "scripts",
        "env",
        "workers",
        "ci",
        "proof",
        "unknowns",
    ] {
        let unavailable = horizon(ledger, group);
        assert_eq!(
            unavailable["count"]["closure"], "unavailable",
            "{group}: an absent scope proves nothing: {json:#}"
        );
        assert_eq!(
            unavailable["count"]["reasons"],
            serde_json::json!(["anchor_not_indexed"]),
            "{group}: {json:#}"
        );
        assert_horizon_certificate_resolves(ledger, unavailable);
    }
}

#[test]
fn runtime_root_cache_preserves_all_group_horizons_on_warm_read() {
    let (repo, cache) = runtime_root_cache_fixture();

    let cold = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let cold_artifact = runtime_root_cache_json(cache.path());
    let warm = run_markdown(repo.path(), cache.path(), &["runtime", "."]);
    let warm_artifact = runtime_root_cache_json(cache.path());

    assert_lens_markdown_eq(
        &cold,
        &warm,
        "the warm runtime root must preserve every cold group horizon",
    );
    assert_eq!(
        warm_artifact["report"]["observations"], cold_artifact["report"]["observations"],
        "the persisted group ledger must survive a warm cache read unchanged",
    );
    assert_eq!(
        cold_artifact["report"]["observations"]["horizons"]
            .as_array()
            .expect("cached horizons")
            .len(),
        8,
        "the trusted-local runtime root must persist all eight group horizons: {cold_artifact:#}"
    );
}

fn runtime_group_horizon_fixture() -> TempDir {
    let repo = TempDir::new().expect("runtime group horizon repo");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/api/main.go"),
        "package main\n\nfunc main() {}\n",
    );
    write(&repo.path().join("src/app/main.py"), "def main():\n    return True\n");
    write(
        &repo.path().join("src/worker/jobs/email.ts"),
        "const queue = process.env.QUEUE_URL;\nexport function runEmailJob() { return queue; }\n",
    );
    write(
        &repo.path().join("src/env.ts"),
        "const key = resolveKey();\nexport const value = process.env[key];\n",
    );
    write(
        &repo.path().join("src/nested/package.json"),
        r#"{"name":"nested-fixture","private":true,"scripts":{"test":"vitest run"}}"#,
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime group horizon fixture"]);
    repo
}
