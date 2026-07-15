// Responsibility: repo-packages-workspace-members
use crate::model::FileInfo;
use crate::repo::{
    cargo_workspace_array_values, normalize_rel_path, parse_toml_value, should_ignore_rel,
    toml_string_array, unique_strings, unquote,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn workspace_domain_paths(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for pattern in workspace_patterns(root, files) {
        expand_workspace_pattern(root, files, &pattern, &mut out);
    }
    out
}

fn workspace_patterns(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<String> {
    let mut patterns = Vec::new();
    if indexed_readable(files, "package.json")
        && let Ok(text) = fs::read_to_string(root.join("package.json"))
        && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(workspaces) = value.get("workspaces")
    {
        if let Some(array) = workspaces.as_array() {
            patterns.extend(
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string)),
            );
        } else if let Some(array) = workspaces.get("packages").and_then(|v| v.as_array()) {
            patterns.extend(
                array
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string)),
            );
        }
    }
    if indexed_readable(files, "pnpm-workspace.yaml")
        && let Ok(text) = fs::read_to_string(root.join("pnpm-workspace.yaml"))
    {
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(value) = trimmed.strip_prefix("- ") {
                patterns.push(unquote(value.trim()).unwrap_or_else(|| value.trim().to_string()));
            }
        }
    }
    if indexed_readable(files, "Cargo.toml")
        && let Ok(text) = fs::read_to_string(root.join("Cargo.toml"))
    {
        patterns.extend(cargo_workspace_array_values(&text, "members"));
    }
    if indexed_readable(files, "go.work")
        && let Ok(text) = fs::read_to_string(root.join("go.work"))
    {
        patterns.extend(go_work_uses(&text));
    }
    if indexed_readable(files, "pyproject.toml")
        && let Ok(text) = fs::read_to_string(root.join("pyproject.toml"))
    {
        patterns.extend(pyproject_workspace_patterns(&text));
    }
    patterns
        .into_iter()
        .map(|pattern| normalize_rel_path(pattern.trim().trim_start_matches("./")))
        .filter(|pattern| !pattern.is_empty() && pattern != ".")
        .collect()
}

fn indexed_readable(files: &BTreeMap<String, FileInfo>, path: &str) -> bool {
    files
        .get(path)
        .is_some_and(|file| file.content_hash.is_some())
}

fn expand_workspace_pattern(
    root: &Path,
    files: &BTreeMap<String, FileInfo>,
    pattern: &str,
    out: &mut BTreeSet<String>,
) {
    if pattern.starts_with('!') || pattern.contains("**") || pattern.contains('{') {
        return;
    }
    if let Some(base) = pattern.strip_suffix("/*") {
        let base = normalize_rel_path(base);
        let Ok(children) = fs::read_dir(root.join(&base)) else {
            return;
        };
        for child in children.flatten() {
            let child_path = child.path();
            if child_path.is_dir() {
                let rel = normalize_rel_path(
                    &child_path
                        .strip_prefix(root)
                        .unwrap_or(&child_path)
                        .to_string_lossy(),
                );
                if workspace_path_has_project(root, files, &rel) {
                    out.insert(rel);
                }
            }
        }
        return;
    }
    if !pattern.contains('*') && workspace_path_has_project(root, files, pattern) {
        out.insert(normalize_rel_path(pattern));
    }
}

fn workspace_path_has_project(root: &Path, files: &BTreeMap<String, FileInfo>, rel: &str) -> bool {
    let rel = normalize_rel_path(rel);
    if !root.join(&rel).is_dir() || should_ignore_rel(&rel) {
        return false;
    }
    let prefix = format!("{}/", rel.trim_end_matches('/'));
    files.keys().any(|file| file.starts_with(&prefix))
        || [
            "package.json",
            "Cargo.toml",
            "go.mod",
            "pyproject.toml",
            "src",
        ]
        .iter()
        .any(|marker| root.join(&rel).join(marker).exists())
}

pub(crate) fn pyproject_workspace_patterns(text: &str) -> Vec<String> {
    let Some(value) = parse_toml_value(text) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in ["members", "packages"] {
        if let Some(values) = value.get(key).and_then(toml_string_array) {
            out.extend(values);
        }
        if let Some(values) = value
            .get("project")
            .and_then(|project| project.get(key))
            .and_then(toml_string_array)
        {
            out.extend(values);
        }
    }
    if let Some(values) = value
        .get("tool")
        .and_then(|tool| tool.get("uv"))
        .and_then(|uv| uv.get("workspace"))
        .and_then(|workspace| workspace.get("members"))
        .and_then(toml_string_array)
    {
        out.extend(values);
    }
    unique_strings(out)
}

fn go_work_uses(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("use (") {
            in_block = true;
            continue;
        }
        if in_block {
            if trimmed.starts_with(')') {
                in_block = false;
            } else if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("use ") {
            out.push(value.trim().to_string());
        }
    }
    out
}
