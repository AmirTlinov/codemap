// Responsibility: runtime-lens-cli-entrypoints
use crate::model::{EvidenceStrength, FileInfo, Project, Surface};
use crate::repo;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn runtime_manifest_entrypoints(project: &Project, file: &FileInfo) -> Vec<Surface> {
    let package = project
        .packages
        .iter()
        .find(|package| package.manifest == file.rel);
    let package_path = package
        .map(|package| package.path.clone())
        .unwrap_or_else(|| manifest_parent(&file.rel));
    let package_name = package
        .map(|package| package.name.clone())
        .unwrap_or_else(|| package_name_from_manifest_path(&package_path));
    let Some(name) = Path::new(&file.rel)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return Vec::new();
    };
    match name.to_ascii_lowercase().as_str() {
        "package.json" => {
            js_manifest_cli_entrypoints(project, &file.rel, &package_path, &package_name)
        }
        "cargo.toml" => {
            cargo_manifest_cli_entrypoints(project, &file.rel, &package_path, &package_name)
        }
        "pyproject.toml" => pyproject_manifest_cli_entrypoints(project, &file.rel, &package_path),
        _ => Vec::new(),
    }
}

fn js_manifest_cli_entrypoints(
    project: &Project,
    manifest: &str,
    package_path: &str,
    package_name: &str,
) -> Vec<Surface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(manifest)) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(bin) = value.get("bin") else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    if let Some(target) = bin.as_str() {
        entries.push((package_name.to_string(), target.to_string()));
    } else if let Some(map) = bin.as_object() {
        entries.extend(
            map.iter().filter_map(|(command, target)| {
                Some((command.clone(), target.as_str()?.to_string()))
            }),
        );
    }
    entries
        .into_iter()
        .map(|(command, target)| {
            let resolved = repo::package_public_target_candidates(package_path, &target)
                .into_iter()
                .find(|candidate| project.files.contains_key(candidate));
            cli_entrypoint_surface(manifest, &command, &target, resolved, "package_json_bin")
        })
        .collect()
}

fn cargo_manifest_cli_entrypoints(
    project: &Project,
    manifest: &str,
    package_path: &str,
    package_name: &str,
) -> Vec<Surface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(manifest)) else {
        return Vec::new();
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut explicit_bins = BTreeSet::new();
    if let Some(bins) = value.get("bin").and_then(|value| value.as_array()) {
        for bin in bins {
            let Some(command) = bin.get("name").and_then(|value| value.as_str()) else {
                continue;
            };
            let Some(target) = bin.get("path").and_then(|value| value.as_str()) else {
                continue;
            };
            let resolved = repo::package_target_path(package_path, target)
                .filter(|candidate| project.files.contains_key(candidate));
            explicit_bins.insert((command.to_string(), resolved.clone()));
            out.push(cli_entrypoint_surface(
                manifest,
                command,
                target,
                resolved,
                "cargo_bin_target",
            ));
        }
    }
    let default_target = repo::package_target_path(package_path, "src/main.rs");
    if let Some(path) = default_target.filter(|candidate| project.files.contains_key(candidate)) {
        if explicit_bins.contains(&(package_name.to_string(), Some(path.clone()))) {
            return out;
        }
        out.push(cli_entrypoint_surface(
            manifest,
            package_name,
            "src/main.rs",
            Some(path),
            "cargo_default_bin_convention",
        ));
    }
    out
}

fn pyproject_manifest_cli_entrypoints(
    project: &Project,
    manifest: &str,
    package_path: &str,
) -> Vec<Surface> {
    let Ok(text) = std::fs::read_to_string(project.root.join(manifest)) else {
        return Vec::new();
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    collect_toml_script_table(
        &value,
        &["project", "scripts"],
        "pyproject_project_scripts",
        &mut entries,
    );
    collect_toml_script_table(
        &value,
        &["project", "gui-scripts"],
        "pyproject_project_gui_scripts",
        &mut entries,
    );
    collect_toml_script_table(
        &value,
        &["tool", "poetry", "scripts"],
        "pyproject_poetry_scripts",
        &mut entries,
    );
    entries
        .into_iter()
        .map(|(command, target, evidence)| {
            let resolved = python_entrypoint_target(project, package_path, &target);
            cli_entrypoint_surface(manifest, &command, &target, resolved, evidence)
        })
        .collect()
}

fn collect_toml_script_table(
    value: &toml::Value,
    path: &[&str],
    evidence: &'static str,
    out: &mut Vec<(String, String, &'static str)>,
) {
    let Some(table) = toml_path(value, path).and_then(|value| value.as_table()) else {
        return;
    };
    out.extend(table.iter().filter_map(|(command, target)| {
        Some((command.clone(), target.as_str()?.to_string(), evidence))
    }));
}

fn toml_path<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    path.iter()
        .try_fold(value, |current, segment| current.get(*segment))
}

fn python_entrypoint_target(project: &Project, package_path: &str, target: &str) -> Option<String> {
    let module = target.split(':').next()?.trim();
    if module.is_empty() {
        return None;
    }
    let rel = module.replace('.', "/");
    [
        format!("{rel}.py"),
        format!("{rel}/__init__.py"),
        format!("src/{rel}.py"),
        format!("src/{rel}/__init__.py"),
    ]
    .into_iter()
    .filter_map(|candidate| repo::package_target_path(package_path, &candidate))
    .find(|candidate| project.files.contains_key(candidate))
}

fn manifest_parent(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .map(|parent| repo::normalize_rel_path(&parent.to_string_lossy()))
        .filter(|parent| !parent.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn package_name_from_manifest_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn cli_entrypoint_surface(
    manifest: &str,
    command: &str,
    target: &str,
    resolved: Option<String>,
    evidence: &str,
) -> Surface {
    let display_target = resolved.clone().unwrap_or_else(|| target.to_string());
    Surface {
        id: format!("surface:cli_entrypoint:{manifest}:{command}"),
        kind: "cli_entrypoint".to_string(),
        path: Some(resolved.unwrap_or_else(|| manifest.to_string())),
        role: Some("runtime_entrypoint".to_string()),
        evidence: evidence.to_string(),
        strength: EvidenceStrength::Hard,
        count: Some(1),
        examples: vec![format!("{command} -> {display_target}")],
        hidden_count: 0,
    }
}
