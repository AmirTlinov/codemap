// Responsibility: repo-packages-scripts
use crate::model::{FileInfo, ScriptInfo};
use crate::repo::{justfile_scripts, makefile_scripts};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) fn detect_package_manager(files: &BTreeMap<String, FileInfo>) -> String {
    if indexed_present(files, "pnpm-lock.yaml") || indexed_present(files, "pnpm-workspace.yaml") {
        "pnpm"
    } else if indexed_present(files, "yarn.lock") {
        "yarn"
    } else if indexed_present(files, "bun.lockb") {
        "bun"
    } else if indexed_present(files, "package.json") {
        "npm"
    } else if indexed_present(files, "Cargo.toml") {
        "cargo"
    } else if indexed_present(files, "go.mod") || indexed_present(files, "go.work") {
        "go"
    } else if indexed_present(files, "pyproject.toml") || indexed_present(files, "requirements.txt")
    {
        "python"
    } else if indexed_present(files, "Package.swift") {
        "swift"
    } else {
        "unknown"
    }
    .to_string()
}

pub(crate) fn detect_scripts(root: &Path, files: &BTreeMap<String, FileInfo>) -> Vec<ScriptInfo> {
    let mut scripts = Vec::new();
    if indexed_readable(files, "package.json") {
        let pm = detect_package_manager(files);
        if let Ok(text) = fs::read_to_string(root.join("package.json"))
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(map) = value.get("scripts").and_then(|v| v.as_object())
        {
            for (name, command) in map {
                let name_l = name.to_ascii_lowercase();
                if !(name_l.contains("test")
                    || name_l.contains("type")
                    || name_l.contains("lint")
                    || name_l.contains("check"))
                {
                    continue;
                }
                let runner = if pm == "unknown" { "npm" } else { &pm };
                let invoke = if name == "test" {
                    match runner {
                        "npm" => "npm test".to_string(),
                        "yarn" => "yarn test".to_string(),
                        "bun" => "bun test".to_string(),
                        _ => format!("{runner} test"),
                    }
                } else {
                    format!("{runner} run {name}")
                };
                scripts.push(ScriptInfo {
                    name: name.clone(),
                    command: invoke,
                    reason: format!("package.json script: {}", command.as_str().unwrap_or("")),
                    path: Some("package.json".to_string()),
                    line_start: json_key_line(&text, name),
                });
            }
        }
    }
    if indexed_readable(files, "Cargo.toml") {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "cargo test".to_string(),
            reason: "Cargo.toml detected".to_string(),
            path: Some("Cargo.toml".to_string()),
            line_start: Some(1),
        });
    }
    if indexed_readable(files, "go.mod") {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "go test ./...".to_string(),
            reason: "go.mod detected".to_string(),
            path: Some("go.mod".to_string()),
            line_start: Some(1),
        });
    }
    if indexed_readable(files, "pyproject.toml") || indexed_readable(files, "requirements.txt") {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "pytest".to_string(),
            reason: "Python project files detected".to_string(),
            path: ["pyproject.toml", "requirements.txt"]
                .into_iter()
                .find(|path| indexed_readable(files, path))
                .map(str::to_string),
            line_start: Some(1),
        });
    }
    if indexed_readable(files, "Package.swift") {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "swift test".to_string(),
            reason: "Package.swift detected".to_string(),
            path: Some("Package.swift".to_string()),
            line_start: Some(1),
        });
    }
    scripts.extend(makefile_scripts(root, files));
    scripts.extend(justfile_scripts(root, files));
    scripts.sort_by(|a, b| a.command.cmp(&b.command));
    scripts.dedup_by(|a, b| a.command == b.command);
    scripts
}

fn indexed_readable(files: &BTreeMap<String, FileInfo>, path: &str) -> bool {
    files
        .get(path)
        .is_some_and(|file| file.content_hash.is_some())
}

fn indexed_present(files: &BTreeMap<String, FileInfo>, path: &str) -> bool {
    files.contains_key(path)
}

pub(crate) fn json_key_line(text: &str, key: &str) -> Option<usize> {
    let quoted = format!("\"{key}\"");
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(&quoted))
        .map(|(index, _)| index + 1)
}
