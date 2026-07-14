// Responsibility: repo-packages-js-deps
use crate::model::{PackageDependency, PackageInfo};
use crate::repo::{path_is_absolute_like, resolve_repo_relative_path};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn js_package_edges(
    root: &Path,
    package: &PackageInfo,
    by_name: &BTreeMap<String, &PackageInfo>,
    by_path: &BTreeMap<String, &PackageInfo>,
) -> Vec<PackageDependency> {
    let Ok(text) = fs::read_to_string(root.join(&package.manifest)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let base = Path::new(&package.manifest)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut edges = Vec::new();
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        let Some(map) = value.get(section).and_then(|v| v.as_object()) else {
            continue;
        };
        for (dep, spec) in map {
            if let Some(spec) = spec.as_str() {
                if let Some(path) = js_local_dependency_path(spec) {
                    if let Some(target_path) = resolve_repo_relative_path(base, &path)
                        && let Some(target) = by_path.get(&target_path)
                    {
                        edges.push(PackageDependency {
                            from: package.path.clone(),
                            from_manifest: package.manifest.clone(),
                            to: target.path.clone(),
                            to_manifest: Some(target.manifest.clone()),
                            workspace_manifest: None,
                            dependency: dep.clone(),
                            dependency_kind: js_dependency_kind(section).to_string(),
                            source: format!("package.json {section} local path"),
                        });
                    }
                    continue;
                }
                if js_dependency_spec_is_local_protocol(spec) {
                    continue;
                }
            }
            if let Some(target) = by_name.get(dep) {
                edges.push(PackageDependency {
                    from: package.path.clone(),
                    from_manifest: package.manifest.clone(),
                    to: target.path.clone(),
                    to_manifest: Some(target.manifest.clone()),
                    workspace_manifest: None,
                    dependency: dep.clone(),
                    dependency_kind: js_dependency_kind(section).to_string(),
                    source: format!("package.json {section}"),
                });
            }
        }
    }
    edges
}

fn js_dependency_kind(section: &str) -> &str {
    match section {
        "devDependencies" => "dev",
        "peerDependencies" => "peer",
        "optionalDependencies" => "optional",
        _ => "runtime",
    }
}

pub(crate) fn js_local_dependency_path(spec: &str) -> Option<String> {
    let spec = spec.trim();
    for prefix in ["file:", "link:", "portal:", "workspace:"] {
        if let Some(path) = spec.strip_prefix(prefix) {
            let path = path.trim();
            if path.starts_with("./") || path.starts_with("../") || path == "." || path == ".." {
                return Some(path.to_string());
            }
        }
    }
    None
}

pub(crate) fn js_dependency_spec_is_local_protocol(spec: &str) -> bool {
    let spec = spec.trim();
    if ["file:", "link:", "portal:"]
        .iter()
        .any(|prefix| spec.starts_with(prefix))
    {
        return true;
    }
    let Some(path) = spec.strip_prefix("workspace:") else {
        return false;
    };
    let path = path.trim().replace('\\', "/");
    path.starts_with("./")
        || path.starts_with("../")
        || path == "."
        || path == ".."
        || path_is_absolute_like(&path)
}
