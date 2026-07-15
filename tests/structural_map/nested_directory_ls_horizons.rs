// Responsibility: nested-directory-ls-horizon-contract
const NESTED_DIRECTORY_ANCHOR: &str = "src/domain";

#[test]
fn nested_directory_relations_are_bounded_in_readable_and_complete_in_json() {
    let repo = nested_directory_fixture();
    let readable = run_markdown(
        repo.path(),
        TempDir::new().expect("directory readable cache").path(),
        &["ls", NESTED_DIRECTORY_ANCHOR, "--limit", "2"],
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("directory json cache").path(),
        &[
            "ls",
            NESTED_DIRECTORY_ANCHOR,
            "--limit",
            "2",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/ls.schema.json", &json);
    assert_eq!(json["schema_version"], "13", "{json:#}");
    assert_eq!(json["edges"].as_array().expect("directory edges").len(), 5);
    let relations = horizon(&json["observations"], "relations");
    assert_eq!(relations["count"]["observed"], 5, "{json:#}");
    assert_eq!(relations["count"]["closure"], "closed", "{json:#}");
    assert_eq!(relations["shown"], 5, "{json:#}");
    assert_eq!(relations["hidden"], 0, "{json:#}");
    assert_horizon_certificate_resolves(&json["observations"], relations);
    let surface_groups = horizon(&json["observations"], "surface_groups");
    let surface_members = horizon(&json["observations"], "surface_members");
    assert_eq!(json["directory"].as_array().expect("surface groups").len(), 3);
    assert_eq!(surface_groups["count"]["observed"], 3, "{json:#}");
    assert_eq!(surface_groups["shown"], 3, "{json:#}");
    assert_eq!(surface_members["count"]["observed"], 12, "{json:#}");
    assert_eq!(surface_members["shown"], 12, "{json:#}");
    assert_horizon_certificate_resolves(&json["observations"], surface_groups);
    assert_horizon_certificate_resolves(&json["observations"], surface_members);
    assert_shared_surface_candidate_basis(&json, surface_groups, surface_members);
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .all(|group| !legacy_nested_reason(group["reason"].as_str().unwrap_or_default())),
        "nested visibility accounting belongs only to the horizons: {json:#}"
    );

    let digest = relations["count"]["certificate_id"]
        .as_str()
        .expect("relations certificate")
        .strip_prefix("coverage-v1:")
        .expect("coverage certificate");
    let row = readable
        .lines()
        .find(|line| line.starts_with("- relations:") && line.contains("cert=`v1:"))
        .expect("readable relation horizon");
    assert!(row.contains("counted(5)"), "{readable}");
    assert!(row.contains("shown=2 hidden=3"), "{readable}");
    assert!(row.contains(&format!("cert=`v1:{}`", &digest[..12])), "{readable}");
    assert!(!readable.contains("directory edges hidden by limit"), "{readable}");
    let group_row = readable
        .lines()
        .find(|line| line.starts_with("- surface_groups:"))
        .expect("readable surface group horizon");
    let member_row = readable
        .lines()
        .find(|line| line.starts_with("- surface_members:"))
        .expect("readable surface member horizon");
    assert!(group_row.contains("counted(3)"), "{readable}");
    assert!(group_row.contains("shown=2 hidden=1"), "{readable}");
    assert!(member_row.contains("counted(12)"), "{readable}");
    assert!(member_row.contains("shown=8 hidden=4"), "{readable}");
    assert!(!legacy_nested_reason_in(&readable), "{readable}");
}

#[test]
fn dynamic_and_unresolved_candidates_keep_directory_relations_open() {
    let repo = nested_directory_fixture();
    write(
        &repo.path().join("src/dynamic/loader.ts"),
        "import { absent } from '../missing/value';\nconst path = '../domain/a';\nexport const load = () => import(path);\nvoid absent;\n",
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("directory gap cache").path(),
        &["ls", NESTED_DIRECTORY_ANCHOR, "--format", "json"],
    );
    let relations = horizon(&json["observations"], "relations");
    assert_eq!(relations["count"]["observed"], 5, "{json:#}");
    assert_eq!(relations["count"]["closure"], "open", "{json:#}");
    let reasons = relations["count"]["reasons"].as_array().expect("reasons");
    assert!(reasons.iter().any(|reason| reason == "dynamic_import_flow"));
    assert!(reasons.iter().any(|reason| reason == "incomplete_traversal"));
    let id = relations["count"]["certificate_id"].as_str().expect("id");
    let certificate = &json["observations"]["certificates"][id];
    assert!(!certificate["dynamic_stops"].as_array().unwrap().is_empty());
    assert!(!certificate["unresolved_stops"].as_array().unwrap().is_empty());
}

#[test]
fn supported_empty_nested_directory_proves_zero_relations() {
    let repo = TempDir::new().expect("empty nested directory repo");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"empty-directory","private":true}"#,
    );
    write(&repo.path().join("src/empty/index.ts"), "export const value = 1;\n");
    let json = run_json(
        repo.path(),
        TempDir::new().expect("empty directory cache").path(),
        &["ls", "src/empty", "--format", "json"],
    );
    let relations = horizon(&json["observations"], "relations");
    assert_eq!(relations["count"]["observed"], 0, "{json:#}");
    assert_eq!(relations["count"]["closure"], "closed", "{json:#}");
    assert!(json["edges"].as_array().expect("edges").is_empty());
}

#[test]
fn unavailable_relation_candidate_keeps_the_directory_horizon_open() {
    let repo = nested_directory_fixture();
    write(
        &repo.path().join("src/domain/e/huge.ts"),
        &"x".repeat(901_000),
    );
    let json = run_json(
        repo.path(),
        TempDir::new().expect("unavailable directory cache").path(),
        &["ls", NESTED_DIRECTORY_ANCHOR, "--format", "json"],
    );
    let relations = horizon(&json["observations"], "relations");
    assert_eq!(relations["count"]["closure"], "open", "{json:#}");
    assert!(
        relations["count"]["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|reason| reason == "unsupported_construct"),
        "{json:#}"
    );
    assert!(!relations["unsupported"].as_array().expect("unsupported").is_empty());
    for group in ["surface_groups", "surface_members"] {
        let surface = horizon(&json["observations"], group);
        assert_eq!(surface["count"]["closure"], "open", "{json:#}");
        assert!(
            surface["count"]["reasons"]
                .as_array()
                .expect("surface reasons")
                .iter()
                .any(|reason| reason == "unsupported_construct"),
            "{json:#}"
        );
    }
}

#[test]
fn malformed_manifest_keeps_the_nested_surface_basis_open() {
    let repo = nested_directory_fixture();
    write(&repo.path().join("src/domain/package.json"), "{ malformed");
    let json = run_json(
        repo.path(),
        TempDir::new().expect("malformed surface cache").path(),
        &["ls", NESTED_DIRECTORY_ANCHOR, "--format", "json"],
    );
    let groups = horizon(&json["observations"], "surface_groups");
    let members = horizon(&json["observations"], "surface_members");
    assert_eq!(groups["count"]["closure"], "open", "{json:#}");
    assert_eq!(members["count"]["closure"], "open", "{json:#}");
    assert_shared_surface_candidate_basis(&json, groups, members);
    assert!(
        groups["unsupported"]
            .as_array()
            .expect("unsupported manifests")
            .iter()
            .any(|gap| gap["file"] == "src/domain/package.json"),
        "{json:#}"
    );
}

#[test]
fn nested_directory_ls_cache_preserves_complete_relations() {
    let repo = nested_directory_fixture();
    let cache = TempDir::new().expect("nested directory warm cache");
    let args = [
        "ls",
        NESTED_DIRECTORY_ANCHOR,
        "--limit",
        "2",
        "--format",
        "json",
    ];
    let cold = run_json(repo.path(), cache.path(), &args);
    let warm = run_json(repo.path(), cache.path(), &args);
    assert_eq!(warm, cold, "warm nested-directory LS must preserve relations");
    let artifact: Value = serde_json::from_str(
        &fs::read_to_string(lens_artifact_path(cache.path(), "ls-current.json"))
            .expect("nested-directory ls artifact"),
    )
    .expect("nested-directory ls artifact json");
    assert_eq!(artifact["complete_directory_projection"], true, "{artifact:#}");
    assert_eq!(
        artifact["report"]["observations"]["horizons"]
            .as_array()
            .expect("cached horizons")
            .len(),
        3,
        "{artifact:#}"
    );
}

fn assert_shared_surface_candidate_basis(json: &Value, groups: &Value, members: &Value) {
    let certificates = &json["observations"]["certificates"];
    let group_certificate = &certificates[groups["count"]["certificate_id"].as_str().unwrap()];
    let member_certificate = &certificates[members["count"]["certificate_id"].as_str().unwrap()];
    for field in [
        "scope",
        "snapshot",
        "eligible_files",
        "visited_files",
        "excluded_files_by_reason",
        "extractor_capabilities",
        "unsupported",
        "closure",
        "reasons",
    ] {
        assert_eq!(group_certificate[field], member_certificate[field], "{field}: {json:#}");
    }
}

fn legacy_nested_reason(reason: &str) -> bool {
    [
        "directory edges hidden by limit",
        "directory surfaces hidden by limit",
        "generic source files hidden",
        "support packages hidden below support scopes",
        "support artifacts hidden",
        "recursive files below this level hidden",
    ]
    .contains(&reason)
}

fn legacy_nested_reason_in(output: &str) -> bool {
    output.lines().any(|line| {
        let label = line.trim_start().trim_start_matches("- ");
        legacy_nested_reason(label.split(':').next().unwrap_or(label))
    })
}

fn nested_directory_fixture() -> TempDir {
    let repo = TempDir::new().expect("nested directory ls repo");
    git(repo.path(), &["init", "-q"]);
    write(
        &repo.path().join("package.json"),
        r#"{"name":"nested-directory","private":true}"#,
    );
    write(
        &repo.path().join("src/domain/a/index.ts"),
        "import { b } from '../b';\nimport { c } from '../c';\nexport const a = b + c;\n",
    );
    write(
        &repo.path().join("src/domain/b/index.ts"),
        "import { c } from '../c';\nexport const b = c;\n",
    );
    write(
        &repo.path().join("src/domain/c/index.ts"),
        "import { d } from '../d';\nexport const c = d;\n",
    );
    write(&repo.path().join("src/domain/d/index.ts"), "export const d = 1;\n");
    write(
        &repo.path().join("src/outside.ts"),
        "import { a } from './domain/a';\nexport const outside = a;\n",
    );
    repo
}
