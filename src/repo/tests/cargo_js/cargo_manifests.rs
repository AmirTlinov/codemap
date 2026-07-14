// Responsibility: repo-tests-cargo-manifests
use crate::model::PackageInfo;
use crate::repo::tests::playwright::write_test_file;
use crate::repo::{
    CacheWriteMode, RootSelection, cargo_package_name, cargo_path_dependencies,
    cargo_workspace_array_values, cargo_workspace_dependency_names, cargo_workspace_infos,
    cargo_workspace_member_pattern_matches, cargo_workspace_path_dependencies,
    load_project_with_cache,
};
use std::collections::BTreeMap;

#[test]
fn cargo_workspace_table_and_dotted_forms_parse() {
    let workspace = r#"[workspace]
members = [
  "crates/app",
  "crates/renderer",
]
exclude = ["crates/ignored"]
dependencies.codemap_fixture_tools = { path = "crates/tools" }
dependencies.codemap_fixture_extra.path = "crates/extra"
dependencies.codemap_fixture_inline = { path = "crates/inline" }
dependencies.codemap_fixture_quoted = { version = "0.1, still a string", path = "crates/quoted,comma" }

[workspace.dependencies.codemap_fixture_replay]
path = "crates/replay"
"#;
    assert_eq!(
        cargo_workspace_array_values(workspace, "members"),
        vec!["crates/app".to_string(), "crates/renderer".to_string()]
    );
    assert_eq!(
        cargo_workspace_array_values(workspace, "exclude"),
        vec!["crates/ignored".to_string()]
    );
    let deps = cargo_workspace_path_dependencies(workspace);
    assert_eq!(
        deps.get("codemap_fixture_replay").map(String::as_str),
        Some("crates/replay")
    );
    assert_eq!(
        deps.get("codemap_fixture_tools").map(String::as_str),
        Some("crates/tools")
    );
    assert_eq!(
        deps.get("codemap_fixture_extra").map(String::as_str),
        Some("crates/extra")
    );
    assert_eq!(
        deps.get("codemap_fixture_inline").map(String::as_str),
        Some("crates/inline")
    );
    assert_eq!(
        deps.get("codemap_fixture_quoted").map(String::as_str),
        Some("crates/quoted,comma")
    );
    let root_dotted = r#"workspace.members = ["crates/app", "crates/replay"]
workspace.dependencies.codemap_fixture_root = { path = "crates/root" }
"#;
    assert_eq!(
        cargo_workspace_array_values(root_dotted, "members"),
        vec!["crates/app".to_string(), "crates/replay".to_string()]
    );
    assert_eq!(
        cargo_workspace_path_dependencies(root_dotted)
            .get("codemap_fixture_root")
            .map(String::as_str),
        Some("crates/root")
    );

    let package = r#"[dependencies]
codemap_fixture_replay.workspace = true
codemap_fixture_tools.workspace = true
codemap_fixture_inline = { version = "0.1, still a string", path = "crates/inline,comma" }

[dependencies.codemap_fixture_table]
workspace = true

[dev-dependencies]
codemap_fixture_test = { path = "crates/test" }

[build-dependencies]
codemap_fixture_build.workspace = true

[target.'cfg(unix)'.dependencies.codemap_fixture_target]
path = "crates/target"

[package.metadata.fake.dependencies.codemap_fixture_ignored]
path = "crates/ignored"
"#;
    assert_eq!(
        cargo_workspace_dependency_names(package),
        vec![
            ("codemap_fixture_replay".to_string(), "runtime".to_string()),
            ("codemap_fixture_table".to_string(), "runtime".to_string()),
            ("codemap_fixture_tools".to_string(), "runtime".to_string()),
            ("codemap_fixture_build".to_string(), "build".to_string())
        ]
    );
    let path_deps = cargo_path_dependencies(package);
    assert!(path_deps.iter().any(|(name, path, kind)| {
        name == "codemap_fixture_inline" && path == "crates/inline,comma" && kind == "runtime"
    }));
    assert!(path_deps.iter().any(|(name, path, kind)| {
        name == "codemap_fixture_test" && path == "crates/test" && kind == "dev"
    }));
    assert!(path_deps.iter().any(|(name, path, kind)| {
        name == "codemap_fixture_target" && path == "crates/target" && kind == "runtime"
    }));
    assert!(
        path_deps
            .iter()
            .all(|(name, _, _)| name != "codemap_fixture_ignored")
    );
    assert!(cargo_workspace_member_pattern_matches(
        "crates/renderer",
        "crates/renderer"
    ));
    assert!(cargo_workspace_member_pattern_matches(
        "crates/group/app",
        "crates/*/app"
    ));
}

#[test]
fn cargo_package_name_uses_structural_toml() {
    assert_eq!(
        cargo_package_name(
            r#"[package]
name = "codemap_fixture_renderer"
version = "0.1.0"
edition = "2024"
"#
        )
        .as_deref(),
        Some("codemap_fixture_renderer")
    );
}

#[test]
fn cargo_workspace_edges_use_workspace_dependency_tables() {
    let repo = tempfile::TempDir::new().expect("temp repo");
    write_test_file(
        &repo.path().join("Cargo.toml"),
        r#"[workspace]
members = [
  "crates/renderer",
  "crates/replay",
]

[workspace.dependencies.codemap_fixture_replay]
path = "crates/replay"
"#,
    );
    write_test_file(
        &repo.path().join("crates/renderer/Cargo.toml"),
        r#"[package]
name = "codemap_fixture_renderer"
version = "0.1.0"
edition = "2024"

[dependencies.codemap_fixture_replay]
workspace = true
"#,
    );
    write_test_file(
        &repo.path().join("crates/replay/Cargo.toml"),
        r#"[package]
name = "codemap_fixture_replay"
version = "0.1.0"
edition = "2024"
"#,
    );
    let project = load_project_with_cache(
        RootSelection::Exact(repo.path().to_path_buf()),
        CacheWriteMode::ReadOnly,
    )
    .expect("load project");
    let by_path: BTreeMap<String, &PackageInfo> = project
        .packages
        .iter()
        .map(|package| (package.path.clone(), package))
        .collect();
    let workspaces =
        cargo_workspace_infos(repo.path(), &project.files, &project.packages, &by_path);
    assert!(
        project.package_edges.iter().any(|edge| {
            edge.from == "crates/renderer"
                && edge.to == "crates/replay"
                && edge.source == "Cargo.toml workspace dependency"
        }),
        "files: {:#?}; packages: {:#?}; workspaces: {:#?}; package edges: {:#?}",
        project.files.keys().collect::<Vec<_>>(),
        project.packages,
        workspaces,
        project.package_edges
    );
}
