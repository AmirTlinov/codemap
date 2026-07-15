// Responsibility: runtime-route-coverage-boundaries
#[test]
fn dynamic_rust_axum_path_keeps_the_runtime_route_horizon_open() {
    let repo = TempDir::new().expect("dynamic axum repo");
    let cache = TempDir::new().expect("dynamic axum cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/main.rs"),
        "use axum::{routing::get, Router};\nfn app(dynamic_path: &str) { let _ = Router::new().route(dynamic_path, get(handler)); }\nasync fn handler() {}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "dynamic axum route"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/main.rs", "--format", "json"],
    );
    assert!(
        json["routes"]
            .as_array()
            .expect("runtime routes")
            .is_empty(),
        "a dynamic axum path must not become a static route fact: {json:#}"
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["observed"], 0, "{json:#}");
    assert_eq!(routes["count"]["closure"], "open", "{json:#}");
    assert_eq!(
        routes["dynamic"].as_array().expect("dynamic routes").len(),
        1,
        "the dropped axum registration needs an exact dynamic stop: {json:#}"
    );
    assert_eq!(
        routes["dynamic"][0]["kind"],
        "dynamic_runtime_registration",
        "{json:#}"
    );
    assert_eq!(
        routes["dynamic"][0]["location"]["path"],
        "src/main.rs",
        "{json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);
}

#[test]
fn same_line_dynamic_route_registrations_remain_distinct_coverage_stops() {
    let repo = TempDir::new().expect("same-line dynamic route repo");
    let cache = TempDir::new().expect("same-line dynamic route cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/dynamic.ts"),
        "router.get(firstPath, firstHandler); router.post(secondPath, secondHandler);\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "same-line dynamic routes"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/dynamic.ts", "--format", "json"],
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["closure"], "open", "{json:#}");
    assert_eq!(
        routes["dynamic"].as_array().expect("dynamic routes").len(),
        2,
        "each registration on a minified line needs its own coverage stop: {json:#}"
    );
    assert_eq!(
        json["unknowns"]
            .as_array()
            .expect("runtime unknowns")
            .iter()
            .filter(|unknown| unknown["kind"] == "route_dynamic_path")
            .count(),
        2,
        "runtime unknowns must preserve both same-line registrations: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);
}

#[test]
fn multiline_route_registration_keeps_the_horizon_open_when_not_statically_assembled() {
    let repo = TempDir::new().expect("multiline route repo");
    let cache = TempDir::new().expect("multiline route cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/routes.ts"),
        "router.get(\n  dynamicPath,\n  handler\n);\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "multiline dynamic route"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/routes.ts", "--format", "json"],
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["observed"], 0, "{json:#}");
    assert_eq!(routes["count"]["closure"], "open", "{json:#}");
    assert!(
        !routes["unsupported"]
            .as_array()
            .expect("unsupported routes")
            .is_empty(),
        "an unassembled multiline registration needs a typed unsupported boundary: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);
}

#[test]
fn go_handle_func_dynamic_and_multiline_paths_never_prove_route_absence() {
    for (name, body) in [
        (
            "dynamic",
            "package api\nimport \"net/http\"\nfunc routes(dynamicPath string) { http.HandleFunc(dynamicPath, handler) }\n",
        ),
        (
            "multiline",
            "package api\nimport \"net/http\"\nfunc routes(dynamicPath string) {\n  http.HandleFunc(\n    dynamicPath,\n    handler,\n  )\n}\n",
        ),
    ] {
        let repo = TempDir::new().expect("Go route gap repo");
        let cache = TempDir::new().expect("Go route gap cache");
        initialize_runtime_coverage_repo(&repo);
        write(&repo.path().join("src/routes.go"), body);
        git(repo.path(), &["add", "."]);
        git(repo.path(), &["commit", "-qm", "Go route gap"]);

        let json = run_json(
            repo.path(),
            cache.path(),
            &["runtime", "src/routes.go", "--format", "json"],
        );
        let ledger = &json["observations"];
        let routes = horizon(ledger, "routes");
        assert_eq!(routes["count"]["observed"], 0, "{name}: {json:#}");
        assert_eq!(routes["count"]["closure"], "open", "{name}: {json:#}");
        assert!(
            !routes["dynamic"]
                .as_array()
                .expect("dynamic routes")
                .is_empty()
                || !routes["unsupported"]
                    .as_array()
                    .expect("unsupported routes")
                    .is_empty(),
            "{name}: dropped HandleFunc registration needs an explicit gap: {json:#}"
        );
        assert_horizon_certificate_resolves(ledger, routes);
    }
}

#[test]
fn computed_javascript_method_with_dynamic_path_keeps_route_coverage_open() {
    let repo = TempDir::new().expect("computed JS route repo");
    let cache = TempDir::new().expect("computed JS route cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/routes.ts"),
        "router[method](dynamicPath, handler);\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "computed JS route"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/routes.ts", "--format", "json"],
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["observed"], 0, "{json:#}");
    assert_eq!(routes["count"]["closure"], "open", "{json:#}");
    assert_eq!(
        routes["dynamic"].as_array().expect("dynamic routes").len(),
        1,
        "the computed registration needs one dynamic stop: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);
}

#[test]
fn computed_method_on_a_static_javascript_route_chain_keeps_coverage_open() {
    let repo = TempDir::new().expect("computed chained JS route repo");
    let cache = TempDir::new().expect("computed chained JS route cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/routes.ts"),
        "router.route('/users')[method](handler);\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "computed chained JS route"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/routes.ts", "--format", "json"],
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["closure"], "open", "{json:#}");
    assert_eq!(
        routes["dynamic"].as_array().expect("dynamic routes").len(),
        1,
        "the computed chained method needs one dynamic stop: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);
}

#[test]
fn same_line_go_handle_func_registrations_are_all_observed() {
    let repo = TempDir::new().expect("same-line Go route repo");
    let cache = TempDir::new().expect("same-line Go route cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/routes.go"),
        "package api\nimport \"net/http\"\nfunc routes() { http.HandleFunc(\"/a\", a); http.HandleFunc(\"/b\", b) }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "same-line Go routes"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/routes.go", "--format", "json"],
    );
    let paths = json["routes"]
        .as_array()
        .expect("Go runtime routes")
        .iter()
        .filter_map(|route| route["path"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(paths, std::collections::BTreeSet::from(["/a", "/b"]));
    let routes = horizon(&json["observations"], "routes");
    assert_eq!(routes["count"]["observed"], 2, "{json:#}");
    assert_eq!(routes["count"]["closure"], "closed", "{json:#}");
    assert_horizon_certificate_resolves(&json["observations"], routes);
}

#[test]
fn later_same_line_go_dynamic_method_keeps_the_partial_route_count_open() {
    let repo = TempDir::new().expect("same-line Go dynamic method repo");
    let cache = TempDir::new().expect("same-line Go dynamic method cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/routes.go"),
        "package api\nimport \"net/http\"\nfunc routes(method string) { http.HandleFunc(\"/a\", a); router.HandleFunc(\"/b\", b).Methods(method) }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "same-line Go dynamic method"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/routes.go", "--format", "json"],
    );
    let routes = horizon(&json["observations"], "routes");
    assert_eq!(routes["count"]["observed"], 1, "{json:#}");
    assert_eq!(routes["count"]["closure"], "open", "{json:#}");
    assert_eq!(
        routes["dynamic"].as_array().expect("dynamic routes").len(),
        1,
        "the later unresolved Methods call needs a dynamic stop: {json:#}"
    );
    assert_horizon_certificate_resolves(&json["observations"], routes);
}

#[test]
fn dynamic_route_object_and_prefixless_router_mount_are_explicit_gaps() {
    let repo = TempDir::new().expect("dynamic JS route forms repo");
    let cache = TempDir::new().expect("dynamic JS route forms cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/routes.ts"),
        "fastify.route({ method: 'GET', url: '/a', handler: a }); fastify.route(dynamicRouteOptions);\napp.use(cors()); app.use('/api', apiRouter);\napp.use(importedRouter);\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "dynamic JS route forms"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/routes.ts", "--format", "json"],
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["observed"], 1, "{json:#}");
    assert_eq!(routes["count"]["closure"], "open", "{json:#}");
    let kinds = json["unknowns"]
        .as_array()
        .expect("runtime unknowns")
        .iter()
        .filter_map(|unknown| unknown["kind"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(kinds.contains("route_object_dynamic"), "{json:#}");
    assert!(kinds.contains("route_mount_prefix"), "{json:#}");
    assert!(kinds.contains("route_mount_target"), "{json:#}");
    assert_eq!(
        routes["dynamic"].as_array().expect("dynamic routes").len(),
        3,
        "all unresolved route-producing forms need coverage stops: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);
}
