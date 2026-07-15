// Responsibility: runtime-route-scope-and-placeholder-boundaries
#[test]
fn unindexed_runtime_scope_is_unavailable_instead_of_proven_zero() {
    let repo = TempDir::new().expect("missing runtime scope repo");
    let cache = TempDir::new().expect("missing runtime scope cache");
    initialize_runtime_coverage_repo(&repo);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "missing runtime scope fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &[
            "runtime",
            "definitely-not-an-indexed-scope",
            "--format",
            "json",
        ],
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["observed"], 0, "{json:#}");
    assert_eq!(routes["count"]["closure"], "unavailable", "{json:#}");
    assert_eq!(
        routes["count"]["reasons"],
        serde_json::json!(["anchor_not_indexed"]),
        "an absent scope cannot prove route absence: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);
}

#[test]
fn existing_empty_runtime_directory_is_a_valid_proven_zero_scope() {
    let repo = TempDir::new().expect("empty runtime scope repo");
    let cache = TempDir::new().expect("empty runtime scope cache");
    initialize_runtime_coverage_repo(&repo);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "empty runtime scope fixture"]);
    fs::create_dir_all(repo.path().join("empty-scope")).expect("empty runtime directory");

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "empty-scope", "--format", "json"],
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["observed"], 0, "{json:#}");
    assert_eq!(routes["count"]["closure"], "closed", "{json:#}");
    assert_eq!(routes["count"]["reasons"], serde_json::json!([]));
    assert_horizon_certificate_resolves(ledger, routes);
}

#[test]
fn unsupported_regular_placeholder_timestamp_does_not_change_its_certificate() {
    let repo = TempDir::new().expect("runtime placeholder repo");
    let cache = TempDir::new().expect("runtime placeholder cache");
    initialize_runtime_coverage_repo(&repo);
    let placeholder = repo.path().join("src/routes.mts");
    let body = "router.get('/not-parsed', handler);\n";
    write(&placeholder, body);
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "runtime parser placeholder"]);

    let first = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src", "--format", "json"],
    );
    std::thread::sleep(std::time::Duration::from_millis(5));
    write(&placeholder, body);
    let second = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src", "--format", "json"],
    );
    let first_id = horizon(&first["observations"], "routes")["count"]["certificate_id"]
        .as_str()
        .expect("first placeholder certificate");
    let second_id = horizon(&second["observations"], "routes")["count"]["certificate_id"]
        .as_str()
        .expect("second placeholder certificate");
    assert_eq!(
        first_id, second_id,
        "mtime-only churn on an unparsed placeholder cannot change map truth"
    );
}

#[cfg(unix)]
#[test]
fn tracked_route_symlink_is_not_followed_and_remains_a_typed_runtime_boundary() {
    use std::os::unix::fs::symlink;

    let workspace = TempDir::new().expect("runtime symlink workspace");
    let repo = workspace.path().join("repo");
    let cache = TempDir::new().expect("runtime symlink cache");
    fs::create_dir_all(repo.join("src")).expect("runtime source directory");
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.email", "a@example.com"]);
    git(&repo, &["config", "user.name", "a"]);
    write(
        &repo.join("package.json"),
        r#"{"name":"runtime-symlink-coverage","private":true}"#,
    );
    let external = workspace.path().join("external-routes.ts");
    write(
        &external,
        "router.get('/outside-repository', externalHandler);\n",
    );
    symlink(&external, repo.join("src/routes.ts")).expect("runtime source symlink");
    git(&repo, &["add", "package.json", "src/routes.ts"]);
    git(&repo, &["commit", "-qm", "tracked runtime route symlink"]);

    let mut certificate_ids = Vec::new();
    for iteration in 0..2 {
        let json = run_json(
            &repo,
            cache.path(),
            &["runtime", "src", "--format", "json"],
        );
        assert!(
            json["routes"]
                .as_array()
                .expect("runtime routes")
                .is_empty(),
            "external symlink contents must never become route facts: {json:#}"
        );
        let ledger = &json["observations"];
        let routes = horizon(ledger, "routes");
        assert_eq!(routes["count"]["observed"], 0, "{json:#}");
        assert_eq!(routes["count"]["closure"], "open", "{json:#}");
        assert!(
            routes["count"]["reasons"]
                .as_array()
                .expect("route coverage reasons")
                .iter()
                .any(|reason| reason == "unsupported_construct"),
            "the unread source placeholder needs a typed closure reason: {json:#}"
        );
        assert_unsupported_file(routes, "src/routes.ts", &json);
        assert!(
            routes["dynamic"]
                .as_array()
                .expect("dynamic route stops")
                .is_empty(),
            "unread external contents must not leak dynamic evidence: {json:#}"
        );
        assert_horizon_certificate_resolves(ledger, routes);

        let certificate_id = routes["count"]["certificate_id"]
            .as_str()
            .expect("runtime route certificate id");
        certificate_ids.push(certificate_id.to_string());
        let certificate = &ledger["certificates"][certificate_id];
        assert_eq!(certificate["eligible_files"], 1, "{json:#}");
        assert_eq!(certificate["visited_files"], 0, "{json:#}");
        assert_eq!(certificate["observed_facts"], 0, "{json:#}");
        if iteration == 0 {
            write(
                &external,
                "router.post('/changed-outside-repository', changedHandler);\n",
            );
            fs::remove_file(repo.join("src/routes.ts")).expect("replace runtime source symlink");
            symlink(&external, repo.join("src/routes.ts")).expect("recreate runtime source symlink");
        }
    }
    assert_eq!(
        certificate_ids[0], certificate_ids[1],
        "external symlink target state must not change the repository certificate"
    );
}
