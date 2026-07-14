// Responsibility: repo-tests-package-paths
use crate::repo::{
    cargo_workspace_member_pattern_matches, pyproject_package_name, pyproject_path_dependencies,
    pyproject_workspace_patterns, resolve_repo_relative_path,
};
use std::path::Path;

#[test]
fn cargo_paths_resolve_inside_repo_without_root_escape() {
    assert_eq!(
        resolve_repo_relative_path(Path::new("crates/app"), "../renderer").as_deref(),
        Some("crates/renderer")
    );
    assert_eq!(
        resolve_repo_relative_path(Path::new("."), "crates/replay").as_deref(),
        Some("crates/replay")
    );
    assert_eq!(
        resolve_repo_relative_path(Path::new("."), "../external"),
        None
    );
    assert_eq!(
        resolve_repo_relative_path(Path::new("nested"), "../../external"),
        None
    );
    assert_eq!(
        resolve_repo_relative_path(Path::new("."), "/tmp/external"),
        None
    );
    assert!(!cargo_workspace_member_pattern_matches(
        "external",
        "../external"
    ));
}

#[test]
fn pyproject_paths_use_structural_toml() {
    let pyproject = r#"[project]
name = "codemap-renderer"

[tool.uv.sources]
codemap-replay = { path = "../replay,with-comma", marker = "platform_system == 'Darwin,macOS'" }

[tool.poetry.dependencies]
codemap-tools = { path = "../tools" }
codemap-version-only = "^1"
"#;
    assert_eq!(
        pyproject_package_name(pyproject).as_deref(),
        Some("codemap-renderer")
    );
    let deps = pyproject_path_dependencies(pyproject);
    assert!(
        deps.iter()
            .any(|(name, path)| name == "codemap-replay" && path == "../replay,with-comma")
    );
    assert!(
        deps.iter()
            .any(|(name, path)| name == "codemap-tools" && path == "../tools")
    );
    assert!(deps.iter().all(|(name, _)| name != "codemap-version-only"));
}

#[test]
fn pyproject_workspace_patterns_ignore_unrelated_tool_metadata() {
    let pyproject = r#"[project]
name = "codemap-python-workspace"
members = ["services/replay"]
packages = ["apps/api"]

[tool.uv.workspace]
members = ["libs/*"]

[tool.unrelated]
members = ["shadow/replay"]
packages = ["shadow/renderer"]

[tool.poetry]
packages = ["not-a-workspace"]
"#;
    let patterns = pyproject_workspace_patterns(pyproject);
    assert!(patterns.iter().any(|item| item == "services/replay"));
    assert!(patterns.iter().any(|item| item == "apps/api"));
    assert!(patterns.iter().any(|item| item == "libs/*"));
    assert!(patterns.iter().all(|item| item != "shadow/replay"));
    assert!(patterns.iter().all(|item| item != "shadow/renderer"));
    assert!(patterns.iter().all(|item| item != "not-a-workspace"));
}
