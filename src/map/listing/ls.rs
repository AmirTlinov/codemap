// Responsibility: map-listing-ls
mod anchor_reports;
mod surface_meta;

pub(crate) use anchor_reports::*;
pub(crate) use surface_meta::*;

use crate::map::{
    balanced_edge_prefix_by_source, boundary_facts_for_ls, direct_files_under_directory,
    directory_edges, directory_role_surface, file_kind_for_ls, files_under_directory,
    immediate_child_dirs, is_generic_noise, is_support_artifact_path, path_under_scope,
    shell_quote, surface_priority,
};
use crate::model::{DirectorySurface, HiddenGroup, LsReport, Project};
use std::collections::BTreeMap;

pub(crate) fn ls_directory_report(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    limit: usize,
) -> LsReport {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for domain in &project.domains {
        if path_under_scope(&domain.path, rel) {
            grouped
                .entry("domain".to_string())
                .or_default()
                .push(domain.path.clone());
        }
    }
    for package in &project.packages {
        if path_under_scope(&package.path, rel) || path_under_scope(&package.manifest, rel) {
            let package_is_support = is_support_artifact_path(&package.path)
                || is_support_artifact_path(&package.manifest);
            let scope_is_support = is_support_artifact_path(rel);
            if package_is_support && !scope_is_support && !include_hidden {
                grouped
                    .entry("support_package_hidden".to_string())
                    .or_default()
                    .push(package.manifest.clone());
                continue;
            }
            let kind = if package_is_support && !scope_is_support {
                format!("support_package:{}", package.ecosystem)
            } else {
                format!("package:{}", package.ecosystem)
            };
            grouped
                .entry(kind)
                .or_default()
                .push(package.manifest.clone());
        }
    }
    for script in &project.scripts {
        if rel == "." {
            grouped
                .entry("script".to_string())
                .or_default()
                .push(format!("{}: {}", script.name, script.command));
        }
    }
    let direct_files = direct_files_under_directory(project, rel);
    let scope_is_support = is_support_artifact_path(rel);
    let mut hidden_support_artifact_count = 0;
    for dir in immediate_child_dirs(project, rel) {
        if is_support_artifact_path(&dir) && !scope_is_support && !include_hidden {
            hidden_support_artifact_count += 1;
            continue;
        }
        if let Some(kind) = directory_role_surface(project, &dir) {
            grouped.entry(kind).or_default().push(dir.clone());
        }
        grouped.entry("dir".to_string()).or_default().push(dir);
    }
    for file in &direct_files {
        let kind = file_kind_for_ls(file);
        let noisy = is_generic_noise(file);
        if noisy && !include_hidden {
            grouped
                .entry("generic_hidden".to_string())
                .or_default()
                .push(file.rel.clone());
            continue;
        }
        grouped.entry(kind).or_default().push(file.rel.clone());
    }
    let recursive_files = files_under_directory(project, rel)
        .into_iter()
        .filter(|file| !direct_files.iter().any(|direct| direct.rel == file.rel))
        .collect::<Vec<_>>();
    if include_hidden {
        for file in &recursive_files {
            let kind = format!("recursive:{}", file_kind_for_ls(file));
            grouped.entry(kind).or_default().push(file.rel.clone());
        }
    }
    let hidden_generic_count = grouped
        .remove("generic_hidden")
        .map(|v| v.len())
        .unwrap_or(0);
    let hidden_support_package_count = grouped
        .remove("support_package_hidden")
        .map(|v| v.len())
        .unwrap_or(0);
    let mut surfaces = grouped
        .into_iter()
        .map(|(kind, mut files)| {
            files.sort();
            let count = files.len();
            let examples = files.into_iter().take(5).collect::<Vec<_>>();
            DirectorySurface {
                id: directory_surface_id(rel, &kind, &examples),
                path: directory_surface_path(&examples),
                role: directory_surface_role(&kind),
                evidence: directory_surface_evidence(&kind),
                strength: directory_surface_strength(&kind),
                kind,
                count,
                examples,
                hidden_count: count.saturating_sub(5),
            }
        })
        .collect::<Vec<_>>();
    surfaces.sort_by(|a, b| {
        surface_priority(&a.kind)
            .cmp(&surface_priority(&b.kind))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    let surface_count = surfaces.len();
    surfaces.truncate(limit);
    let mut hidden = Vec::new();
    let mut edges = directory_edges(project, rel, include_hidden);
    let edge_count = edges.len();
    if !include_hidden {
        edges = balanced_edge_prefix_by_source(&edges, limit);
    }
    if edge_count > edges.len() {
        hidden.push(HiddenGroup {
            reason: "directory edges hidden by limit".to_string(),
            count: edge_count - edges.len(),
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if surface_count > surfaces.len() {
        hidden.push(HiddenGroup {
            reason: "directory surfaces hidden by limit".to_string(),
            count: surface_count - surfaces.len(),
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if hidden_generic_count > 0 {
        hidden.push(HiddenGroup {
            reason: "generic source files hidden".to_string(),
            count: hidden_generic_count,
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if hidden_support_package_count > 0 {
        hidden.push(HiddenGroup {
            reason: "support packages hidden below support scopes".to_string(),
            count: hidden_support_package_count,
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if hidden_support_artifact_count > 0 {
        hidden.push(HiddenGroup {
            reason: "support artifacts hidden".to_string(),
            count: hidden_support_artifact_count,
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if !include_hidden && !recursive_files.is_empty() {
        hidden.push(HiddenGroup {
            reason: "recursive files below this level hidden".to_string(),
            count: recursive_files.len(),
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    LsReport {
        kind: "ls_report",
        schema_version: "5",
        path: rel.to_string(),
        mode: "directory".to_string(),
        anchor: None,
        directory: surfaces,
        boundary_facts: boundary_facts_for_ls(project, rel),
        edges,
        hidden,
        next: directory_next_commands(rel),
    }
}
