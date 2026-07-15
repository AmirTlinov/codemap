// Responsibility: exact-file-cone-relationship-horizon-contract
const FILE_CONE_GROUPS: [(&str, &str); 5] = [
    ("outgoing", "outgoing"),
    ("incoming", "incoming"),
    ("verification", "proof"),
    ("contracts", "contracts"),
    ("boundary", "boundary"),
];

#[test]
fn exact_file_cone_relationships_are_bounded_in_readable_and_complete_in_json() {
    let repo = file_cone_fixture();
    let readable = run_markdown(
        repo.path(),
        TempDir::new().expect("file cone readable cache").path(),
        &["cone", "src/owner.ts", "--depth", "1", "--limit", "1"],
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("file cone json cache").path(),
        &[
            "cone",
            "src/owner.ts",
            "--depth",
            "1",
            "--limit",
            "1",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &json);
    assert_eq!(json["schema_version"], "17", "{json:#}");
    assert_eq!(json["outgoing"].as_array().expect("outgoing").len(), 1);
    assert_eq!(json["incoming"].as_array().expect("incoming").len(), 2);
    assert_eq!(json["proof"].as_array().expect("proof").len(), 1);
    for (group, field) in FILE_CONE_GROUPS {
        let item = horizon(&json["observations"], group);
        let observed = json[field].as_array().expect("relationship section").len();
        assert_eq!(item["count"]["observed"], observed, "{group}: {json:#}");
        assert_eq!(item["shown"], observed, "{group}: {json:#}");
        assert_horizon_certificate_resolves(&json["observations"], item);
        let row = readable
            .lines()
            .find(|line| line.starts_with(&format!("- {group}:")))
            .unwrap_or_else(|| panic!("missing {group} horizon: {readable}"));
        assert!(
            row.contains(&format!(
                "shown={} hidden={}",
                observed.min(1),
                observed.saturating_sub(1)
            )),
            "{group}: {readable}"
        );
    }
    assert!(
        json["hidden"].as_array().expect("hidden").iter().all(|hidden| {
            !hidden["reason"]
                .as_str()
                .unwrap_or_default()
                .contains("edges hidden by limit")
        }),
        "relationship horizons own exact-file cone truncation: {json:#}"
    );
    assert!(!readable.contains("edges hidden by limit"), "{readable}");
}

#[test]
fn supported_isolated_file_proves_zero_relationship_groups() {
    let repo = TempDir::new().expect("isolated file cone repo");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("src/isolated.ts"), "export const value = 1;\n");
    let json = run_json(
        repo.path(),
        TempDir::new().expect("isolated file cone cache").path(),
        &["cone", "src/isolated.ts", "--format", "json"],
    );
    for (group, field) in FILE_CONE_GROUPS {
        assert!(json[field].as_array().expect("section").is_empty(), "{json:#}");
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["observed"], 0, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], "closed", "{group}: {json:#}");
    }
}

#[test]
fn exact_file_cone_symbol_catalog_is_bounded_and_complete() {
    let repo = TempDir::new().expect("file cone symbol catalog repo");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("src/catalog.ts"),
        "export const a = 1;\nexport const b = 2;\nexport const c = 3;\nexport const d = 4;\n",
    );
    let readable = run_markdown(
        repo.path(),
        TempDir::new().expect("cone catalog readable cache").path(),
        &["cone", "src/catalog.ts", "--limit", "2"],
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("cone catalog json cache").path(),
        &["cone", "src/catalog.ts", "--limit", "2", "--format", "json"],
    );
    let symbols = horizon(&json["observations"], "symbols");
    assert_eq!(symbols["count"]["observed"], 4, "{json:#}");
    assert_eq!(symbols["count"]["closure"], "closed", "{json:#}");
    assert_eq!(symbols["shown"], 4, "{json:#}");
    assert_eq!(json["anchor"]["symbols"].as_array().unwrap().len(), 4);
    assert!(
        readable
            .lines()
            .any(|line| line.starts_with("- symbols:") && line.contains("shown=2 hidden=2")),
        "{readable}"
    );
    assert!(!readable.contains("symbols hidden by limit"), "{readable}");
    assert!(!readable.contains("nested symbols hidden by default"), "{readable}");
    let outputs = horizon(&json["observations"], "xray_outputs");
    assert_eq!(outputs["count"]["observed"], 4, "{json:#}");
    assert_eq!(outputs["count"]["closure"], "closed", "{json:#}");
    assert_eq!(outputs["shown"], 4, "{json:#}");
    assert_eq!(json["xray"]["outputs"].as_array().unwrap().len(), 4);
    assert!(
        readable.lines().any(|line| {
            line.starts_with("- xray_outputs:") && line.contains("shown=3 hidden=1")
        }),
        "{readable}"
    );
    assert!(!readable.contains("more Outputs entries hidden"), "{readable}");
}

#[test]
fn supported_empty_file_cone_proves_an_empty_symbol_catalog() {
    let repo = TempDir::new().expect("empty file cone catalog repo");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("src/empty.ts"), "");
    let json = run_json(
        repo.path(),
        TempDir::new().expect("empty cone catalog cache").path(),
        &["cone", "src/empty.ts", "--format", "json"],
    );
    let symbols = horizon(&json["observations"], "symbols");
    assert_eq!(symbols["count"]["observed"], 0, "{json:#}");
    assert_eq!(symbols["count"]["closure"], "closed", "{json:#}");
    assert!(json["anchor"]["symbols"].as_array().unwrap().is_empty());
    let outputs = horizon(&json["observations"], "xray_outputs");
    assert_eq!(outputs["count"]["observed"], 0, "{json:#}");
    assert_eq!(outputs["count"]["closure"], "closed", "{json:#}");
    assert!(json["xray"]["outputs"].as_array().unwrap().is_empty());
}

#[test]
fn unsupported_file_cone_keeps_symbol_catalog_unavailable() {
    let repo = TempDir::new().expect("unsupported file cone catalog repo");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("notes.txt"), "plain text\n");
    let json = run_json(
        repo.path(),
        TempDir::new().expect("unsupported cone catalog cache").path(),
        &["cone", "notes.txt", "--format", "json"],
    );
    let symbols = horizon(&json["observations"], "symbols");
    assert_eq!(symbols["count"]["closure"], "unavailable", "{json:#}");
    assert_eq!(
        symbols["count"]["reasons"],
        serde_json::json!(["unsupported_language"]),
        "{json:#}"
    );
    let outputs = horizon(&json["observations"], "xray_outputs");
    assert_eq!(outputs["count"]["closure"], "unavailable", "{json:#}");
}

#[test]
fn dynamic_unresolved_and_reexport_flows_keep_file_cone_groups_open() {
    let repo = TempDir::new().expect("open file cone repo");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("src/owner.ts"),
        "export { value } from './dep';\nconst path = './dep';\nexport const load = () => import(path);\nimport { absent } from './missing';\nvoid absent;\n",
    );
    write(&repo.path().join("src/dep.ts"), "export const value = 1;\n");
    write(
        &repo.path().join("src/dynamic-consumer.ts"),
        "const path = './owner';\nexport const load = () => import(path);\n",
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("open file cone cache").path(),
        &["cone", "src/owner.ts", "--depth", "1", "--format", "json"],
    );
    for group in ["outgoing", "incoming", "verification", "contracts"] {
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
    }
    let outgoing = horizon(&json["observations"], "outgoing");
    let reasons = outgoing["count"]["reasons"].as_array().expect("reasons");
    assert!(reasons.iter().any(|reason| reason == "dynamic_import_flow"));
    assert!(reasons.iter().any(|reason| reason == "incomplete_traversal"));
    assert!(reasons.iter().any(|reason| reason == "reexport_flow"));
    let xray_unknowns = horizon(&json["observations"], "xray_unknowns");
    assert_eq!(xray_unknowns["count"]["closure"], "open", "{json:#}");
    let xray_reasons = xray_unknowns["count"]["reasons"].as_array().unwrap();
    assert!(xray_reasons.iter().any(|reason| reason == "dynamic_import_flow"));
    assert!(xray_reasons.iter().any(|reason| reason == "reexport_flow"));
}

#[test]
fn unavailable_body_keeps_every_file_cone_group_open() {
    let repo = TempDir::new().expect("unavailable file cone repo");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("src/huge.ts"), &"x".repeat(901_000));
    let json = run_json(
        repo.path(),
        TempDir::new().expect("unavailable file cone cache").path(),
        &["cone", "src/huge.ts", "--format", "json"],
    );
    for (group, _) in FILE_CONE_GROUPS {
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
        assert!(
            item["unsupported"]
                .as_array()
                .expect("unsupported")
                .iter()
                .any(|gap| gap["file"] == "src/huge.ts"),
            "{group}: {json:#}"
        );
    }
    let symbols = horizon(&json["observations"], "symbols");
    assert_eq!(symbols["count"]["closure"], "unavailable", "{json:#}");
    assert_eq!(symbols["count"]["observed"], 0, "{json:#}");
    let outputs = horizon(&json["observations"], "xray_outputs");
    assert_eq!(outputs["count"]["closure"], "unavailable", "{json:#}");
    assert_eq!(outputs["count"]["observed"], 0, "{json:#}");
}

#[test]
fn malformed_manifest_keeps_every_file_cone_candidate_basis_open() {
    let repo = TempDir::new().expect("malformed manifest cone repo");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("package.json"), "{ malformed");
    let json = run_json(
        repo.path(),
        TempDir::new().expect("malformed manifest cone cache").path(),
        &["cone", "package.json", "--format", "json"],
    );
    for (group, _) in FILE_CONE_GROUPS {
        let item = horizon(&json["observations"], group);
        assert_eq!(item["count"]["closure"], "open", "{group}: {json:#}");
        assert!(
            item["unsupported"]
                .as_array()
                .expect("unsupported")
                .iter()
                .any(|gap| gap["file"] == "package.json"),
            "{group}: {json:#}"
        );
    }
}

#[test]
fn exact_file_cone_cache_preserves_the_thirteen_group_ledger() {
    let repo = file_cone_fixture();
    let cache = TempDir::new().expect("file cone warm cache");
    let args = [
        "cone",
        "src/owner.ts",
        "--depth",
        "1",
        "--limit",
        "1",
        "--format",
        "json",
    ];
    let cold = run_json(repo.path(), cache.path(), &args);
    let warm = run_json(repo.path(), cache.path(), &args);
    assert_eq!(warm, cold, "warm exact-file cone must preserve the ledger");
    let artifact: Value = serde_json::from_str(
        &fs::read_to_string(lens_artifact_path(cache.path(), "cone-current.json"))
            .expect("cached exact-file cone artifact"),
    )
    .expect("exact-file cone artifact json");
    assert_eq!(
        artifact["report"]["observations"]["horizons"]
            .as_array()
            .expect("cached horizons")
            .len(),
        13,
        "{artifact:#}"
    );
}

fn file_cone_fixture() -> TempDir {
    let repo = TempDir::new().expect("file cone fixture");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("src/owner.ts"),
        "import { dep } from './dep';\nexport const owner = dep;\n",
    );
    write(&repo.path().join("src/dep.ts"), "export const dep = 1;\n");
    for name in ["consumer-a", "consumer-b"] {
        write(
            &repo.path().join(format!("src/{name}.ts")),
            "import { owner } from './owner';\nvoid owner;\n",
        );
    }
    write(
        &repo.path().join("tests/owner.test.ts"),
        "import { owner } from '../src/owner';\ntest('owner', () => expect(owner).toBe(1));\n",
    );
    repo
}
