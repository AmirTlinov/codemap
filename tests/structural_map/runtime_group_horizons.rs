// Responsibility: runtime-group-horizon-contract
const S03C_RUNTIME_GROUPS: [&str; 9] = [
    "entrypoints",
    "routes",
    "paths",
    "scripts",
    "env",
    "workers",
    "ci",
    "proof",
    "unknowns",
];

#[test]
fn runtime_flagship_groups_share_one_basis_across_bounded_and_full_projections() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readable_cache = TempDir::new().expect("runtime group readable cache");
    let json_cache = TempDir::new().expect("runtime group json cache");
    let readable = run_markdown(
        root,
        readable_cache.path(),
        &["runtime", "fixtures", "--limit", "3"],
    );
    let json = run_json(
        root,
        json_cache.path(),
        &["runtime", "fixtures", "--limit", "3", "--format", "json"],
    );
    assert_schema("schemas/runtime.schema.json", &json);
    assert_eq!(json["schema_version"], "6", "{json:#}");

    let observations = &json["observations"];
    let horizons = observations["horizons"]
        .as_array()
        .expect("runtime horizons");
    let certificates = observations["certificates"]
        .as_object()
        .expect("runtime certificates");
    assert_eq!(horizons.len(), 9, "{json:#}");
    assert_eq!(certificates.len(), 9, "{json:#}");
    assert_eq!(
        horizons
            .iter()
            .map(|item| item["group"].as_str().expect("runtime group"))
            .collect::<BTreeSet<_>>(),
        S03C_RUNTIME_GROUPS.into_iter().collect(),
        "{json:#}"
    );

    let expected = [
        ("entrypoints", 8, "open", 3, 5),
        ("routes", 8, "open", 3, 5),
        ("paths", 10, "open", 3, 7),
        ("scripts", 0, "open", 0, 0),
        ("env", 0, "open", 0, 0),
        ("workers", 0, "closed", 0, 0),
        ("ci", 0, "closed", 0, 0),
        ("proof", 0, "open", 0, 0),
        ("unknowns", 7, "open", 3, 4),
    ];
    let mut certificate_ids = BTreeSet::new();
    for (group, observed, closure, readable_shown, readable_hidden) in expected {
        let item = horizon(observations, group);
        assert_eq!(item["count"]["observed"], observed, "{group}: {json:#}");
        assert_eq!(item["count"]["closure"], closure, "{group}: {json:#}");
        assert_eq!(item["shown"], observed, "{group}: {json:#}");
        assert_eq!(item["hidden"], 0, "{group}: {json:#}");
        assert!(item["expand"].is_null(), "{group}: {json:#}");
        assert_horizon_certificate_resolves(observations, item);
        let certificate_id = item["count"]["certificate_id"]
            .as_str()
            .expect("certificate id");
        assert!(certificate_ids.insert(certificate_id), "{group}: {json:#}");
        let preview = format!(
            "v1:{}",
            certificate_id
                .strip_prefix("coverage-v1:")
                .expect("coverage certificate")
                .chars()
                .take(12)
                .collect::<String>()
        );
        let line = readable
            .lines()
            .find(|line| line.starts_with(&format!("- {group}:")))
            .unwrap_or_else(|| panic!("missing readable {group} horizon: {readable}"));
        assert!(line.contains(&format!("shown={readable_shown} hidden={readable_hidden}")));
        assert!(line.contains(&format!("cert=`{preview}`")), "{line}");
    }
    assert!(
        readable.contains("scripts: unknown lower bound: 0")
            && readable.contains("env: unknown lower bound: 0")
            && readable.contains("proof: unknown lower bound: 0")
            && readable.contains("workers: proven-zero")
            && readable.contains("ci: proven-zero"),
        "pilot zeroes must preserve their causal closure: {readable}"
    );
    for legacy in [
        "runtime entrypoints hidden by limit",
        "runtime path relations hidden by limit",
        "runtime scripts hidden by limit",
        "environment surfaces hidden by limit",
        "worker/job surfaces hidden by limit",
        "ci surfaces hidden by limit",
        "runtime verification edges hidden by limit",
        "runtime unknowns hidden by limit",
    ] {
        assert!(!readable.contains(legacy), "{legacy}: {readable}");
    }

    for group in ["workers", "ci"] {
        let certificate = runtime_group_certificate(&json, group);
        let eligible = certificate["eligible_files"]
            .as_u64()
            .expect("eligible files");
        assert!(eligible > 0, "{group}: {json:#}");
        assert_eq!(certificate["visited_files"], eligible, "{group}: {json:#}");
        for gaps in [
            "unsupported",
            "dynamic_stops",
            "unresolved_stops",
            "external_stops",
        ] {
            assert!(
                certificate[gaps].as_array().expect("coverage gaps").is_empty(),
                "{group}/{gaps}: {json:#}"
            );
        }
    }
    let scripts = runtime_group_certificate(&json, "scripts");
    assert!(scripts["eligible_files"].as_u64().unwrap_or(0) > 0, "{json:#}");
    assert_eq!(scripts["visited_files"], 0, "{json:#}");
    assert!(
        !scripts["excluded_files_by_reason"]["unsupported_construct"]
            .as_array()
            .expect("script exclusions")
            .is_empty()
            && !scripts["unresolved_stops"]
                .as_array()
                .expect("script stops")
                .is_empty(),
        "the pilot script zero must retain its root-only and nonexhaustive basis: {json:#}"
    );
    let env = runtime_group_certificate(&json, "env");
    assert!(!env["dynamic_stops"].as_array().unwrap().is_empty(), "{json:#}");
    assert!(!env["unresolved_stops"].as_array().unwrap().is_empty(), "{json:#}");
    let proof = runtime_group_certificate(&json, "proof");
    assert!(
        proof["unresolved_stops"]
            .as_array()
            .expect("proof stops")
            .iter()
            .any(|stop| stop["kind"] == "verification_relation_flow"),
        "{json:#}"
    );
    for group in ["entrypoints", "unknowns"] {
        assert!(
            !runtime_group_certificate(&json, group)["unsupported"]
                .as_array()
                .expect("unsupported observations")
                .is_empty(),
            "{group}: {json:#}"
        );
    }
}

#[test]
fn runtime_root_keeps_full_group_certificates_behind_current_level_display() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readable_cache = TempDir::new().expect("root group readable cache");
    let json_cache = TempDir::new().expect("root group json cache");
    let readable = run_markdown(root, readable_cache.path(), &["runtime", "."]);
    let json = run_json(
        root,
        json_cache.path(),
        &["runtime", ".", "--format", "json"],
    );
    for group in S03C_RUNTIME_GROUPS {
        let item = horizon(&json["observations"], group);
        let certificate_id = item["count"]["certificate_id"]
            .as_str()
            .expect("root certificate");
        let preview = &certificate_id["coverage-v1:".len()..][..12];
        let line = readable
            .lines()
            .find(|line| line.starts_with(&format!("- {group}:")))
            .unwrap_or_else(|| panic!("missing root {group} horizon: {readable}"));
        assert!(line.contains(&format!("cert=`v1:{preview}`")), "{group}: {line}");
        assert_eq!(item["shown"], item["count"]["observed"], "{group}: {json:#}");
        assert_eq!(item["hidden"], 0, "{group}: {json:#}");
    }
    assert!(
        readable.contains("recursive runtime files hidden at root scope"),
        "current-level presentation must retain its aggregate root boundary: {readable}"
    );
}
