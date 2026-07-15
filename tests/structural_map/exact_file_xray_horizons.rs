// Responsibility: complete-exact-file-xray-horizon-contract
const EXACT_FILE_XRAY_GROUPS: [(&str, &str, &str); 7] = [
    ("xray_roles", "roles", "file_xray_role_surfaces"),
    ("xray_outputs", "outputs", "file_xray_output_surfaces"),
    ("xray_state", "state", "file_xray_state_surfaces"),
    ("xray_side_effects", "side_effects", "file_xray_side_effect_surfaces"),
    ("xray_flow", "flow", "file_xray_flow_steps"),
    ("xray_nearby", "nearby", "file_xray_nearby_surfaces"),
    ("xray_unknowns", "unknowns", "file_xray_unknown_surfaces"),
];

#[test]
fn every_exact_file_xray_group_is_bounded_and_machine_complete() {
    let repo = xray_mass_fixture();
    let args = ["cone", "src/owner.ts", "--depth", "1", "--limit", "1"];
    let readable = run_markdown(
        repo.path(),
        TempDir::new().expect("xray mass readable cache").path(),
        &args,
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("xray mass json cache").path(),
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
    assert_eq!(json["observations"]["horizons"].as_array().unwrap().len(), 13);
    assert!(readable.contains("xray ledger: certified=7"), "{readable}");
    assert!(
        !readable.contains("xray_proof_"),
        "default visibility should account verification once through its canonical group: {readable}"
    );
    for (group, field, query_kind) in EXACT_FILE_XRAY_GROUPS {
        let item = horizon(&json["observations"], group);
        let facts = json["xray"][field].as_array().expect("X-Ray fact list").len();
        assert_eq!(item["count"]["observed"], facts, "{group}: {json:#}");
        assert_eq!(item["shown"], facts, "{group}: {json:#}");
        assert_horizon_certificate_resolves(&json["observations"], item);
        let certificate = &json["observations"]["certificates"]
            [item["count"]["certificate_id"].as_str().unwrap()];
        assert!(
            certificate["query_kind"]
                .as_str()
                .unwrap()
                .starts_with(query_kind),
            "{group}: {certificate:#}"
        );
        if facts > 3 {
            assert!(
                readable.lines().any(|line| {
                    line.starts_with(&format!("- {group}:"))
                        && line.contains(&format!("shown=3 hidden={}", facts - 3))
                }),
                "{group}: {readable}"
            );
        }
    }
    for detached in [
        "compact x-ray limit",
        "more structural flow steps hidden",
        "more Unknown entries hidden",
    ] {
        assert!(!readable.contains(detached), "{readable}");
    }
    for group in [
        "xray_outputs",
        "xray_nearby",
        "xray_unknowns",
    ] {
        assert!(
            horizon(&json["observations"], group)["count"]["observed"]
                .as_u64()
                .unwrap()
                > 3,
            "mass fixture did not saturate {group}: {json:#}"
        );
    }
}

#[test]
fn exact_file_xray_zero_and_unavailable_bases_remain_typed() {
    let empty = TempDir::new().expect("empty exact-file X-Ray repo");
    git(empty.path(), &["init", "-q"]);
    write(&empty.path().join("src/empty.ts"), "");
    let empty_json = run_json(
        empty.path(),
        TempDir::new().expect("empty exact-file X-Ray cache").path(),
        &["cone", "src/empty.ts", "--format", "json"],
    );
    for (group, _, _) in EXACT_FILE_XRAY_GROUPS {
        assert_eq!(
            horizon(&empty_json["observations"], group)["count"]["closure"],
            "closed",
            "{group}: {empty_json:#}"
        );
    }

    let unsupported = TempDir::new().expect("unsupported exact-file X-Ray repo");
    git(unsupported.path(), &["init", "-q"]);
    write(&unsupported.path().join("notes.txt"), "plain text\n");
    let unsupported_json = run_json(
        unsupported.path(),
        TempDir::new().expect("unsupported exact-file X-Ray cache").path(),
        &["cone", "notes.txt", "--format", "json"],
    );
    assert_eq!(
        horizon(&unsupported_json["observations"], "xray_outputs")["count"]["closure"],
        "unavailable",
        "{unsupported_json:#}"
    );
    assert_eq!(
        horizon(&unsupported_json["observations"], "xray_roles")["count"]["closure"],
        "closed",
        "path-derived roles stay available: {unsupported_json:#}"
    );

    let unavailable = TempDir::new().expect("unavailable exact-file X-Ray repo");
    git(unavailable.path(), &["init", "-q"]);
    write(&unavailable.path().join("src/huge.ts"), &"x".repeat(901_000));
    let unavailable_json = run_json(
        unavailable.path(),
        TempDir::new().expect("unavailable exact-file X-Ray cache").path(),
        &["cone", "src/huge.ts", "--format", "json"],
    );
    for (group, _, _) in EXACT_FILE_XRAY_GROUPS {
        assert_ne!(
            horizon(&unavailable_json["observations"], group)["count"]["closure"],
            "closed",
            "{group} forged completeness from an unavailable body: {unavailable_json:#}"
        );
    }
}

fn xray_mass_fixture() -> TempDir {
    let repo = TempDir::new().expect("exact-file X-Ray mass repo");
    git(repo.path(), &["init", "-q"]);
    let deps = ["schema", "repository", "validate", "checksum"];
    let mut owner = String::new();
    for (index, dep) in deps.iter().enumerate() {
        owner.push_str(&format!("import {{ v{index} }} from './{dep}';\n"));
    }
    for index in 0..4 {
        owner.push_str(&format!("import './missing-{index}';\n"));
    }
    owner.push_str(
        "export const one = v0;\nexport const two = v1;\nexport const three = v2;\nexport const four = v3;\n",
    );
    write(&repo.path().join("src/owner.ts"), &owner);
    for (index, dep) in deps.iter().enumerate() {
        write(
            &repo.path().join(format!("src/{dep}.ts")),
            &format!("export const v{index} = {index};\n"),
        );
    }
    for index in 0..4 {
        write(
            &repo.path().join(format!("src/consumer-{index}.ts")),
            "import { one } from './owner';\nvoid one;\n",
        );
        write(
            &repo.path().join(format!("tests/owner-{index}.test.ts")),
            "import { one } from '../src/owner';\ntest('owner', () => expect(one).toBe(0));\n",
        );
    }
    repo
}
