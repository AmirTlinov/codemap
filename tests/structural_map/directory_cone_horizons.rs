// Responsibility: directory-cone-relationship-horizon-contract
const DIRECTORY_CONE_GROUPS: [(&str, &str); 5] = [
    ("outgoing", "outgoing"),
    ("incoming", "incoming"),
    ("verification", "proof"),
    ("contracts", "contracts"),
    ("boundary", "boundary"),
];

#[test]
fn directory_cone_relationships_are_bounded_in_readable_and_complete_in_json() {
    let repo = nested_directory_fixture();
    let readable = run_markdown(
        repo.path(),
        TempDir::new().expect("directory cone readable cache").path(),
        &["cone", NESTED_DIRECTORY_ANCHOR, "--depth", "1", "--limit", "2"],
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("directory cone json cache").path(),
        &[
            "cone",
            NESTED_DIRECTORY_ANCHOR,
            "--depth",
            "1",
            "--limit",
            "2",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/cone.schema.json", &json);
    for (group, field) in DIRECTORY_CONE_GROUPS {
        let group_horizon = horizon(&json["observations"], group);
        let observed = json[field].as_array().expect("relationship section").len();
        assert_eq!(group_horizon["count"]["observed"], observed, "{group}: {json:#}");
        assert_eq!(group_horizon["shown"], observed, "{group}: {json:#}");
        assert_horizon_certificate_resolves(&json["observations"], group_horizon);
        let readable_row = readable
            .lines()
            .find(|line| line.starts_with(&format!("- {group}:")))
            .unwrap_or_else(|| panic!("missing {group} horizon: {readable}"));
        assert!(
            readable_row.contains(&format!("shown={} hidden={}", observed.min(2), observed.saturating_sub(2))),
            "{group}: {readable}"
        );
    }
    assert!(
        json["hidden"].as_array().expect("hidden").iter().all(|hidden| {
            !hidden["reason"]
                .as_str()
                .unwrap_or_default()
                .starts_with("directory ")
        }),
        "relationship horizons own directory cone truncation: {json:#}"
    );
    assert!(!readable.contains("directory outgoing edges hidden by limit"));
}

#[test]
fn supported_empty_directory_cone_proves_zero_relationship_groups() {
    let repo = TempDir::new().expect("empty directory cone repo");
    git(repo.path(), &["init", "-q"]);
    write(&repo.path().join("src/empty/index.ts"), "export const value = 1;\n");
    let json = run_json(
        repo.path(),
        TempDir::new().expect("empty directory cone cache").path(),
        &["cone", "src/empty", "--format", "json"],
    );
    for (group, field) in DIRECTORY_CONE_GROUPS {
        assert!(json[field].as_array().expect("section").is_empty(), "{json:#}");
        let group_horizon = horizon(&json["observations"], group);
        assert_eq!(group_horizon["count"]["observed"], 0, "{json:#}");
        assert_eq!(group_horizon["count"]["closure"], "closed", "{json:#}");
    }
}

#[test]
fn unavailable_body_keeps_every_directory_cone_group_open() {
    let repo = nested_directory_fixture();
    write(
        &repo.path().join("src/domain/e/huge.ts"),
        &"x".repeat(901_000),
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("unavailable directory cone cache").path(),
        &["cone", NESTED_DIRECTORY_ANCHOR, "--format", "json"],
    );
    for (group, _) in DIRECTORY_CONE_GROUPS {
        let group_horizon = horizon(&json["observations"], group);
        assert_eq!(group_horizon["count"]["closure"], "open", "{group}: {json:#}");
        assert!(
            group_horizon["unsupported"]
                .as_array()
                .expect("unsupported")
                .iter()
                .any(|gap| gap["file"] == "src/domain/e/huge.ts"),
            "{group}: {json:#}"
        );
    }
}

#[test]
fn dynamic_and_unresolved_flows_keep_directory_cone_relations_open() {
    let repo = nested_directory_fixture();
    write(
        &repo.path().join("src/domain/dynamic.ts"),
        "import { absent } from './missing';\nconst path = './a';\nexport const load = () => import(path);\nvoid absent;\n",
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("dynamic directory cone cache").path(),
        &["cone", NESTED_DIRECTORY_ANCHOR, "--format", "json"],
    );
    for group in ["outgoing", "incoming", "verification", "contracts"] {
        let group_horizon = horizon(&json["observations"], group);
        assert_eq!(group_horizon["count"]["closure"], "open", "{group}: {json:#}");
        let reasons = group_horizon["count"]["reasons"].as_array().expect("reasons");
        assert!(reasons.iter().any(|reason| reason == "dynamic_import_flow"));
        assert!(reasons.iter().any(|reason| reason == "incomplete_traversal"));
    }
}

#[test]
fn malformed_manifest_keeps_directory_cone_candidate_bases_open() {
    let repo = nested_directory_fixture();
    write(&repo.path().join("src/domain/package.json"), "{ malformed");
    let json = run_json(
        repo.path(),
        TempDir::new().expect("malformed directory cone cache").path(),
        &["cone", NESTED_DIRECTORY_ANCHOR, "--format", "json"],
    );
    for (group, _) in DIRECTORY_CONE_GROUPS {
        let group_horizon = horizon(&json["observations"], group);
        assert_eq!(group_horizon["count"]["closure"], "open", "{group}: {json:#}");
        assert!(
            group_horizon["unsupported"]
                .as_array()
                .expect("unsupported")
                .iter()
                .any(|gap| gap["file"] == "src/domain/package.json"),
            "{group}: {json:#}"
        );
    }
}

#[test]
fn directory_cone_cache_preserves_the_five_group_ledger() {
    let repo = nested_directory_fixture();
    let cache = TempDir::new().expect("directory cone warm cache");
    let args = [
        "cone",
        NESTED_DIRECTORY_ANCHOR,
        "--depth",
        "1",
        "--limit",
        "2",
        "--format",
        "json",
    ];
    let cold = run_json(repo.path(), cache.path(), &args);
    let warm = run_json(repo.path(), cache.path(), &args);
    assert_eq!(warm, cold, "warm directory cone must preserve the complete ledger");
    let artifact: Value = serde_json::from_str(
        &fs::read_to_string(lens_artifact_path(cache.path(), "cone-current.json"))
            .expect("directory cone artifact"),
    )
    .expect("directory cone artifact json");
    assert_eq!(
        artifact["report"]["observations"]["horizons"]
            .as_array()
            .expect("cached horizons")
            .len(),
        5,
        "{artifact:#}"
    );
}
