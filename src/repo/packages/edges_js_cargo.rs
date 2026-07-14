// Responsibility: repo-packages-edges-js-cargo
use crate::model::{FileInfo, PackageDependency, PackageInfo};
use crate::repo::{go_package_edges, python_package_edges, swift_package_edges};
use std::collections::BTreeMap;
use std::path::Path;

mod cargo_edges;
mod js_deps;
mod rel_paths;

pub(crate) use cargo_edges::*;
pub(crate) use js_deps::*;
pub(crate) use rel_paths::*;

pub(crate) fn detect_package_edges(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    packages: &[PackageInfo],
) -> Vec<PackageDependency> {
    let mut edges = Vec::new();
    let by_name: BTreeMap<String, &PackageInfo> = packages
        .iter()
        .map(|package| (package.name.clone(), package))
        .collect();
    let by_path: BTreeMap<String, &PackageInfo> = packages
        .iter()
        .map(|package| (package.path.clone(), package))
        .collect();
    let cargo_workspaces = cargo_workspace_infos(root, files, packages, &by_path);

    for package in packages {
        match package.ecosystem.as_str() {
            "javascript" => {
                edges.extend(js_package_edges(root, package, &by_name, &by_path));
            }
            "rust" => {
                edges.extend(cargo_package_edges(
                    root,
                    package,
                    &by_path,
                    &cargo_workspaces,
                ));
            }
            "go" => {
                edges.extend(go_package_edges(root, package, &by_name, &by_path));
            }
            "python" => {
                edges.extend(python_package_edges(root, package, &by_path));
            }
            "swift" => {
                edges.extend(swift_package_edges(root, package, &by_path));
            }
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
