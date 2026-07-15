// Responsibility: map-listing-ls
mod anchor_reports;
mod root_horizons;
mod surface_meta;

pub(crate) use anchor_reports::*;
pub(crate) use root_horizons::*;
pub(crate) use surface_meta::*;

use crate::map::{
    ObservationProjection, RootAtlasProjection, balanced_edge_prefix_by_source,
    boundary_facts_for_ls, bounded_directory_surfaces, direct_files_under_directory,
    directory_edges, directory_relation_observation, directory_role_surface,
    directory_surface_observations, file_kind_for_ls, files_under_directory, immediate_child_dirs,
    inventory_recursive_structural_kind, is_generic_noise, is_support_artifact_path,
    path_under_scope, root_atlas_projection, shell_quote, surface_priority,
};
use crate::model::{DirectorySurface, FileInfo, HiddenGroup, LsReport, Project};
use std::collections::BTreeMap;

pub(crate) fn ls_directory_report(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    limit: usize,
    complete_directory_projection: bool,
) -> LsReport {
    // Root observation truth is assembled from the complete current scope and
    // is intentionally independent of the readable projection. This makes a
    // limit a display choice rather than a different candidate universe.
    let root_files = (rel == ".").then(|| project.files.keys().cloned().collect::<Vec<_>>());
    let root_atlas = root_files
        .as_ref()
        .map(|files| root_atlas_projection(&project.root, files, &project.packages));
    let complete_root =
        (rel == ".").then(|| directory_grouping(project, rel, true, root_atlas.as_ref()));
    let complete_nested_edges = (rel != ".").then(|| directory_edges(project, rel, true));
    let complete_nested = (rel != ".").then(|| directory_grouping(project, rel, true, None));
    let complete_nested_surfaces = complete_nested
        .as_ref()
        .map(|complete| directory_surfaces(project, rel, complete.grouped.clone(), true));
    let projected = directory_grouping(
        project,
        rel,
        include_hidden || complete_directory_projection,
        root_atlas.as_ref(),
    );
    let DirectoryGrouping {
        grouped,
        direct_files: _,
        recursive_files,
        hidden_generic_count,
        hidden_support_artifact_count,
        child_dir_count: _,
    } = projected;

    let mut surfaces = directory_surfaces(
        project,
        rel,
        grouped,
        include_hidden || complete_directory_projection,
    );
    if !complete_directory_projection {
        surfaces = bounded_directory_surfaces(surfaces, limit, rel);
    }

    let mut hidden = Vec::new();
    let mut edges = if complete_directory_projection {
        complete_nested_edges.clone().unwrap_or_default()
    } else {
        directory_edges(project, rel, include_hidden)
    };
    if rel == "." {
        merge_root_atlas_edges(&mut edges, root_atlas.as_ref(), include_hidden);
    }
    let edge_count = complete_nested_edges
        .as_ref()
        .map(|complete| complete.len())
        .unwrap_or(edges.len());
    if !include_hidden && !complete_directory_projection {
        edges = balanced_edge_prefix_by_source(&edges, limit);
    }
    if edge_count > edges.len() && rel == "." {
        hidden.push(HiddenGroup {
            reason: "directory edges hidden by limit".to_string(),
            count: edge_count - edges.len(),
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if hidden_generic_count > 0 && rel == "." {
        hidden.push(HiddenGroup {
            reason: "generic source files hidden".to_string(),
            count: hidden_generic_count,
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if hidden_support_artifact_count > 0 && rel == "." {
        hidden.push(HiddenGroup {
            reason: "support artifacts hidden".to_string(),
            count: hidden_support_artifact_count,
            expand: format!("codemap ls {} --all", shell_quote(rel)),
        });
    }
    if !include_hidden && !recursive_files.is_empty() && rel == "." {
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
        let complete_surfaces = complete_nested_surfaces
            .as_ref()
            .expect("nested directory surface inventory");
        let surface_group_count = complete_surfaces.len();
        let surface_member_count = complete_surfaces
            .iter()
            .map(|surface| surface.count)
            .sum::<usize>();
        let shown_member_count = surfaces
            .iter()
            .map(|surface| surface.examples.len())
            .sum::<usize>();
        let expand = || format!("codemap ls {} --all", shell_quote(rel));
        let mut observations = directory_relation_observation(
            project,
            ObservationProjection {
                group: "relations",
                scope: rel,
                observed: edge_count,
                shown: edges.len(),
                expand: (edges.len() < edge_count)
                    .then(|| format!("codemap ls {} --all", shell_quote(rel))),
            },
        );
        observations.merge(&directory_surface_observations(
            project,
            ObservationProjection {
                group: "surface_groups",
                scope: rel,
                observed: surface_group_count,
                shown: surfaces.len(),
                expand: (surfaces.len() < surface_group_count).then(expand),
            },
            ObservationProjection {
                group: "surface_members",
                scope: rel,
                observed: surface_member_count,
                shown: shown_member_count,
                expand: (shown_member_count < surface_member_count).then(expand),
            },
        ));
        observations
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
    hidden_support_artifact_count: usize,
    child_dir_count: usize,
}

fn directory_grouping<'a>(
    project: &'a Project,
    rel: &str,
    include_hidden: bool,
    root_atlas: Option<&RootAtlasProjection>,
) -> DirectoryGrouping<'a> {
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for domain in project.domains.iter().filter(|_| root_atlas.is_none()) {
        if path_under_scope(&domain.path, rel) {
            grouped
                .entry("domain".to_string())
                .or_default()
                .push(domain.path.clone());
        }
    }
    for package in project.packages.iter().filter(|_| root_atlas.is_none()) {
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
            // Complete root output keeps structural rails, not a recursive
            // source galaxy; exact scopes own the omitted file layer.
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
    if let Some(atlas) = root_atlas {
        for legacy_test_kind in ["test", "e2e_test", "test_support"] {
            grouped.remove(legacy_test_kind);
        }
        for (kind, paths) in &atlas.grouped {
            for path in paths {
                if !include_hidden && is_support_artifact_path(path) {
                    continue;
                }
                let values = grouped.entry(kind.clone()).or_default();
                if !values.contains(path) {
                    values.push(path.clone());
                }
            }
        }
    }
    let hidden_generic_count = grouped
        .remove("generic_hidden")
        .map(|files| files.len())
        .unwrap_or(0);
    grouped.remove("support_package_hidden");
    DirectoryGrouping {
        grouped,
        direct_files,
        recursive_files,
        hidden_generic_count,
        hidden_support_artifact_count,
        child_dir_count,
    }
}

fn merge_root_atlas_edges(
    edges: &mut Vec<crate::model::StructuralEdge>,
    atlas: Option<&RootAtlasProjection>,
    include_hidden: bool,
) {
    let Some(atlas) = atlas else {
        return;
    };
    for edge in atlas.edges.iter().filter(|edge| {
        include_hidden
            || (!is_support_artifact_path(&edge.from) && !is_support_artifact_path(&edge.to))
    }) {
        if let Some(existing) = edges.iter_mut().find(|existing| {
            existing.from == edge.from
                && existing.to == edge.to
                && existing.edge_type == edge.edge_type
        }) {
            *existing = edge.clone();
        } else {
            edges.push(edge.clone());
        }
    }
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.edge_type.cmp(&b.edge_type))
            .then_with(|| a.to.cmp(&b.to))
    });
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
            if kind == "domain" || kind.starts_with("package:") || kind.ends_with("_container") {
                files.sort_by(|a, b| {
                    a.matches('/')
                        .count()
                        .cmp(&b.matches('/').count())
                        .then_with(|| a.cmp(b))
                });
            } else {
                files.sort();
            }
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
