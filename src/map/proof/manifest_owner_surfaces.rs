// Responsibility: map-proof-manifest-owner-surfaces
use crate::map::{
    ci_run_reference_proof_surfaces, command_invokes_script, command_target, command_tokens,
    first_line_containing, manifest_script_is_proof_relevant, package_json_scripts,
    package_script_command, shell_quote, structural_edge_with_locations,
};
use crate::model::{
    EvidenceLocation, EvidenceStrength, FileInfo, Project, ProofSurface, StructuralEdge,
};
use crate::repo;
use std::path::Path;

#[derive(Debug, Clone)]
struct WorkspacePattern {
    pattern: String,
    line: usize,
    excluded: bool,
}

pub(crate) fn cargo_manifest_builtin_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    if manifest_file_name(&file.rel) != "Cargo.toml" {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    if !text.contains("[package]") && !text.contains("[workspace]") {
        return Vec::new();
    }
    let line =
        first_line_containing(project, &file.rel, &["[package]", "[workspace]"]).unwrap_or(1);
    let prefix = manifest_command_prefix(&file.rel);
    ["cargo test", "cargo check", "cargo build"]
        .into_iter()
        .map(|command| ProofSurface {
            command: Some(format!("{prefix}{command}")),
            path: Some(file.rel.clone()),
            target_anchor: Some(file.rel.clone()),
            evidence: "cargo_manifest_command".to_string(),
            strength: EvidenceStrength::Hard,
            reason: format!("Cargo manifest gives package-local `{command}` surface"),
            locations: vec![EvidenceLocation::line(&file.rel, line, "cargo_manifest")],
        })
        .collect()
}

pub(crate) fn swift_manifest_builtin_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    if manifest_file_name(&file.rel) != "Package.swift" {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    if !text.contains("Package(") {
        return Vec::new();
    }
    let line = first_line_containing(project, &file.rel, &["Package("]).unwrap_or(1);
    let prefix = manifest_command_prefix(&file.rel);
    ["swift test", "swift build"]
        .into_iter()
        .map(|command| ProofSurface {
            command: Some(format!("{prefix}{command}")),
            path: Some(file.rel.clone()),
            target_anchor: Some(file.rel.clone()),
            evidence: "swift_package_command".to_string(),
            strength: EvidenceStrength::Hard,
            reason: format!("Swift package manifest gives package-local `{command}` surface"),
            locations: vec![EvidenceLocation::line(
                &file.rel,
                line,
                "swift_package_manifest",
            )],
        })
        .collect()
}

pub(crate) fn workspace_manifest_script_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    let Some(root_manifest) = workspace_root_package_manifest(project, &file.rel) else {
        return Vec::new();
    };
    let Some(root_package) = project
        .packages
        .iter()
        .find(|package| package.manifest == root_manifest)
    else {
        return Vec::new();
    };
    package_json_scripts(project, &root_manifest)
        .into_iter()
        .filter(|(name, command, _)| workspace_script_is_proof_relevant(name, command))
        .map(|(name, command, line)| ProofSurface {
            command: package_script_command(project, root_package, &name),
            path: Some(root_manifest.clone()),
            target_anchor: Some(file.rel.clone()),
            evidence: "workspace_manifest_script".to_string(),
            strength: EvidenceStrength::Hard,
            reason: format!("workspace root script `{name}` is tied to {command}"),
            locations: vec![EvidenceLocation::line(
                &root_manifest,
                line,
                "workspace_script",
            )],
        })
        .collect()
}

pub(crate) fn workspace_manifest_ci_reference_proof_surfaces(
    project: &Project,
    file: &FileInfo,
) -> Vec<ProofSurface> {
    ci_run_reference_proof_surfaces(
        project,
        file,
        "workspace_manifest_ci_reference",
        |command| workspace_ci_run_match_reason(project, &file.rel, command),
    )
}

pub(crate) fn owner_workspace_manifest_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for pattern in pnpm_workspace_patterns(project, rel) {
        let target = if pattern.excluded {
            format!("workspace_exclude:{}", pattern.pattern)
        } else {
            format!("workspace_pattern:{}", pattern.pattern)
        };
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            target,
            "declares_workspace_pattern",
            "pnpm_workspace_manifest",
            EvidenceStrength::Hard,
            vec![EvidenceLocation::line(
                rel,
                pattern.line,
                "workspace_pattern",
            )],
        ));
    }
    for (package, line) in workspace_manifest_member_packages(project, rel) {
        edges.push(structural_edge_with_locations(
            rel.to_string(),
            package.manifest.clone(),
            "workspace_member",
            "pnpm_workspace_pattern",
            EvidenceStrength::Hard,
            vec![
                EvidenceLocation::line(rel, line, "workspace_pattern"),
                EvidenceLocation::line(
                    &package.manifest,
                    first_line_containing(project, &package.manifest, &["\"name\""]).unwrap_or(1),
                    "package_manifest",
                ),
            ],
        ));
    }
    if let Some(root_manifest) = workspace_root_package_manifest(project, rel) {
        for (name, command, line) in package_json_scripts(project, &root_manifest)
            .into_iter()
            .filter(|(name, command, _)| workspace_script_is_proof_relevant(name, command))
        {
            edges.push(structural_edge_with_locations(
                rel.to_string(),
                format!("script:{name}"),
                "workspace_script",
                "workspace_root_package_script",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(
                    &root_manifest,
                    line,
                    "package_script",
                )],
            ));
            edges.push(structural_edge_with_locations(
                format!("script:{name}"),
                command_target(&command),
                "runs_command",
                "workspace_root_package_script",
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(
                    &root_manifest,
                    line,
                    "package_script",
                )],
            ));
        }
    }
    edges
}

pub(crate) fn workspace_manifest_file(rel: &str) -> bool {
    matches!(
        manifest_file_name(rel),
        "pnpm-workspace.yaml" | "pnpm-workspace.yml"
    )
}

fn workspace_root_package_manifest(project: &Project, rel: &str) -> Option<String> {
    if !workspace_manifest_file(rel) {
        return None;
    }
    let manifest = manifest_sibling_path(rel, "package.json");
    project.files.contains_key(&manifest).then_some(manifest)
}

fn pnpm_workspace_patterns(project: &Project, rel: &str) -> Vec<WorkspacePattern> {
    if !workspace_manifest_file(rel) {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut in_packages = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("packages:") {
            in_packages = true;
            continue;
        }
        if !in_packages {
            continue;
        }
        if !trimmed.starts_with("- ") && !line.starts_with(' ') && !line.starts_with('\t') {
            in_packages = false;
            continue;
        }
        let Some(value) = trimmed.strip_prefix("- ") else {
            continue;
        };
        let pattern = unquote_yaml_scalar(value.trim());
        if pattern.is_empty() {
            continue;
        }
        let excluded = pattern.starts_with('!');
        out.push(WorkspacePattern {
            pattern: repo::normalize_rel_path(
                pattern.trim_start_matches('!').trim_start_matches("./"),
            ),
            line: index + 1,
            excluded,
        });
    }
    out
}

pub(crate) fn workspace_manifest_member_packages<'a>(
    project: &'a Project,
    rel: &str,
) -> Vec<(&'a crate::model::PackageInfo, usize)> {
    let patterns = pnpm_workspace_patterns(project, rel);
    let workspace_root = manifest_dir_for_rel(rel);
    let root_manifest = workspace_root_package_manifest(project, rel);
    let mut out = Vec::new();
    for package in &project.packages {
        if package.ecosystem != "javascript" || root_manifest.as_deref() == Some(&package.manifest)
        {
            continue;
        }
        let Some(line) = patterns
            .iter()
            .filter(|pattern| !pattern.excluded)
            .find(|pattern| {
                workspace_pattern_matches_package(&workspace_root, &pattern.pattern, &package.path)
            })
            .map(|pattern| pattern.line)
        else {
            continue;
        };
        let excluded = patterns
            .iter()
            .filter(|pattern| pattern.excluded)
            .any(|pattern| {
                workspace_pattern_matches_package(&workspace_root, &pattern.pattern, &package.path)
            });
        if !excluded {
            out.push((package, line));
        }
    }
    out
}

fn workspace_script_is_proof_relevant(name: &str, command: &str) -> bool {
    manifest_script_is_proof_relevant(name, command)
        || name.to_ascii_lowercase().starts_with("verify:")
}

fn workspace_ci_run_match_reason(project: &Project, rel: &str, command: &str) -> Option<String> {
    if command_mentions_workspace_tooling(command) {
        return Some("CI run step uses pnpm/turbo workspace tooling".to_string());
    }
    let command_lower = command.to_ascii_lowercase();
    let root_manifest = workspace_root_package_manifest(project, rel)?;
    package_json_scripts(project, &root_manifest)
        .into_iter()
        .find(|(name, script_command, _)| {
            workspace_script_is_proof_relevant(name, script_command)
                && command_invokes_script(&command_lower, &name.to_ascii_lowercase())
        })
        .map(|(name, _, _)| format!("CI run step invokes workspace root script `{name}`"))
}

fn command_mentions_workspace_tooling(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens
        .iter()
        .any(|token| matches!(token.as_str(), "turbo" | "pnpm"))
        && (tokens.iter().any(|token| {
            matches!(
                token.as_str(),
                "install" | "-r" | "--recursive" | "--filter" | "-F" | "--workspace-root" | "-w"
            )
        }) || tokens.iter().any(|token| token == "turbo"))
}

fn workspace_pattern_matches_package(
    workspace_root: &str,
    pattern: &str,
    package_path: &str,
) -> bool {
    let pattern_path = workspace_pattern_repo_path(workspace_root, pattern);
    if let Some(base) = pattern_path.strip_suffix("/*") {
        let Some(rest) = package_path.strip_prefix(&format!("{}/", base.trim_end_matches('/')))
        else {
            return false;
        };
        return !rest.is_empty() && !rest.contains('/');
    }
    package_path == pattern_path
}

fn workspace_pattern_repo_path(workspace_root: &str, pattern: &str) -> String {
    if workspace_root == "." {
        repo::normalize_rel_path(pattern)
    } else {
        repo::normalize_rel_path(&format!("{workspace_root}/{pattern}"))
    }
}

fn manifest_sibling_path(rel: &str, name: &str) -> String {
    let dir = manifest_dir_for_rel(rel);
    if dir == "." {
        name.to_string()
    } else {
        repo::normalize_rel_path(&format!("{dir}/{name}"))
    }
}

pub(crate) fn manifest_dir_for_rel(rel: &str) -> String {
    Path::new(rel)
        .parent()
        .and_then(|parent| parent.to_str())
        .map(repo::normalize_rel_path)
        .unwrap_or_else(|| ".".to_string())
}

pub(crate) fn manifest_file_name(rel: &str) -> &str {
    Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(rel)
}

pub(crate) fn manifest_command_prefix(rel: &str) -> String {
    let path = manifest_dir_for_rel(rel);
    if path == "." {
        String::new()
    } else {
        format!("cd {} && ", shell_quote(&path))
    }
}

fn unquote_yaml_scalar(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.as_bytes()[0] as char;
        let last = value.as_bytes()[value.len() - 1] as char;
        if matches!(first, '"' | '\'') && first == last {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}
