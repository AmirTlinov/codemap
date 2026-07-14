// Responsibility: map-listing-root-inventory
mod classify;
mod graph;

pub(crate) use classify::*;
pub(crate) use graph::*;

use crate::map::{
    boundary_facts_from_paths, directory_next_commands, inventory_package_kind,
    inventory_recursive_structural_kind, inventory_root_script_edges, inventory_support_unit,
    inventory_workspace_edges, is_support_artifact_path,
};
use crate::model::{HiddenGroup, LsReport};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) fn root_inventory_ls_report(root: &Path, files: &[String], limit: usize) -> LsReport {
    let mut grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut hidden_support = BTreeSet::new();
    let mut recursive_hidden = 0usize;
    let mut source_edge_hidden = 0usize;

    for dir in inventory_top_level_dirs(files) {
        if is_support_artifact_path(&dir) {
            hidden_support.insert(dir);
            continue;
        }
        if let Some(role) = inventory_dir_role(&dir) {
            inventory_push(&mut grouped, &role, &dir);
        }
        inventory_push(&mut grouped, "dir", &dir);
    }

    let (script_labels, mut edges) = inventory_root_script_edges(root, files);
    for label in script_labels {
        inventory_push(&mut grouped, "script", &label);
    }

    for rel in files {
        if is_support_artifact_path(rel) {
            hidden_support.insert(inventory_support_unit(rel));
            continue;
        }
        let direct = !rel.contains('/');
        let kind = inventory_file_kind(rel);
        if let Some(package_kind) = inventory_package_kind(root, rel) {
            inventory_push(&mut grouped, &package_kind, rel);
        }
        if direct || inventory_recursive_structural_kind(&kind, rel) {
            inventory_push(&mut grouped, &kind, rel);
        } else {
            recursive_hidden += 1;
            if kind == "source" {
                source_edge_hidden += 1;
            }
        }
    }

    edges.extend(inventory_workspace_edges(root, files));
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| inventory_edge_priority(a).cmp(&inventory_edge_priority(b)))
            .then_with(|| a.edge_type.cmp(&b.edge_type))
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });

    let mut hidden = Vec::new();
    let mut surfaces = inventory_surfaces(".", grouped);
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
    let surface_count = surfaces.len();
    surfaces.truncate(limit);
    if surface_count > surfaces.len() {
        hidden.push(HiddenGroup {
            reason: "directory surfaces hidden by limit".to_string(),
            count: surface_count - surfaces.len(),
            expand: "codemap ls . --all".to_string(),
        });
    }

    let edge_count = edges.len();
    if edge_count > limit {
        edges.truncate(limit);
        hidden.push(HiddenGroup {
            reason: "inventory edges hidden by limit".to_string(),
            count: edge_count - edges.len(),
            expand: "codemap ls . --all".to_string(),
        });
    }
    if !hidden_support.is_empty() {
        hidden.push(HiddenGroup {
            reason: "support artifacts hidden".to_string(),
            count: hidden_support.len(),
            expand: "codemap ls . --all".to_string(),
        });
    }
    if recursive_hidden > 0 {
        hidden.push(HiddenGroup {
            reason: "recursive files below this level hidden".to_string(),
            count: recursive_hidden,
            expand: "codemap ls . --all".to_string(),
        });
    }
    if source_edge_hidden > 0 {
        hidden.push(HiddenGroup {
            reason: "full-index source edges hidden by bounded root inventory".to_string(),
            count: source_edge_hidden,
            expand: "codemap ls . --all".to_string(),
        });
    }

    LsReport {
        kind: "ls_report",
        schema_version: "6",
        path: ".".to_string(),
        mode: "directory".to_string(),
        anchor: None,
        directory: surfaces,
        boundary_facts: boundary_facts_from_paths(files.iter().cloned().collect()),
        edges,
        hidden,
        next: directory_next_commands("."),
    }
}
