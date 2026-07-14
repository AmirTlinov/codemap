// Responsibility: repo-packages-go-mod
use crate::model::{PackageDependency, PackageInfo};
use crate::repo::{resolve_repo_relative_path, unique_strings};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn go_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_name: &BTreeMap<String, &PackageInfo>,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let replaces = go_replaces(&text);
    let mut edges = Vec::new();
    for dep in go_requires(&text) {
        if let Some(replacement) = replaces.get(&dep) {
            if let Some(target) = by_name.get(replacement) {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    workspace_manifest: None,
                    dependency: dep,
                    dependency_kind: "runtime".to_string(),
                    source: "go.mod replace".to_string(),
                });
                continue;
            }
            if let Some(target_path) = resolve_repo_relative_path(base, replacement)
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
                    source: "go.mod local replace".to_string(),
                });
                continue;
            }
        }
        if let Some(target) = by_name.get(&dep) {
            edges.push(PackageDependency {
                from: package.path.clone(),
                from_manifest: package.manifest.clone(),
                to: target.path.clone(),
                to_manifest: Some(target.manifest.clone()),
                workspace_manifest: None,
                dependency: dep,
                dependency_kind: "runtime".to_string(),
                source: "go.mod require".to_string(),
            });
        }
    }
    edges
}

pub(crate) fn go_module_name(text: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if let Some(value) = trimmed.strip_prefix("module ") {
            return value
                .split_whitespace()
                .next()
                .map(str::to_string)
                .filter(|value| !value.is_empty());
        }
    }
    None
}

fn go_requires(text: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some(module) = trimmed.split_whitespace().next()
                && !module.is_empty()
            {
                deps.push(module.to_string());
            }
            continue;
        }
        if trimmed == "require (" {
            in_block = true;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("require ") {
            if value.trim_start().starts_with('(') {
                in_block = true;
                continue;
            }
            if let Some(module) = value.split_whitespace().next()
                && !module.is_empty()
            {
                deps.push(module.to_string());
            }
        }
    }
    unique_strings(deps)
}

fn go_replaces(text: &str) -> BTreeMap<String, String> {
    let mut replaces = BTreeMap::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = strip_go_mod_comment(line).trim();
        if trimmed.is_empty() {
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some((from, to)) = parse_go_replace(trimmed) {
                replaces.insert(from, to);
            }
            continue;
        }
        if trimmed == "replace (" {
            in_block = true;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("replace ") {
            if value.trim_start().starts_with('(') {
                in_block = true;
                continue;
            }
            if let Some((from, to)) = parse_go_replace(value) {
                replaces.insert(from, to);
            }
        }
    }
    replaces
}

fn parse_go_replace(value: &str) -> Option<(String, String)> {
    let (from, to) = value.split_once("=>")?;
    let from = from.split_whitespace().next()?.to_string();
    let to = to.split_whitespace().next()?.to_string();
    (!from.is_empty() && !to.is_empty()).then_some((from, to))
}

fn strip_go_mod_comment(line: &str) -> &str {
    line.split_once("//").map(|(head, _)| head).unwrap_or(line)
}

pub(crate) fn extract_go_imports(text: &str) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
                continue;
            }
            if let Some(path) = quoted_go_import(trimmed) {
                imports.insert(path);
            }
            continue;
        }
        let Some(value) = trimmed.strip_prefix("import") else {
            continue;
        };
        let value = value.trim_start();
        if value.starts_with('(') {
            in_block = true;
            let rest = value.trim_start_matches('(').trim();
            if !rest.is_empty()
                && !rest.starts_with(')')
                && let Some(path) = quoted_go_import(rest)
            {
                imports.insert(path);
            }
            continue;
        }
        if let Some(path) = quoted_go_import(value) {
            imports.insert(path);
        }
    }
    imports
}

fn quoted_go_import(value: &str) -> Option<String> {
    let quote_start = value.find('"')?;
    let tail = &value[quote_start + 1..];
    let quote_end = tail.find('"')?;
    let path = &tail[..quote_end];
    (!path.is_empty()).then_some(path.to_string())
}
