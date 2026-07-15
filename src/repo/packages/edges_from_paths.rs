// Responsibility: repo-package-edges-from-bounded-paths
use crate::model::{PackageDependency, PackageInfo};
use crate::repo::{
    cargo_path_dependencies, cargo_workspace_array_values, cargo_workspace_declared,
    cargo_workspace_dependency_names, cargo_workspace_member_pattern_matches,
    cargo_workspace_path_dependencies, go_package_edges, js_package_edges, manifest_dir,
    normalize_rel_path, python_package_edges, resolve_repo_relative_path, swift_package_edges,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Builds local package crossings from the bounded manifest inventory without
/// requiring a full source index. Every extractor reads only a manifest that
/// package discovery has already admitted to the candidate set.
pub(crate) fn detect_package_edges_from_paths(
    root: &Path,
    manifest_paths: &[String],
    packages: &[PackageInfo],
) -> Vec<PackageDependency> {
    let by_name = packages
        .iter()
        .map(|package| (package.name.clone(), package))
        .collect::<BTreeMap<_, _>>();
    let by_path = packages
        .iter()
        .map(|package| (package.path.clone(), package))
        .collect::<BTreeMap<_, _>>();
    let workspaces = bounded_cargo_workspaces(root, manifest_paths, packages, &by_path);
    let mut edges = Vec::new();
    for package in packages {
        match package.ecosystem.as_str() {
            "javascript" => edges.extend(js_package_edges(root, package, &by_name, &by_path)),
            "rust" => edges.extend(bounded_cargo_package_edges(
                root,
                package,
                &by_path,
                &workspaces,
            )),
            "go" => edges.extend(go_package_edges(root, package, &by_name, &by_path)),
            "python" => edges.extend(python_package_edges(root, package, &by_path)),
            "swift" => edges.extend(swift_package_edges(root, package, &by_path)),
            _ => {}
        }
    }
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.dependency.cmp(&b.dependency))
            .then_with(|| a.dependency_kind.cmp(&b.dependency_kind))
    });
    edges.dedup_by(|a, b| {
        a.from == b.from
            && a.to == b.to
            && a.dependency == b.dependency
            && a.dependency_kind == b.dependency_kind
            && a.source == b.source
    });
    edges
}

#[derive(Debug)]
struct BoundedCargoWorkspace {
    manifest: String,
    path: String,
    members: Vec<String>,
    exclude: Vec<String>,
    dependencies: BTreeMap<String, String>,
}

fn bounded_cargo_workspaces(
    root: &Path,
    manifest_paths: &[String],
    packages: &[PackageInfo],
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<BoundedCargoWorkspace> {
    let mut out = Vec::new();
    for rel in manifest_paths
        .iter()
        .filter(|rel| rel.ends_with("Cargo.toml"))
    {
        let Ok(text) = fs::read_to_string(root.join(rel)) else {
            continue;
        };
        if !cargo_workspace_declared(&text) {
            continue;
        }
        let path = manifest_dir(rel);
        let dependencies = cargo_workspace_path_dependencies(&text)
            .into_iter()
            .filter_map(|(name, dependency_path)| {
                resolve_repo_relative_path(Path::new(&path), &dependency_path)
                    .filter(|resolved| by_path.contains_key(resolved))
                    .map(|resolved| (name, resolved))
            })
            .collect();
        out.push(BoundedCargoWorkspace {
            manifest: rel.clone(),
            path,
            members: cargo_workspace_array_values(&text, "members"),
            exclude: cargo_workspace_array_values(&text, "exclude"),
            dependencies,
        });
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.retain(|workspace| {
        packages
            .iter()
            .any(|package| workspace_contains(workspace, &package.path))
    });
    out
}

fn bounded_cargo_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
    workspaces: &[BoundedCargoWorkspace],
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = cargo_path_dependencies(&text)
        .into_iter()
        .filter_map(|(name, path, kind)| {
            let target_path = resolve_repo_relative_path(base, &path)?;
            let target = by_path.get(&target_path)?;
            Some(package_dependency(
                package,
                target,
                name,
                kind,
                None,
                "Cargo.toml path dependency",
            ))
        })
        .collect::<Vec<_>>();
    let workspace = workspaces
        .iter()
        .filter(|workspace| workspace_contains(workspace, &package.path))
        .max_by_key(|workspace| workspace.path.len());
    if let Some(workspace) = workspace {
        for (name, kind) in cargo_workspace_dependency_names(&text) {
            let Some(path) = workspace.dependencies.get(&name) else {
                continue;
            };
            let Some(target) = by_path.get(path) else {
                continue;
            };
            edges.push(package_dependency(
                package,
                target,
                name,
                kind,
                Some(workspace.manifest.clone()),
                "Cargo.toml workspace dependency",
            ));
        }
    }
    edges
}

fn package_dependency(
    from: &PackageInfo,
    to: &&PackageInfo,
    dependency: String,
    dependency_kind: String,
    workspace_manifest: Option<String>,
    source: &str,
) -> PackageDependency {
    PackageDependency {
        from: from.path.clone(),
        from_manifest: from.manifest.clone(),
        to: to.path.clone(),
        to_manifest: Some(to.manifest.clone()),
        workspace_manifest,
        dependency,
        dependency_kind,
        source: source.to_string(),
    }
}

fn workspace_contains(workspace: &BoundedCargoWorkspace, package_path: &str) -> bool {
    let Some(rel) = relative_to(package_path, &workspace.path) else {
        return false;
    };
    if workspace
        .exclude
        .iter()
        .any(|pattern| cargo_workspace_member_pattern_matches(&rel, pattern))
    {
        return false;
    }
    rel == "."
        || workspace
            .members
            .iter()
            .any(|pattern| cargo_workspace_member_pattern_matches(&rel, pattern))
}

fn relative_to(path: &str, base: &str) -> Option<String> {
    let path = normalize_rel_path(path);
    let base = normalize_rel_path(base);
    if base == "." {
        return Some(path);
    }
    if path == base {
        return Some(".".to_string());
    }
    let prefix = PathBuf::from(&base).to_string_lossy().to_string() + "/";
    path.strip_prefix(&prefix).map(str::to_string)
}
