#[test]
fn schema_contract_role_ignores_engine_implementation_names() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "contract-role-precision",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("src/map/lenses/contract.rs"),
        "pub fn contract_report() -> bool {\n    true\n}\n",
    );
    write(
        &repo.path().join("src/cli/schema_and_roots.rs"),
        "pub fn schema_and_roots() -> bool {\n    true\n}\n",
    );
    write(
        &repo.path().join("src/repo/component_contracts_core.rs"),
        "pub fn component_contracts_core() -> bool {\n    true\n}\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "implementation names"]);

    for path in [
        "src/map/lenses/contract.rs",
        "src/cli/schema_and_roots.rs",
        "src/repo/component_contracts_core.rs",
    ] {
        let file = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &file);
        assert_ne!(
            file["anchor"]["kind"], "schema_contract",
            "{path} should not become schema_contract from implementation naming alone: {file:#}"
        );
        assert!(
            file["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .all(|role| role != "schema_contract"),
            "{path} should not carry schema_contract role from substring matches: {file:#}"
        );
    }

    let contract = run_json(
        repo.path(),
        cache.path(),
        &[
            "contract",
            "src/map/lenses/contract.rs",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/contract.schema.json", &contract);
    assert_eq!(
        contract["contract_kind"], "export_surface",
        "contract lens may expose exports, but must not claim implementation file is a schema contract: {contract:#}"
    );

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "src/map/lenses/contract.rs",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    let cluster = &impact["clusters"][0];
    assert!(
        !cluster["reasons"]
            .as_array()
            .expect("reasons")
            .iter()
            .any(|reason| reason == "schema or DTO contract changed"
                || reason == "contract surface participates"),
        "impact should not inflate implementation contract.rs into contract link: {impact:#}"
    );
    assert!(
        cluster["contract_links"]
            .as_array()
            .expect("contract links")
            .iter()
            .all(|edge| edge["type"] != "contract_changed"),
        "implementation contract.rs must not emit a contract_changed edge: {impact:#}"
    );
}

#[test]
fn schema_contract_role_preserves_explicit_schema_and_contract_surfaces() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "schema-contract-positive",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("schemas/report.schema.json"),
        "{ \"type\": \"object\" }\n",
    );
    write(
        &repo.path().join("src/schema/user.dto.ts"),
        "export interface UserDto { id: string }\n",
    );
    write(
        &repo.path().join("src/types.ts"),
        "export interface FrameDto { frame: number }\n",
    );
    write(
        &repo.path().join("src/types.d.ts"),
        "export interface DeclaredFrameDto { frame: number }\n",
    );
    write(
        &repo.path().join("src/schema.d.ts"),
        "export interface DeclaredSchema { id: string }\n",
    );
    write(
        &repo.path().join("src/contracts/auth.ts"),
        "export interface AuthContract { userId: string }\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "real contracts"]);

    for path in [
        "schemas/report.schema.json",
        "src/schema/user.dto.ts",
        "src/types.ts",
        "src/types.d.ts",
        "src/schema.d.ts",
        "src/contracts/auth.ts",
    ] {
        let file = run_json(repo.path(), cache.path(), &["ls", path, "--format", "json"]);
        assert_schema("schemas/ls.schema.json", &file);
        assert!(
            file["anchor"]["roles"]
                .as_array()
                .expect("roles")
                .iter()
                .any(|role| role == "schema_contract"),
            "{path} should remain an explicit schema/contract surface: {file:#}"
        );
    }

    let impact = run_json(
        repo.path(),
        cache.path(),
        &[
            "impact",
            "--files",
            "src/types.ts",
            "--format",
            "json",
        ],
    );
    assert_schema("schemas/impact.schema.json", &impact);
    assert!(
        impact["clusters"][0]["contract_links"]
            .as_array()
            .expect("contract links")
            .iter()
            .any(|edge| edge["type"] == "contract_changed"
                && edge["evidence"] == "role:schema_contract"),
        "real types/schema surfaces should still emit contract_changed evidence: {impact:#}"
    );
}
