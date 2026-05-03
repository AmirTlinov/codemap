#[test]
fn root_ls_hides_agent_support_dirs_and_edges_by_default() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "agent-support-hidden-fixture",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("src/domain.ts"),
        "export const domainValue = true;\n",
    );
    write(
        &repo.path().join("packages/app/package.json"),
        r#"{
  "name": "@fixture/app",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("packages/app/src/domain.ts"),
        "export const packageDomainValue = true;\n",
    );
    write(
        &repo.path().join(".agents/probes/domain-probe.ts"),
        "import { domainValue } from '../../src/domain';\nexport const probe = domainValue;\n",
    );
    write(
        &repo
            .path()
            .join("packages/app/.agents/probes/domain-probe.ts"),
        "import { packageDomainValue } from '../../src/domain';\nexport const probe = packageDomainValue;\n",
    );
    write(&repo.path().join(".codex/cache-note.md"), "# local note\n");
    write(&repo.path().join(".claude/settings.json"), "{}\n");
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    let directory = json["directory"].as_array().expect("directory surfaces");
    assert!(
        directory.iter().all(|surface| surface["examples"]
            .as_array()
            .expect("examples")
            .iter()
            .all(|example| example != ".agents/" && example != ".codex/" && example != ".claude/")),
        "agent support dirs should not be default root surfaces: {json:#}"
    );
    assert!(
        json["edges"]
            .as_array()
            .expect("edges")
            .iter()
            .all(|edge| !edge["from"]
                .as_str()
                .unwrap_or_default()
                .starts_with(".agents/")
                && !edge["to"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with(".agents/")),
        "agent support imports should not appear as default root edges: {json:#}"
    );
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"] == "support artifacts hidden"),
        "hidden support count should make the omission explicit: {json:#}"
    );

    let expanded = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--all", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &expanded);
    assert!(
        expanded["directory"]
            .as_array()
            .expect("expanded directory")
            .iter()
            .any(|surface| surface["kind"] == "agent_support"
                && surface["examples"]
                    .as_array()
                    .expect("examples")
                    .iter()
                    .any(|example| example == ".agents/")),
        "include-hidden should expose local agent support surfaces: {expanded:#}"
    );

    let scoped = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".agents", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &scoped);
    assert!(
        scoped["edges"]
            .as_array()
            .expect("scoped edges")
            .iter()
            .any(|edge| edge["from"] == ".agents/probes/"
                && edge["to"] == "src/"
                && edge["type"] == "outgoing_import"),
        "scoped agent support maps should still show their structural edges: {scoped:#}"
    );

    let package = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &package);
    assert!(
        package["directory"]
            .as_array()
            .expect("package directory")
            .iter()
            .all(|surface| surface["examples"]
                .as_array()
                .expect("examples")
                .iter()
                .all(|example| example != "packages/app/.agents/")),
        "nested agent support dirs should not be default package-level surfaces: {package:#}"
    );
    assert!(
        package["edges"]
            .as_array()
            .expect("package edges")
            .iter()
            .all(|edge| !edge["from"]
                .as_str()
                .unwrap_or_default()
                .starts_with("packages/app/.agents/")
                && !edge["to"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("packages/app/.agents/")),
        "nested agent support imports should not appear as default package-level edges: {package:#}"
    );

    let package_expanded = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app", "--all", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &package_expanded);
    assert!(
        package_expanded["directory"]
            .as_array()
            .expect("package expanded directory")
            .iter()
            .any(|surface| surface["kind"] == "agent_support"
                && surface["examples"]
                    .as_array()
                    .expect("examples")
                    .iter()
                    .any(|example| example == "packages/app/.agents/")),
        "include-hidden should expose nested agent support surfaces: {package_expanded:#}"
    );

    let package_scoped = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/app/.agents", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &package_scoped);
    assert!(
        package_scoped["edges"]
            .as_array()
            .expect("package scoped edges")
            .iter()
            .any(|edge| edge["from"] == "packages/app/.agents/probes/"
                && edge["to"] == "packages/app/"
                && edge["type"] == "outgoing_import"),
        "scoped nested agent support maps should still show their structural edges: {package_scoped:#}"
    );
}


#[test]
fn root_ls_balances_directory_edges_across_structural_sources() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "balanced-root-fixture",
  "private": true
}
"#,
    );
    for index in 0..30 {
        write(
            &repo
                .path()
                .join(format!("packages/noisy-{index:02}/src/index.ts")),
            &format!("export const noisy{index:02} = {index};\n"),
        );
        write(
            &repo
                .path()
                .join(format!("apps/control-center/src/use-{index:02}.ts")),
            &format!(
                "import {{ noisy{index:02} }} from '../../../packages/noisy-{index:02}/src/index';\nexport const use{index:02} = noisy{index:02};\n"
            ),
        );
    }
    write(
        &repo.path().join("packages/shared/src/index.ts"),
        "export const shared = true;\n",
    );
    write(
        &repo.path().join("services/api/src/use-shared.ts"),
        "import { shared } from '../../../packages/shared/src/index';\nexport const apiUsesShared = shared;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(
        repo.path(),
        cache.path(),
        &["ls", ".", "--limit", "8", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &json);
    let edges = json["edges"].as_array().expect("edges");
    assert_eq!(edges.len(), 8);

    let froms = edges
        .iter()
        .map(|edge| edge["from"].as_str().expect("edge from"))
        .collect::<Vec<_>>();
    assert!(
        froms.contains(&"apps/control-center/"),
        "noisy source should still be represented: {json:#}"
    );
    assert!(
        froms.contains(&"services/api/"),
        "bounded root map should preserve a second structural source instead of letting one source consume the edge budget: {json:#}"
    );
    assert!(
        froms
            .iter()
            .filter(|from| **from == "apps/control-center/")
            .count()
            < edges.len(),
        "default root edge budget must not be monopolized by one noisy source: {json:#}"
    );
    assert!(
        json["hidden"]
            .as_array()
            .expect("hidden")
            .iter()
            .any(|hidden| hidden["reason"] == "directory edges hidden by limit"),
        "hidden edge count should still make the bounded cut explicit: {json:#}"
    );
    assert_eq!(json.get("read_first"), None);
}


#[test]
fn root_ls_collapses_nested_manifest_edges_to_current_level_package() {
    let repo = TempDir::new().expect("repo tempdir");
    let cache = TempDir::new().expect("cache tempdir");
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "a@example.com"]);
    git(repo.path(), &["config", "user.name", "a"]);
    write(
        &repo.path().join("package.json"),
        r#"{
  "name": "nested-manifest-root-map-fixture",
  "private": true,
  "workspaces": ["packages/*"]
}
"#,
    );
    write(
        &repo.path().join("packages/storefront-kit/package.json"),
        r#"{
  "name": "@fixture/storefront-kit",
  "private": true
}
"#,
    );
    write(
        &repo
            .path()
            .join("packages/storefront-kit/src/ui/hooks/package.json"),
        r#"{
  "name": "@fixture/storefront-hooks",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("packages/contracts/package.json"),
        r#"{
  "name": "@fixture/contracts",
  "private": true
}
"#,
    );
    write(
        &repo.path().join("packages/contracts/src/index.ts"),
        "export const contractValue = true;\n",
    );
    write(
        &repo
            .path()
            .join("packages/storefront-kit/src/ui/hooks/use-contract.ts"),
        "import { contractValue } from '../../../../contracts/src/index';\nexport const useContract = () => contractValue;\n",
    );
    git(repo.path(), &["add", "."]);
    git(repo.path(), &["commit", "-qm", "fixture"]);

    let json = run_json(repo.path(), cache.path(), &["ls", ".", "--format", "json"]);
    assert_schema("schemas/ls.schema.json", &json);
    let edges = json["edges"].as_array().expect("edges");
    assert!(
        edges.iter().any(|edge| edge["type"] == "outgoing_import"
            && edge["from"] == "packages/storefront-kit/"
            && edge["to"] == "packages/contracts/"),
        "root map should show the current-level package relation: {json:#}"
    );
    assert!(
        edges.iter().all(
            |edge| edge["from"] != "packages/storefront-kit/src/ui/hooks/"
                && edge["to"] != "packages/storefront-kit/src/ui/hooks/"
        ),
        "root map must not leak nested package internals as top-level edge endpoints: {json:#}"
    );
    assert_eq!(json.get("read_first"), None);

    let scoped = run_json(
        repo.path(),
        cache.path(),
        &["ls", "packages/storefront-kit/src/ui", "--format", "json"],
    );
    assert_schema("schemas/ls.schema.json", &scoped);
    let scoped_edges = scoped["edges"].as_array().expect("scoped edges");
    assert!(
        scoped_edges
            .iter()
            .any(|edge| edge["type"] == "outgoing_import"
                && edge["from"] == "packages/storefront-kit/src/ui/hooks/"
                && edge["to"] == "packages/contracts/"),
        "scoped maps should still expose the nested relation once the agent drills into that level: {scoped:#}"
    );
}
