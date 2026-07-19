#[test]
fn runtime_route_horizon_owns_mass_truncation_and_route_gaps() {
    let repo = runtime_route_coverage_fixture();
    let readable_cache = TempDir::new().expect("readable runtime cache");
    let json_cache = TempDir::new().expect("json runtime cache");

    let markdown = run_markdown(
        repo.path(),
        readable_cache.path(),
        &["runtime", "src", "--limit", "12"],
    );
    assert!(
        markdown.contains("- routes: counted-at-least(227,")
            && markdown.contains("shown=12 hidden=215"),
        "bounded runtime output must identify its route sample as a lower bound: {markdown}"
    );
    assert!(
        markdown.contains("dynamic=3") && markdown.contains("unsupported_files=2"),
        "readable route horizon must name both unresolved dynamic registrations and unsupported files: {markdown}"
    );
    assert!(
        markdown.lines().count() <= 90,
        "the 227-route fixture must remain a bounded daily map: {markdown}"
    );

    let json = run_json(
        repo.path(),
        json_cache.path(),
        &["runtime", "src", "--limit", "12", "--format", "json"],
    );
    assert_eq!(
        json["routes"].as_array().expect("runtime routes").len(),
        227,
        "machine output must imply full visibility despite the readable limit: {json:#}"
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["observed"], 227, "{json:#}");
    assert_eq!(routes["count"]["closure"], "open", "{json:#}");
    assert_eq!(routes["shown"], 227, "{json:#}");
    assert_eq!(routes["hidden"], 0, "{json:#}");
    assert!(routes["expand"].is_null(), "{json:#}");
    assert_eq!(
        routes["dynamic"]
            .as_array()
            .expect("dynamic route stops")
            .len(),
        3,
        "every dynamic registration must stay attached to the route horizon: {json:#}"
    );
    let unsupported_files = routes["unsupported"]
        .as_array()
        .expect("unsupported routes")
        .iter()
        .filter_map(|item| item["file"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        unsupported_files,
        std::collections::BTreeSet::from([
            "src/nest-admin.controller.ts",
            "src/nest-user.controller.ts",
        ]),
        "unsupported syntax must be counted by exact file, not hidden in a generic unknown: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);

    let certificate_id = routes["count"]["certificate_id"]
        .as_str()
        .expect("route certificate id");
    let certificate = &ledger["certificates"][certificate_id];
    assert_eq!(certificate["observed_facts"], 227, "{json:#}");
    assert_eq!(certificate["dynamic_stops"], routes["dynamic"], "{json:#}");
    assert_eq!(
        certificate["unsupported"], routes["unsupported"],
        "{json:#}"
    );
}

#[test]
fn supported_typescript_file_without_routes_is_certificate_backed_proven_zero() {
    let repo = TempDir::new().expect("runtime proven-zero repo");
    let cache = TempDir::new().expect("runtime proven-zero cache");
    initialize_runtime_coverage_repo(&repo);
    write(
        &repo.path().join("src/no-routes.ts"),
        "export function healthyModule(): boolean { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "supported no-route fixture"],
    );

    let json = run_json(
        repo.path(),
        cache.path(),
        &["runtime", "src/no-routes.ts", "--format", "json"],
    );
    assert!(
        json["routes"]
            .as_array()
            .expect("runtime routes")
            .is_empty(),
        "a supported route-free file must not invent runtime routes: {json:#}"
    );
    let ledger = &json["observations"];
    let routes = horizon(ledger, "routes");
    assert_eq!(routes["count"]["observed"], 0, "{json:#}");
    assert_eq!(routes["count"]["closure"], "closed", "{json:#}");
    assert_eq!(
        routes["count"]["reasons"],
        serde_json::json!([]),
        "a fully supported exact file should prove absence without a gap reason: {json:#}"
    );
    assert_eq!(routes["shown"], 0, "{json:#}");
    assert_eq!(routes["hidden"], 0, "{json:#}");
    assert!(routes["expand"].is_null(), "{json:#}");
    assert!(
        routes["dynamic"]
            .as_array()
            .expect("dynamic stops")
            .is_empty(),
        "{json:#}"
    );
    assert!(
        routes["unsupported"]
            .as_array()
            .expect("unsupported routes")
            .is_empty(),
        "{json:#}"
    );
    assert_horizon_certificate_resolves(ledger, routes);

    let certificate_id = routes["count"]["certificate_id"]
        .as_str()
        .expect("route certificate id");
    let certificate = &ledger["certificates"][certificate_id];
    assert_eq!(certificate["eligible_files"], 1, "{json:#}");
    assert_eq!(certificate["visited_files"], 1, "{json:#}");
    assert_eq!(certificate["observed_facts"], 0, "{json:#}");

    let proof = horizon(ledger, "proof");
    assert_eq!(proof["count"]["observed"], 0, "{json:#}");
    assert_eq!(proof["count"]["closure"], "closed", "{json:#}");
    assert_eq!(proof["count"]["reasons"], serde_json::json!([]));
    assert_horizon_certificate_resolves(ledger, proof);
}

#[test]
fn runtime_mass_keeps_each_fact_group_visible_and_path_order_invariant() {
    let first = runtime_diversified_coverage_fixture(false);
    let reordered = runtime_diversified_coverage_fixture(true);
    let first_cache = TempDir::new().expect("runtime mass cache");
    let reordered_cache = TempDir::new().expect("reordered runtime mass cache");
    let readable = run_markdown(
        first.path(),
        first_cache.path(),
        &["runtime", "src", "--limit", "3"],
    );
    for (group, observed) in [("routes", 227), ("env", 120), ("workers", 120)] {
        let line = readable
            .lines()
            .find(|line| line.starts_with(&format!("- {group}:")))
            .unwrap_or_else(|| panic!("missing {group} representation: {readable}"));
        assert!(
            line.contains(&format!("shown=3 hidden={}", observed - 3)),
            "{group} lost exact bounded remainder: {readable}"
        );
        assert!(
            readable.contains(&format!("codemap runtime src --all --limit {observed}")),
            "{group} lost its concrete expansion: {readable}"
        );
    }

    let first_json = run_json(
        first.path(),
        first_cache.path(),
        &["runtime", "src", "--limit", "3", "--format", "json"],
    );
    let reordered_json = run_json(
        reordered.path(),
        reordered_cache.path(),
        &["runtime", "src", "--limit", "3", "--format", "json"],
    );
    for group in ["routes", "env", "workers"] {
        assert_eq!(
            first_json[group], reordered_json[group],
            "path insertion order changed the `{group}` fact group"
        );
        assert_eq!(
            horizon(&first_json["observations"], group)["count"]["observed"],
            horizon(&reordered_json["observations"], group)["count"]["observed"],
            "path insertion order changed the `{group}` remainder scale"
        );
    }
}

fn runtime_route_coverage_fixture() -> TempDir {
    let repo = TempDir::new().expect("runtime coverage repo");
    initialize_runtime_coverage_repo(&repo);

    let mut javascript_routes = String::new();
    for index in 0..114 {
        javascript_routes.push_str(&format!(
            "router.get('/mass/{index:03}', handler{index:03});\n"
        ));
    }
    write(&repo.path().join("src/routes-a.js"), &javascript_routes);

    let mut typescript_routes = String::new();
    for index in 114..227 {
        typescript_routes.push_str(&format!(
            "router.post('/mass/{index:03}', handler{index:03});\n"
        ));
    }
    write(&repo.path().join("src/routes-b.ts"), &typescript_routes);
    write(
        &repo.path().join("src/dynamic-routes.ts"),
        "const prefix = '/dynamic';\nconst routePath = resolveRoutePath();\nconst method = resolveRouteMethod();\nrouter.get(prefix + '/one', dynamicOne);\nrouter.post(routePath, dynamicTwo);\nrouter[method]('/dynamic/three', dynamicThree);\n",
    );
    write(
        &repo.path().join("src/nest-admin.controller.ts"),
        "import { Get } from '@nestjs/common';\n\nexport class AdminController {\n  @Get('/admin')\n  list() { return []; }\n}\n",
    );
    write(
        &repo.path().join("src/nest-user.controller.ts"),
        "import { Post } from '@nestjs/common';\n\nexport class UserController {\n  @Post('/users')\n  create() { return true; }\n}\n",
    );
    write(
        &repo.path().join("src/no-routes.ts"),
        "export function healthyModule(): boolean { return true; }\n",
    );
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &["commit", "-qm", "runtime coverage mass fixture"],
    );
    repo
}

fn runtime_diversified_coverage_fixture(reverse: bool) -> TempDir {
    let repo = runtime_route_coverage_fixture();
    let indexes: Box<dyn Iterator<Item = usize>> = if reverse {
        Box::new((0..120).rev())
    } else {
        Box::new(0..120)
    };
    for index in indexes {
        write(
            &repo.path().join(format!("src/jobs/job-{index:03}.worker.ts")),
            &format!(
                "export const job{index:03} = process.env.RUNTIME_JOB_{index:03};\n"
            ),
        );
    }
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "diversified runtime mass"]);
    repo
}

fn initialize_runtime_coverage_repo(repo: &TempDir) {
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"runtime-coverage-fixture","private":true}"#,
    );
}
