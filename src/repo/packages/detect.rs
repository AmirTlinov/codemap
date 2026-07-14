// Responsibility: repo-packages-detect
use crate::model::{FileInfo, PackageInfo};
use crate::repo::{
    cargo_package_name, go_module_name, manifest_dir, package_name_from_path,
    pyproject_package_name, swift_package_name,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn detect_packages(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<PackageInfo> {
    let mut packages = Vec::new();
    for rel in files.keys() {
        let name = Path::new(rel).file_name().and_then(|s| s.to_str());
        match name {
            Some("package.json") => {
                if let Some(package) = read_js_package(root, rel) {
                    packages.push(package);
                }
            }
            Some("Cargo.toml") => {
                if let Some(package) = read_cargo_package(root, rel) {
                    packages.push(package);
                }
            }
            Some("go.mod") => {
                if let Some(package) = read_go_package(root, rel) {
                    packages.push(package);
                }
            }
            Some("pyproject.toml") => {
                if let Some(package) = read_python_package(root, rel) {
                    packages.push(package);
                }
            }
            Some("Package.swift") => {
                if let Some(package) = read_swift_package(root, rel) {
                    packages.push(package);
                }
            }
            _ => {}
        }
    }
    packages.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.name.cmp(&b.name)));
    packages
}

fn read_js_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    let path = manifest_dir(rel);
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| package_name_from_path(&path));
    Some(PackageInfo {
        name,
        path,
        manifest: rel.to_string(),
        ecosystem: "javascript".to_string(),
    })
}

fn read_cargo_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let name = cargo_package_name(&text)?;
    Some(PackageInfo {
        name,
        path: manifest_dir(rel),
        manifest: rel.to_string(),
        ecosystem: "rust".to_string(),
    })
}

fn read_go_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let name = go_module_name(&text)?;
    Some(PackageInfo {
        name,
        path: manifest_dir(rel),
        manifest: rel.to_string(),
        ecosystem: "go".to_string(),
    })
}

fn read_python_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let path = manifest_dir(rel);
    let name = pyproject_package_name(&text).unwrap_or_else(|| package_name_from_path(&path));
    Some(PackageInfo {
        name,
        path,
        manifest: rel.to_string(),
        ecosystem: "python".to_string(),
    })
}

fn read_swift_package(root: &Path, rel: &str) -> Option<PackageInfo> {
    let text = fs::read_to_string(root.join(rel)).ok()?;
    let path = manifest_dir(rel);
    let name = swift_package_name(&text).unwrap_or_else(|| package_name_from_path(&path));
    Some(PackageInfo {
        name,
        path,
        manifest: rel.to_string(),
        ecosystem: "swift".to_string(),
    })
}
