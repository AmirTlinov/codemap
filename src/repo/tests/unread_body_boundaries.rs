// Responsibility: repo-unread-body-boundary-tests
use crate::repo::tests::playwright::write_test_file;
use crate::repo::{CacheWriteMode, RootSelection, load_project_with_cache};

#[cfg(unix)]
#[test]
fn symlinked_manifests_and_tsconfig_do_not_create_body_facts() {
    use std::os::unix::fs::symlink;

    let workspace = tempfile::TempDir::new().expect("unread symlink workspace");
    let repo = workspace.path().join("repo");
    let external = workspace.path().join("external");
    std::fs::create_dir_all(repo.join("src")).expect("repo source directory");
    std::fs::create_dir_all(repo.join("secret-domain")).expect("candidate domain");
    std::fs::create_dir_all(&external).expect("external directory");
    write_test_file(
        &external.join("package.json"),
        r#"{"name":"external-secret","workspaces":["secret-domain"]}"#,
    );
    write_test_file(
        &external.join("tsconfig.json"),
        r#"{"compilerOptions":{"baseUrl":".","paths":{"@leak/*":["src/*"]}}}"#,
    );
    symlink(external.join("package.json"), repo.join("package.json")).expect("package symlink");
    symlink(external.join("tsconfig.json"), repo.join("tsconfig.json")).expect("tsconfig symlink");
    write_test_file(
        &repo.join("src/app.ts"),
        "import { secret } from '@leak/secret';\nexport const app = secret;\n",
    );
    write_test_file(&repo.join("src/secret.ts"), "export const secret = 1;\n");
    write_test_file(&repo.join("secret-domain/README.md"), "candidate\n");

    let project =
        load_project_with_cache(RootSelection::Exact(repo.clone()), CacheWriteMode::ReadOnly)
            .expect("load symlink boundary project");

    for rel in ["package.json", "tsconfig.json"] {
        let file = project.files.get(rel).expect("indexed symlink placeholder");
        assert!(file.content_hash.is_none(), "{rel}: {file:#?}");
    }
    assert!(
        project
            .files
            .get("package.json")
            .is_some_and(|file| file.roles.contains("manifest")),
        "path-derived manifest role must survive the unread body boundary"
    );
    assert!(project.packages.is_empty(), "{:#?}", project.packages);
    assert!(
        project
            .domains
            .iter()
            .all(|domain| domain.path != "secret-domain"),
        "external workspace patterns leaked into domains: {:#?}",
        project.domains
    );
    assert!(
        project
            .files
            .get("src/app.ts")
            .is_some_and(|file| !file.resolved_imports.contains("src/secret.ts")),
        "external tsconfig aliases must not resolve imports"
    );
}

#[test]
fn oversized_bodies_keep_path_roles_without_manifest_or_header_facts() {
    let repo = tempfile::TempDir::new().expect("oversized body repo");
    std::fs::create_dir_all(repo.path().join("src/services")).expect("source directory");
    std::fs::create_dir_all(repo.path().join("secret-domain")).expect("candidate domain");
    let package = format!(
        "{{\"name\":\"oversized-secret\",\"workspaces\":[\"secret-domain\"],\"padding\":\"{}\"}}\n",
        "x".repeat(910_000)
    );
    write_test_file(&repo.path().join("package.json"), &package);
    let source = format!("// @generated\n#[cfg(test)]\n{}\n", "x".repeat(910_000));
    write_test_file(&repo.path().join("src/services/plain.rs"), &source);
    write_test_file(&repo.path().join("secret-domain/README.md"), "candidate\n");

    let project = load_project_with_cache(
        RootSelection::Exact(repo.path().to_path_buf()),
        CacheWriteMode::ReadOnly,
    )
    .expect("load oversized boundary project");
    let package = project
        .files
        .get("package.json")
        .expect("package placeholder");
    let source = project
        .files
        .get("src/services/plain.rs")
        .expect("source placeholder");
    assert!(package.content_hash.is_none(), "{package:#?}");
    assert!(source.content_hash.is_none(), "{source:#?}");
    assert!(package.roles.contains("manifest"), "{package:#?}");
    assert!(source.roles.contains("service"), "{source:#?}");
    assert!(!source.roles.contains("generated"), "{source:#?}");
    assert!(!source.roles.contains("test_support"), "{source:#?}");
    assert!(project.packages.is_empty(), "{:#?}", project.packages);
    assert!(
        project
            .domains
            .iter()
            .all(|domain| domain.path != "secret-domain"),
        "oversized workspace patterns leaked into domains: {:#?}",
        project.domains
    );
}

#[cfg(unix)]
#[test]
fn symlinked_cargo_workspace_cannot_create_package_edges() {
    use std::os::unix::fs::symlink;

    let workspace = tempfile::TempDir::new().expect("cargo symlink workspace");
    let repo = workspace.path().join("repo");
    let external = workspace.path().join("external-Cargo.toml");
    std::fs::create_dir_all(repo.join("crates/app")).expect("app directory");
    std::fs::create_dir_all(repo.join("crates/lib")).expect("lib directory");
    write_test_file(
        &external,
        r#"[workspace]
members = ["crates/app", "crates/lib"]

[workspace.dependencies]
fixture-lib = { path = "crates/lib" }
"#,
    );
    symlink(&external, repo.join("Cargo.toml")).expect("workspace manifest symlink");
    write_test_file(
        &repo.join("crates/app/Cargo.toml"),
        r#"[package]
name = "fixture-app"
version = "0.1.0"
edition = "2024"

[dependencies]
fixture-lib = { workspace = true }
"#,
    );
    write_test_file(
        &repo.join("crates/lib/Cargo.toml"),
        r#"[package]
name = "fixture-lib"
version = "0.1.0"
edition = "2024"
"#,
    );

    let project = load_project_with_cache(RootSelection::Exact(repo), CacheWriteMode::ReadOnly)
        .expect("load cargo symlink boundary project");
    assert_eq!(project.packages.len(), 2, "{:#?}", project.packages);
    assert!(
        project.package_edges.iter().all(|edge| {
            edge.workspace_manifest.as_deref() != Some("Cargo.toml")
                && edge.source != "Cargo.toml workspace dependency"
        }),
        "external cargo workspace leaked package edges: {:#?}",
        project.package_edges
    );
}

#[test]
fn fresh_project_reads_the_source_bytes_it_indexed() {
    let repo = tempfile::TempDir::new().expect("indexed source repo");
    let source = repo.path().join("src/app.ts");
    write_test_file(&source, "export const value = 'indexed';\n");
    let project = load_project_with_cache(
        RootSelection::Exact(repo.path().to_path_buf()),
        CacheWriteMode::ReadOnly,
    )
    .expect("load indexed source project");

    write_test_file(&source, "export const value = 'later';\n");
    assert_eq!(
        project.read_indexed_text("src/app.ts").as_deref(),
        Some("export const value = 'indexed';\n")
    );
}
