// Responsibility: map-proof-owner-surfaces-package-scripts
use crate::map::{javascript_runner_for_package, javascript_test_command, shell_quote};
use crate::model::Project;

pub(crate) fn package_json_scripts(
    project: &Project,
    manifest: &str,
) -> Vec<(String, String, usize)> {
    let Some(text) = project.read_indexed_text(manifest) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let Some(scripts) = value.get("scripts").and_then(|scripts| scripts.as_object()) else {
        return Vec::new();
    };
    let mut out = scripts
        .iter()
        .filter_map(|(name, value)| {
            let command = value.as_str()?.trim();
            if command.is_empty() {
                return None;
            }
            Some((
                name.clone(),
                command.to_string(),
                json_key_line(&text, name).unwrap_or(1),
            ))
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

pub(crate) fn package_script_command(
    project: &Project,
    package: &crate::model::PackageInfo,
    script_name: &str,
) -> Option<String> {
    match package.ecosystem.as_str() {
        "javascript" => {
            let runner = javascript_runner_for_package(project, package);
            let command = javascript_script_command(&runner, script_name);
            Some(if package.path == "." {
                command
            } else {
                format!("cd {} && {command}", shell_quote(&package.path))
            })
        }
        _ => None,
    }
}

pub(crate) fn javascript_script_command(runner: &str, script_name: &str) -> String {
    if script_name == "test" {
        return javascript_test_command(runner);
    }
    match runner {
        "npm" => format!("npm run {}", shell_quote(script_name)),
        "yarn" => format!("yarn {}", shell_quote(script_name)),
        "bun" => format!("bun run {}", shell_quote(script_name)),
        _ => format!("pnpm run {}", shell_quote(script_name)),
    }
}

pub(crate) fn json_key_line(text: &str, key: &str) -> Option<usize> {
    let quoted = format!("\"{key}\"");
    text.lines()
        .enumerate()
        .find(|(_, line)| line.contains(&quoted))
        .map(|(index, _)| index + 1)
}

pub(crate) fn first_line_containing(
    project: &Project,
    rel: &str,
    needles: &[&str],
) -> Option<usize> {
    let text = project.read_indexed_text(rel)?;
    text.lines()
        .enumerate()
        .find(|(_, line)| needles.iter().any(|needle| line.contains(needle)))
        .map(|(index, _)| index + 1)
}
