// Responsibility: map-listing-root-inventory
mod atlas;
mod classify;
mod graph;

pub(crate) use atlas::*;
pub(crate) use classify::*;
pub(crate) use graph::*;

use crate::map::{
    RootInventoryObservationInput, boundary_facts_from_paths, directory_next_commands,
    group_visibility, inventory_recursive_structural_kind, inventory_root_script_edges,
    inventory_support_unit, inventory_workspace_edges, is_support_artifact_path,
    package_discovery_gap_observation, record_root_inventory_observations,
    root_script_manifest_partition, shown_surface_facts,
};
use crate::model::{HiddenGroup, LsReport, ObservationLedger};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn root_inventory_ls_report(
    root: &Path,
    files: &[String],
    include_hidden: bool,
    limit: usize,
) -> LsReport {
    let mut grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut complete_grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut hidden_support = BTreeSet::new();
    let mut recursive_hidden = 0usize;
    let mut source_edge_hidden = 0usize;
    let package_audit =
        crate::repo::audit_package_discovery_paths(root, files.iter().map(String::as_str));
    let mut atlas = root_atlas_projection(root, files, &package_audit.packages);
    for (kind, paths) in atlas.grouped {
        for path in paths {
            inventory_push(&mut complete_grouped, &kind, &path);
            if is_support_artifact_path(&path) && !include_hidden {
                hidden_support.insert(inventory_support_unit(&path));
            } else {
                inventory_push(&mut grouped, &kind, &path);
            }
        }
    }

    let top_level_dirs = inventory_top_level_dirs(files);
    let top_level_dir_count = top_level_dirs.len();
    for dir in top_level_dirs {
        if let Some(role) = inventory_dir_role(&dir) {
            inventory_push(&mut complete_grouped, &role, &dir);
        }
        inventory_push(&mut complete_grouped, "dir", &dir);
        if is_support_artifact_path(&dir) && !include_hidden {
            hidden_support.insert(dir);
            continue;
        }
        if let Some(role) = inventory_dir_role(&dir) {
            inventory_push(&mut grouped, &role, &dir);
        }
        inventory_push(&mut grouped, "dir", &dir);
    }

    if !include_hidden {
        atlas.edges.retain(|edge| {
            !is_support_artifact_path(&edge.from) && !is_support_artifact_path(&edge.to)
        });
    }
    let (script_labels, mut edges) = inventory_root_script_edges(root, files);
    edges.extend(atlas.edges);
    for label in script_labels {
        inventory_push(&mut complete_grouped, "script", &label);
        inventory_push(&mut grouped, "script", &label);
    }

    for rel in files {
        let direct = !rel.contains('/');
        let kind = inventory_file_kind(rel);
        let root_structural = inventory_recursive_structural_kind(&kind, rel);
        if direct || root_structural {
            inventory_push(&mut complete_grouped, &kind, rel);
        }

        if !direct && !root_structural {
            recursive_hidden += 1;
            if kind == "source" {
                source_edge_hidden += 1;
            }
        }

        if is_support_artifact_path(rel) && !include_hidden {
            hidden_support.insert(inventory_support_unit(rel));
            continue;
        }
        if direct {
            inventory_push(&mut grouped, &kind, rel);
        }
    }

    edges.extend(inventory_workspace_edges(root, files));
    edges.sort_by(|a, b| {
        inventory_edge_priority(a)
            .cmp(&inventory_edge_priority(b))
            .then_with(|| a.from.cmp(&b.from))
            .then_with(|| a.edge_type.cmp(&b.edge_type))
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });

    let packages_observed = package_audit.packages.len();
    let scripts_observed = complete_grouped.get("script").map_or(0, BTreeSet::len);
    let tests_observed = complete_grouped
        .iter()
        .filter(|(kind, _)| LsReport::TEST_SURFACE_KINDS.contains(&kind.as_str()))
        .map(|(_, files)| files.len())
        .sum::<usize>();
    let mut hidden = Vec::new();
    let complete_surfaces = inventory_surfaces(".", complete_grouped, true);
    let surface_count = complete_surfaces.len();
    let mut surfaces = if include_hidden {
        complete_surfaces
    } else {
        inventory_surfaces(".", grouped, false)
    };
    let script_rails = edges
        .iter()
        .filter(|edge| edge.edge_type == "declares_script")
        .map(|edge| edge.from.clone())
        .collect::<BTreeSet<_>>();
    if let Some(surface) = surfaces.iter_mut().find(|surface| surface.kind == "script")
        && script_rails.len() == 1
    {
        surface.path = script_rails.into_iter().next();
    }
    surfaces.truncate(limit);

    let edge_count = edges.len();
    if edge_count > limit {
        edges.truncate(limit);
        hidden.push(HiddenGroup {
            reason: "inventory edges hidden by limit".to_string(),
            count: edge_count - edges.len(),
            expand: "codemap ls . --all".to_string(),
        });
    }
    if !include_hidden && !hidden_support.is_empty() {
        hidden.push(HiddenGroup {
            reason: "support artifacts hidden".to_string(),
            count: hidden_support.len(),
            expand: "codemap ls . --all".to_string(),
        });
    }
    if !include_hidden && recursive_hidden > 0 {
        hidden.push(HiddenGroup {
            reason: "recursive files below this level hidden".to_string(),
            count: recursive_hidden,
            expand: "codemap ls . --all".to_string(),
        });
    }
    // A full JSON projection serializes every fact observed by this cold
    // inventory owner, but it cannot manufacture import edges that require
    // the full index. Keep that capability boundary explicit even when no
    // observed surface is display-hidden.
    if source_edge_hidden > 0 {
        hidden.push(HiddenGroup {
            reason: "full-index source edges hidden by bounded root inventory".to_string(),
            count: source_edge_hidden,
            expand: "codemap ls . --all".to_string(),
        });
    }

    let (script_manifests_visited, script_manifests_excluded) =
        root_script_manifest_partition(files.iter().map(String::as_str));
    let direct_file_count = files.iter().filter(|rel| !rel.contains('/')).count();
    let mut observations = ObservationLedger::default();
    record_root_inventory_observations(
        RootInventoryObservationInput {
            snapshot: crate::cache::inventory_fingerprint(root, files),
            classified_entries: (top_level_dir_count + files.len()) as u64,
            current_level_entries: (top_level_dir_count + direct_file_count) as u64,
            package_manifest_candidates: package_audit
                .candidates
                .into_iter()
                .map(|candidate| candidate.manifest)
                .collect(),
            package_manifests_visited: package_audit.visited_manifests,
            package_manifest_unsupported: package_audit
                .unsupported
                .into_iter()
                .map(package_discovery_gap_observation)
                .collect(),
            script_manifests_visited,
            script_manifests_excluded,
            full_index: false,
            complete_current_level_atlas: true,
            directory_surfaces: group_visibility(surface_count, surfaces.len()),
            packages: group_visibility(
                packages_observed,
                shown_surface_facts(&surfaces, "packages"),
            ),
            scripts: group_visibility(scripts_observed, shown_surface_facts(&surfaces, "scripts")),
            tests: group_visibility(
                tests_observed,
                shown_surface_facts(&surfaces, "test_surfaces"),
            ),
            test_surface_unsupported: Vec::new(),
        },
        &mut observations,
    );
    LsReport {
        kind: "ls_report",
        schema_version: crate::model::LsReport::SCHEMA_VERSION,
        path: ".".to_string(),
        mode: "directory".to_string(),
        anchor: None,
        directory: surfaces,
        boundary_facts: boundary_facts_from_paths(files.iter().cloned().collect()),
        edges,
        observations,
        hidden,
        next: directory_next_commands("."),
    }
}
