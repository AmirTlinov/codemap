// Responsibility: repo-packages-scripts
use crate::model::ScriptInfo;
use crate::repo::{justfile_scripts, makefile_scripts};
use std::fs;
use std::path::Path;

pub(crate) fn detect_package_manager(root: &Path) -> String {
    if root.join("pnpm-lock.yaml").exists() || root.join("pnpm-workspace.yaml").exists() {
        "pnpm"
    } else if root.join("yarn.lock").exists() {
        "yarn"
    } else if root.join("bun.lockb").exists() {
        "bun"
    } else if root.join("package.json").exists() {
        "npm"
    } else if root.join("Cargo.toml").exists() {
        "cargo"
    } else if root.join("go.mod").exists() || root.join("go.work").exists() {
        "go"
    } else if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        "python"
    } else if root.join("Package.swift").exists() {
        "swift"
    } else {
        "unknown"
    }
    .to_string()
}

pub(crate) fn detect_scripts(root: &Path) -> Vec<ScriptInfo> {
    let mut scripts = Vec::new();
    if root.join("package.json").exists() {
        let pm = detect_package_manager(root);
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
    if root.join("Cargo.toml").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "cargo test".to_string(),
            reason: "Cargo.toml detected".to_string(),
            path: Some("Cargo.toml".to_string()),
            line_start: Some(1),
        });
    }
    if root.join("go.mod").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "go test ./...".to_string(),
            reason: "go.mod detected".to_string(),
            path: Some("go.mod".to_string()),
            line_start: Some(1),
        });
    }
    if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "pytest".to_string(),
            reason: "Python project files detected".to_string(),
            path: ["pyproject.toml", "requirements.txt"]
                .into_iter()
                .find(|path| root.join(path).exists())
                .map(str::to_string),
            line_start: Some(1),
        });
    }
    if root.join("Package.swift").exists() {
        scripts.push(ScriptInfo {
            name: "test".to_string(),
            command: "swift test".to_string(),
            reason: "Package.swift detected".to_string(),
            path: Some("Package.swift".to_string()),
            line_start: Some(1),
        });
    }
    scripts.extend(makefile_scripts(root));
    scripts.extend(justfile_scripts(root));
    scripts.sort_by(|a, b| a.command.cmp(&b.command));
    scripts.dedup_by(|a, b| a.command == b.command);
    scripts
}

pub(crate) fn json_key_line(text: &str, key: &str) -> Option<usize> {
    let quoted = format!("\"{key}\"");
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(&quoted))
        .map(|(index, _)| index + 1)
}
