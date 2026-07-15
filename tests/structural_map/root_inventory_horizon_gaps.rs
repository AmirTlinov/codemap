// Responsibility: root-inventory-horizon-adversarial-gaps
#[test]
fn malformed_package_candidate_keeps_the_package_horizon_open() {
    let repo = TempDir::new().expect("malformed package repo");
    let cache = TempDir::new().expect("malformed package cache");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"valid-root","private":true}"#,
    );
    write(&repo.path().join("bad/package.json"), "{\n");
    write(&repo.path().join("README.md"), "# malformed candidate\n");

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let ledger = &json["observations"];
    let packages = horizon(ledger, "packages");
    assert_eq!(packages["count"]["observed"], 1, "{json:#}");
    assert_eq!(packages["count"]["closure"], "open", "{json:#}");
    assert!(
        packages["count"]["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|reason| reason == "unsupported_construct"),
        "a parse gap must remain typed: {json:#}"
    );
    let certificate =
        &ledger["certificates"][packages["count"]["certificate_id"].as_str().expect("id")];
    assert_eq!(certificate["eligible_files"], 2, "{json:#}");
    assert_eq!(certificate["visited_files"], 1, "{json:#}");
    assert!(
        certificate["unsupported"]
            .as_array()
            .expect("unsupported")
            .iter()
            .any(|gap| gap["file"] == "bad/package.json"),
        "the rejected manifest must stay in the certificate: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, packages);
}

#[test]
fn unavailable_source_body_cannot_become_a_closed_test_zero() {
    let repo = TempDir::new().expect("unsupported test repo");
    let cache = TempDir::new().expect("unsupported test cache");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("root.mts"), &"x".repeat(901_000));
    write(&repo.path().join("README.md"), "# unsupported test\n");

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let ledger = &json["observations"];
    for group in ["directory_surfaces", "test_surfaces"] {
        let observation = horizon(ledger, group);
        assert_eq!(
            observation["count"]["closure"], "open",
            "{group}: {json:#}"
        );
        assert!(
            observation["unsupported"]
                .as_array()
                .expect("unsupported")
                .iter()
                .any(|gap| gap["file"] == "root.mts"),
            "{group} must preserve the unavailable role candidate: {json:#}"
        );
        assert_horizon_certificate_resolves(ledger, observation);
    }
    assert_eq!(
        horizon(ledger, "test_surfaces")["count"]["observed"],
        0,
        "the lower bound is zero, but it is not proven-zero: {json:#}"
    );
}

#[test]
fn full_json_serializes_every_member_hidden_by_a_readable_aggregate() {
    let repo = TempDir::new().expect("package mass repo");
    let readable_cache = TempDir::new().expect("package mass readable cache");
    let json_cache = TempDir::new().expect("package mass json cache");
    git(repo.path(), &["init", "-q"]);
    for index in 0..7 {
        write(
            &repo.path().join(format!("apps/app-{index}/package.json")),
            &format!(r#"{{"name":"app-{index}","private":true}}"#),
        );
    }

    let markdown = run_markdown(repo.path(), readable_cache.path(), &["ls", "."]);
    assert!(
        markdown.contains("- packages: counted(7); shown=5 hidden=2"),
        "readable package members must have one exact hidden remainder: {markdown}"
    );

    let json = run_json(
        repo.path(),
        json_cache.path(),
        &["ls", ".", "--format", "json"],
    );
    let packages = horizon(&json["observations"], "packages");
    assert_eq!(packages["count"]["observed"], 7, "{json:#}");
    assert_eq!(packages["shown"], 7, "{json:#}");
    assert_eq!(packages["hidden"], 0, "{json:#}");
    let surface = json["directory"]
        .as_array()
        .expect("directory")
        .iter()
        .find(|surface| surface["kind"] == "package:javascript")
        .expect("package surface");
    assert_eq!(surface["examples"].as_array().expect("examples").len(), 7);
    assert_eq!(surface["hidden_count"], 0, "{json:#}");
}

#[test]
fn corrupted_ls_report_body_misses_instead_of_serving_false_horizons() {
    let repo = root_inventory_horizon_fixture();
    let cache = TempDir::new().expect("corrupt ls cache");
    let cold = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let path = lens_artifact_path(cache.path(), "ls-current.json");
    let mut artifact: Value = serde_json::from_str(
        &fs::read_to_string(&path).expect("read cached ls artifact"),
    )
    .expect("cached ls json");
    artifact["report"]["directory"][0]["examples"][0] =
        Value::String("SENTINEL-CORRUPTION".to_string());
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&artifact).expect("serialize corrupt cache")
        ),
    )
    .expect("write corrupt cache");

    let repaired = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_eq!(
        repaired["observations"], cold["observations"],
        "cache corruption must rebuild the certified projection"
    );
    assert!(
        repaired["directory"]
            .as_array()
            .expect("directory")
            .iter()
            .flat_map(|surface| surface["examples"].as_array().into_iter().flatten())
            .all(|example| example != "SENTINEL-CORRUPTION"),
        "a coherent-looking but hash-invalid report body must never be served: {repaired:#}"
    );
}
