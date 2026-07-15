// Responsibility: root-inventory-horizon-pilot-matrix
#[test]
fn root_inventory_horizons_certify_every_group() {
    let repo = root_inventory_horizon_fixture();
    let cache = TempDir::new().expect("root inventory horizon cache");

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    assert_eq!(json["schema_version"], "11", "{json:#}");
    let ledger = &json["observations"];
    assert_eq!(
        ledger["horizons"].as_array().expect("horizons").len(),
        4,
        "the root ls ledger must carry exactly one horizon per inventory group: {json:#}"
    );

    let surfaces = horizon(ledger, "directory_surfaces");
    assert_eq!(
        surfaces["count"]["observed"],
        json["directory"].as_array().expect("directory").len() as u64,
        "the surface horizon must count the projection's own catalog: {json:#}"
    );
    assert_eq!(surfaces["count"]["closure"], "closed", "{json:#}");
    assert_eq!(surfaces["hidden"], 0, "{json:#}");
    let surfaces_certificate =
        &ledger["certificates"][surfaces["count"]["certificate_id"].as_str().expect("id")];
    assert_eq!(
        surfaces_certificate["eligible_files"], surfaces_certificate["visited_files"],
        "the full-index surface catalog closes only with exact entry accounting: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, surfaces);

    let packages = horizon(ledger, "packages");
    assert_eq!(packages["count"]["observed"], 3, "{json:#}");
    assert_eq!(packages["count"]["closure"], "closed", "{json:#}");
    assert_eq!(packages["shown"], 3, "{json:#}");
    assert_eq!(
        packages["hidden"], 0,
        "full JSON must serialize every observed package fact: {json:#}"
    );
    assert!(
        packages["expand"].is_null(),
        "a full machine projection must not advertise a hidden package remainder: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, packages);

    let scripts = horizon(ledger, "scripts");
    assert_eq!(scripts["count"]["closure"], "open", "{json:#}");
    assert!(
        scripts["count"]["reasons"]
            .as_array()
            .expect("script reasons")
            .iter()
            .any(|reason| reason == "incomplete_traversal"),
        "a root-only script catalog cannot close over nested manifests: {json:#}"
    );
    let scripts_certificate =
        &ledger["certificates"][scripts["count"]["certificate_id"].as_str().expect("id")];
    assert!(
        scripts_certificate["excluded_files_by_reason"]["incomplete_traversal"]
            .as_array()
            .expect("unvisited script manifests")
            .iter()
            .any(|file| file == "packages/app/package.json"),
        "the unvisited nested manifest must be an exact exclusion: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, scripts);

    let tests = horizon(ledger, "test_surfaces");
    assert_eq!(tests["count"]["observed"], 1, "{json:#}");
    assert_eq!(tests["count"]["closure"], "closed", "{json:#}");
    assert_eq!(tests["count"]["reasons"], serde_json::json!([]), "{json:#}");
    let tests_certificate =
        &ledger["certificates"][tests["count"]["certificate_id"].as_str().expect("id")];
    assert_eq!(
        tests_certificate["eligible_files"], tests_certificate["visited_files"],
        "test surfaces close only with exact current-level accounting: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, tests);

    for legacy in [
        "directory surfaces hidden by limit",
        "support packages hidden below support scopes",
    ] {
        assert!(
            json["hidden"]
                .as_array()
                .expect("hidden")
                .iter()
                .all(|group| group["reason"] != legacy),
            "legacy detached hidden group `{legacy}` must not duplicate the horizon: {json:#}"
        );
    }
}

#[test]
fn bounded_readable_root_inventory_shares_the_machine_certificates() {
    let repo = root_inventory_horizon_fixture();
    let readable_cache = TempDir::new().expect("readable root inventory cache");
    let json_cache = TempDir::new().expect("json root inventory cache");

    let markdown = run_markdown(repo.path(), readable_cache.path(), &["ls", "."]);
    assert!(
        markdown.contains("- packages: counted(3); shown=2 hidden=1")
            && markdown.contains("codemap ls . --all"),
        "bounded readable packages must expose their horizon and expansion: {markdown}"
    );
    assert!(
        markdown.contains("- test_surfaces: counted(1); shown=1 hidden=0")
            && markdown.contains("- scripts: counted-at-least("),
        "every root inventory group must print its own certified visibility row: {markdown}"
    );

    let json = run_json(
        repo.path(),
        json_cache.path(),
        &["ls", ".", "--format", "json"],
    );
    for horizon in json["observations"]["horizons"]
        .as_array()
        .expect("root inventory horizons")
    {
        assert_eq!(
            horizon["shown"], horizon["count"]["observed"],
            "full JSON must serialize every fact in {}: {json:#}",
            horizon["group"]
        );
        assert_eq!(horizon["hidden"], 0, "{json:#}");
    }
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
        4,
        "readable root ls output must show all four group certificates: {markdown}"
    );
    for horizon in json["observations"]["horizons"]
        .as_array()
        .expect("root inventory horizons")
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
fn warm_ls_cache_preserves_root_inventory_horizons() {
    let repo = root_inventory_horizon_fixture();
    let cache = TempDir::new().expect("warm root inventory cache");

    let cold = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    let artifact = ls_lens_artifact_json(cache.path());
    assert_eq!(
        artifact["report"]["observations"]["horizons"]
            .as_array()
            .expect("cached horizons")
            .len(),
        4,
        "the trusted-local ls artifact must persist all four group horizons: {artifact:#}"
    );

    let warm = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_eq!(
        warm["observations"], cold["observations"],
        "the persisted group ledger must survive a warm cache read unchanged"
    );
    assert_eq!(warm["directory"], cold["directory"]);
    assert_eq!(warm["hidden"], cold["hidden"]);

    let readable_first = run_markdown(repo.path(), cache.path(), &["ls", "."]);
    let readable_second = run_markdown(repo.path(), cache.path(), &["ls", "."]);
    assert!(
        readable_first.contains("## Visibility"),
        "warm readable root ls must keep its visibility section: {readable_first}"
    );
    assert_lens_markdown_eq(
        &readable_first,
        &readable_second,
        "the warm root ls must preserve every cold group horizon",
    );
}

#[test]
fn cold_root_inventory_fast_path_keeps_horizons_typed_open() {
    let repo = TempDir::new().expect("cold root inventory horizon repo");
    let cache = TempDir::new().expect("cold root inventory horizon cache");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "cold-root-horizon-fixture",
  "private": true,
  "scripts": {
    "test": "vitest run"
  }
}
"#,
    );
    write(
        &repo.path().join("fixtures/demo/package.json"),
        r#"{"name":"@fixture/demo","private":true}"#,
    );
    write(&repo.path().join("tests/replay.test.ts"), "export {};\n");
    write(&repo.path().join("config/.env.example"), "API_URL=\n");
    for index in 0..820 {
        write(
            &repo.path().join(format!("src/bulk/file_{index:03}.ts")),
            &format!("export const value{index} = {index};\n"),
        );
    }

    let first = run_markdown(repo.path(), cache.path(), &["ls", "."]);
    let second = run_markdown(repo.path(), cache.path(), &["ls", "."]);
    assert!(
        first.contains("full-index source edges hidden by bounded root inventory")
            && first.contains("## Visibility"),
        "the cold bounded inventory must expose certified visibility: {first}"
    );
    assert_lens_markdown_eq(
        &first,
        &second,
        "the cold root inventory fast path must return an identical projection",
    );

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    let ledger = &json["observations"];
    assert_eq!(
        ledger["horizons"].as_array().expect("horizons").len(),
        4,
        "{json:#}"
    );
    for complete in ledger["horizons"].as_array().expect("horizons") {
        let certificate = complete["count"]["certificate_id"]
            .as_str()
            .expect("certificate")
            .strip_prefix("coverage-v1:")
            .expect("certificate prefix");
        assert!(
            first.contains(&format!("cert=`v1:{}`", &certificate[..12])),
            "cold readable and full JSON must share the {} certificate: {first}\n{json:#}",
            complete["group"]
        );
        assert_eq!(complete["shown"], complete["count"]["observed"], "{json:#}");
        assert_eq!(complete["hidden"], 0, "{json:#}");
    }
    assert!(
        json["directory"]
            .as_array()
            .expect("directory")
            .iter()
            .any(|surface| surface["kind"] == "env_config"),
        "expansion must preserve the bounded env surface identity: {json:#}"
    );
    assert!(
        json["directory"]
            .as_array()
            .expect("directory")
            .iter()
            .all(|surface| surface["kind"] != "recursive:env_config"),
        "expansion may not rename an already observed surface: {json:#}"
    );
    for group in ["directory_surfaces", "test_surfaces"] {
        let bounded = horizon(ledger, group);
        assert_eq!(
            bounded["count"]["closure"], "open",
            "{group}: the bounded inventory grammar cannot prove completeness: {json:#}"
        );
        assert!(
            bounded["count"]["reasons"]
                .as_array()
                .expect("bounded reasons")
                .iter()
                .any(|reason| reason == "unsupported_construct"),
            "{group}: the extractor gap must stay typed: {json:#}"
        );
        assert_horizon_certificate_resolves(ledger, bounded);
    }
    let tests = horizon(ledger, "test_surfaces");
    assert_eq!(
        tests["count"]["observed"], 0,
        "the bounded inventory cannot see test roles, so zero stays open: {json:#}"
    );
    let packages = horizon(ledger, "packages");
    assert_eq!(packages["count"]["closure"], "open", "{json:#}");
    let packages_certificate =
        &ledger["certificates"][packages["count"]["certificate_id"].as_str().expect("id")];
    assert_eq!(
        packages_certificate["eligible_files"], packages_certificate["visited_files"],
        "the bounded path owner sees the complete finite manifest inventory but stays typed open: {json:#}"
    );
    assert_horizon_certificate_resolves(ledger, packages);
    assert_eq!(
        horizon(ledger, "scripts")["count"]["closure"],
        "open",
        "{json:#}"
    );
}

fn root_inventory_horizon_fixture() -> TempDir {
    let repo = TempDir::new().expect("root inventory horizon repo");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "root-inventory-horizon-fixture",
  "private": true,
  "workspaces": ["packages/*"],
  "scripts": {
    "test": "vitest run"
  }
}
"#,
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{"name":"@fixture/app","private":true}"#,
    );
    write(
        &repo.path().join("packages/app/src/index.ts"),
        "export const app = true;\n",
    );
    write(
        &repo.path().join("fixtures/demo/package.json"),
        r#"{"name":"@fixture/demo","private":true}"#,
    );
    write(&repo.path().join("tests/replay.test.ts"), "export {};\n");
    write(&repo.path().join("src/main.ts"), "export const main = 1;\n");
    write(&repo.path().join("README.md"), "# Root Inventory Horizons\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "root inventory horizon fixture"]);
    repo
}

fn ls_lens_artifact_json(cache: &Path) -> Value {
    let path = lens_artifact_path(cache, "ls-current.json");
    serde_json::from_str(&fs::read_to_string(path).expect("ls lens artifact"))
        .expect("ls lens artifact json")
}
