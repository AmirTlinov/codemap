// Responsibility: repo-packages-python-swift
use crate::model::{PackageDependency, PackageInfo};
use crate::repo::{
    package_name_from_path, parse_toml_value, resolve_repo_relative_path, swift_package_name_re,
    swift_package_path_dependency_re, toml_path_field, unique_pairs,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn python_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for (dep, path) in pyproject_path_dependencies(&text) {
        if let Some(target_path) = resolve_repo_relative_path(base, &path)
            && let Some(target) = by_path.get(&target_path)
        {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: dep,
                dependency_kind: "runtime".to_string(),
                source: "pyproject local path dependency".to_string(),
            });
        }
    }
    edges
}

pub(crate) fn swift_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for path in swift_package_path_dependencies(&text) {
        if let Some(target_path) = resolve_repo_relative_path(base, &path)
            && let Some(target) = by_path.get(&target_path)
        {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: package_name_from_path(&target.path),
                dependency_kind: "runtime".to_string(),
                source: "Package.swift local path dependency".to_string(),
            });
        }
    }
    edges
}

pub(crate) fn pyproject_package_name(text: &str) -> Option<String> {
    let value = parse_toml_value(text)?;
    value
        .get("project")
        .and_then(|project| project.get("name"))
        .or_else(|| {
            value
                .get("tool")
                .and_then(|tool| tool.get("poetry"))
                .and_then(|poetry| poetry.get("name"))
        })
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .filter(|name| !name.is_empty())
}

pub(crate) fn pyproject_path_dependencies(text: &str) -> Vec<(String, String)> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut deps = Vec::new();
    if let Some(table) = value
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("sources"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.push((name.to_string(), path));
            }
        }
    }
    if let Some(table) = value
        .get("tool")
        .and_then(|tool| tool.get("poetry"))
        .and_then(|poetry| poetry.get("dependencies"))
        .and_then(toml::Value::as_table)
    {
        for (name, dependency) in table {
            if let Some(path) = toml_path_field(dependency) {
                deps.push((name.to_string(), path));
            }
        }
    }
    unique_pairs(deps)
}

pub(crate) fn swift_package_name(text: &str) -> Option<String> {
    swift_package_name_re()
        .captures(text)?
        .get(1)
        .map(|m| m.as_str().to_string())
        .filter(|name| !name.is_empty())
}

fn swift_package_path_dependencies(text: &str) -> Vec<String> {
    let mut deps = swift_package_path_dependency_re()
        .captures_iter(text)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .collect::<Vec<_>>();
    deps.sort();
    deps.dedup();
    deps
}
