// Responsibility: map-listing-ls
mod anchor_reports;
mod root_horizons;
mod surface_meta;

pub(crate) use anchor_reports::*;
pub(crate) use root_horizons::*;
pub(crate) use surface_meta::*;

use crate::map::{
    ObservationProjection, balanced_edge_prefix_by_source, boundary_facts_for_ls,
    direct_files_under_directory, directory_edges, directory_relation_observation,
    directory_role_surface, file_kind_for_ls, files_under_directory, immediate_child_dirs,
    inventory_recursive_structural_kind, is_generic_noise, is_support_artifact_path,
    path_under_scope, shell_quote, surface_priority,
};
use crate::model::{DirectorySurface, FileInfo, HiddenGroup, LsReport, Project};
use std::collections::BTreeMap;

pub(crate) fn ls_directory_report(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    limit: usize,
    complete_directory_relations: bool,
) -> LsReport {
    // Root observation truth is assembled from the complete current scope and
    // is intentionally independent of the readable projection. This makes a
    // limit a display choice rather than a different candidate universe.
    let complete_root = (rel == ".").then(|| directory_grouping(project, rel, true));
    let complete_nested_edges = (rel != ".").then(|| directory_edges(project, rel, true));
    let projected = directory_grouping(project, rel, include_hidden);
    let DirectoryGrouping {
        grouped,
        direct_files: _,
        recursive_files,
        hidden_generic_count,
        hidden_support_package_count,
        hidden_support_artifact_count,
        child_dir_count: _,
    } = projected;

    let mut surfaces = directory_surfaces(project, rel, grouped, include_hidden);
    let surface_count = surfaces.len();
    surfaces.truncate(limit);

    let mut hidden = Vec::new();
    let mut edges = if complete_directory_relations {
        complete_nested_edges.clone().unwrap_or_default()
    } else {
        directory_edges(project, rel, include_hidden)
    };
    let edge_count = complete_nested_edges
        .as_ref()
        .map(|complete| complete.len())
        .unwrap_or(edges.len());
    if !include_hidden && !complete_directory_relations {
        edges = balanced_edge_prefix_by_source(&edges, limit);
    }
    if edge_count > edges.len() && rel == "." {
        hidden.push(HiddenGroup {
            reason: "directory edges hidden by limit".to_string(),
            count: edge_count - edges.len(),
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if surface_count > surfaces.len() && rel != "." {
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
    if hidden_support_package_count > 0 && rel != "." {
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

    let observations = if let Some(complete) = complete_root {
        let current_level_entries = complete.direct_files.len() + complete.child_dir_count;
        let classified_entries = current_level_entries + complete.recursive_files.len();
        let complete_surfaces = directory_surfaces(project, rel, complete.grouped, true);
        root_ls_observations(
            project,
            &RootLsGroupCounts {
                surface_total: complete_surfaces.len(),
                packages_observed: shown_surface_facts(&complete_surfaces, "packages"),
                scripts_observed: shown_surface_facts(&complete_surfaces, "scripts"),
                tests_observed: shown_surface_facts(&complete_surfaces, "test_surfaces"),
                current_level_entries,
                classified_entries,
            },
            &surfaces,
        )
    } else {
        directory_relation_observation(
            project,
            ObservationProjection {
                group: "relations",
                scope: rel,
                observed: edge_count,
                shown: edges.len(),
                expand: (edges.len() < edge_count)
                    .then(|| format!("codemap ls {} --all", shell_quote(rel))),
            },
        )
    };
    LsReport {
        kind: "ls_report",
        schema_version: crate::model::LsReport::SCHEMA_VERSION,
        path: rel.to_string(),
        mode: "directory".to_string(),
        anchor: None,
        directory: surfaces,
        boundary_facts: boundary_facts_for_ls(project, rel),
        edges,
        observations,
        hidden,
        next: directory_next_commands(rel),
    }
}

struct DirectoryGrouping<'a> {
    grouped: BTreeMap<String, Vec<String>>,
    direct_files: Vec<&'a FileInfo>,
    recursive_files: Vec<&'a FileInfo>,
    hidden_generic_count: usize,
    hidden_support_package_count: usize,
    hidden_support_artifact_count: usize,
    child_dir_count: usize,
}

fn directory_grouping<'a>(
    project: &'a Project,
    rel: &str,
    include_hidden: bool,
) -> DirectoryGrouping<'a> {
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
    if rel == "." {
        for script in &project.scripts {
            grouped
                .entry("script".to_string())
                .or_default()
                .push(format!("{}: {}", script.name, script.command));
        }
    }

    let direct_files = direct_files_under_directory(project, rel);
    let scope_is_support = is_support_artifact_path(rel);
    let mut hidden_support_artifact_count = 0;
    let mut child_dir_count = 0;
    for dir in immediate_child_dirs(project, rel) {
        child_dir_count += 1;
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
        if is_generic_noise(file) && !include_hidden {
            grouped
                .entry("generic_hidden".to_string())
                .or_default()
                .push(file.rel.clone());
        } else {
            grouped.entry(kind).or_default().push(file.rel.clone());
        }
    }
    let recursive_files = files_under_directory(project, rel)
        .into_iter()
        .filter(|file| !direct_files.iter().any(|direct| direct.rel == file.rel))
        .collect::<Vec<_>>();
    if include_hidden {
        for file in &recursive_files {
            let file_kind = file_kind_for_ls(file);
            // Root `ls .` is a current-level atlas even in its complete
            // machine projection. Nested manifests/config/schema/CI rails
            // are root-level structural facts; arbitrary recursive source
            // files are not, and must be opened through an exact scope.
            if rel == "." && !inventory_recursive_structural_kind(&file_kind, &file.rel) {
                continue;
            }
            let kind = if rel == "." {
                file_kind
            } else {
                format!("recursive:{file_kind}")
            };
            grouped.entry(kind).or_default().push(file.rel.clone());
        }
    }
    let hidden_generic_count = grouped
        .remove("generic_hidden")
        .map(|files| files.len())
        .unwrap_or(0);
    let hidden_support_package_count = grouped
        .remove("support_package_hidden")
        .map(|files| files.len())
        .unwrap_or(0);
    DirectoryGrouping {
        grouped,
        direct_files,
        recursive_files,
        hidden_generic_count,
        hidden_support_package_count,
        hidden_support_artifact_count,
        child_dir_count,
    }
}

fn directory_surfaces(
    project: &Project,
    rel: &str,
    grouped: BTreeMap<String, Vec<String>>,
    include_all_examples: bool,
) -> Vec<DirectorySurface> {
    let mut surfaces = grouped
        .into_iter()
        .map(|(kind, mut files)| {
            files.sort();
            let count = files.len();
            let examples = if include_all_examples {
                files
            } else {
                files.into_iter().take(5).collect::<Vec<_>>()
            };
            let shown = examples.len();
            DirectorySurface {
                id: directory_surface_id(rel, &kind, &examples),
                path: if kind == "script" {
                    script_surface_path(project)
                } else {
                    directory_surface_path(&examples)
                },
                role: directory_surface_role(&kind),
                evidence: directory_surface_evidence(&kind),
                strength: directory_surface_strength(&kind),
                kind,
                count,
                examples,
                hidden_count: count.saturating_sub(shown),
            }
        })
        .collect::<Vec<_>>();
    surfaces.sort_by(|a, b| {
        surface_priority(&a.kind)
            .cmp(&surface_priority(&b.kind))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    surfaces
}
