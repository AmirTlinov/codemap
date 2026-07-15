// Responsibility: repository-command-target-resolution
use crate::map::{
    clean_path_token, command_invokes_script, command_invokes_script_surface, command_tokens,
    package_json_scripts, runtime_manifest_entrypoints, script_target_for_package,
    script_target_for_path, shell_words,
};
use crate::model::Project;
use crate::repo;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn repo_script_targets(project: &Project, command: &str) -> Vec<String> {
    let mut targets = BTreeSet::new();
    for token in shell_words(command) {
        let path = clean_path_token(&token);
        if path.contains('$') || path.starts_with('-') {
            continue;
        }
        let path = path.strip_prefix("./").unwrap_or(&path);
        let normalized = repo::normalize_rel_path(path);
        if project.files.contains_key(&normalized) && executable_script_path(&normalized) {
            targets.insert(normalized);
        }
    }
    for script in &project.scripts {
        if command_invokes_script_surface(command, script) {
            targets.insert(script_target_for_path(
                script.path.as_deref().unwrap_or("script"),
                &script.name,
            ));
        }
    }
    for package in project
        .packages
        .iter()
        .filter(|package| package.ecosystem == "javascript")
    {
        for (name, _body, _line) in package_json_scripts(project, &package.manifest) {
            if command_invokes_script(command, &name) {
                targets.insert(script_target_for_package(package, &name));
            }
        }
    }
    resolve_cargo_bin_target(project, command, &mut targets);
    targets.into_iter().collect()
}

pub(crate) fn package_script_body_for_target(
    project: &Project,
    target: &str,
) -> Option<(String, String, usize)> {
    for package in project
        .packages
        .iter()
        .filter(|package| package.ecosystem == "javascript")
    {
        for (name, body, line) in package_json_scripts(project, &package.manifest) {
            if script_target_for_package(package, &name) == target {
                return Some((package.manifest.clone(), body, line));
            }
        }
    }
    None
}

fn resolve_cargo_bin_target(project: &Project, command: &str, targets: &mut BTreeSet<String>) {
    let Some(target) = cargo_bin_target(command) else {
        return;
    };
    for package in &project.packages {
        let Some(manifest) = project.files.get(&package.manifest) else {
            continue;
        };
        for surface in runtime_manifest_entrypoints(project, manifest) {
            if matches!(
                surface.evidence.as_str(),
                "cargo_bin_target" | "cargo_default_bin_convention"
            ) && surface.id.ends_with(&format!(":{target}"))
                && let Some(path) = surface.path
                && path != package.manifest
            {
                targets.insert(path);
            }
        }
    }
}

fn cargo_bin_target(command: &str) -> Option<String> {
    let tokens = command_tokens(command);
    let cargo = tokens.iter().position(|token| token == "cargo")?;
    let run = tokens[cargo + 1..]
        .iter()
        .position(|token| token == "run")?
        + cargo
        + 1;
    let bin = tokens[run + 1..]
        .iter()
        .position(|token| token == "--bin")?
        + run
        + 1;
    tokens.get(bin + 1).cloned()
}

fn executable_script_path(path: &str) -> bool {
    matches!(
        Path::new(path).extension().and_then(|ext| ext.to_str()),
        Some("sh" | "bash" | "zsh" | "js" | "mjs" | "cjs" | "ts" | "mts" | "cts" | "py" | "rb")
    )
}
