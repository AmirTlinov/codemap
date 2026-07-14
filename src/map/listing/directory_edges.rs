// Responsibility: map-listing-directory-edges
mod command_surfaces;
mod endpoints;
mod owner_support;

pub(crate) use command_surfaces::*;
pub(crate) use endpoints::*;
pub(crate) use owner_support::*;

use crate::map::{
    current_level_owner_edges, edge_with_aggregate_location, files_under_directory,
    is_support_artifact_path, sort_edges,
};
use crate::model::{EvidenceStrength, Project, StructuralEdge};
use std::collections::BTreeMap;

pub(crate) fn directory_edges(
    project: &Project,
    rel: &str,
    include_hidden: bool,
) -> Vec<StructuralEdge> {
    directory_edges_at_depth(project, rel, include_hidden, 1)
}

pub(crate) fn directory_edges_at_depth(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut grouped: BTreeMap<(String, String, String, String, EvidenceStrength), usize> =
        BTreeMap::new();
    let scope_is_support = is_support_artifact_path(rel);
    for file in files_under_directory(project, rel) {
        for target in &file.resolved_imports {
            if !include_hidden
                && !scope_is_support
                && (is_support_artifact_path(&file.rel) || is_support_artifact_path(target))
            {
                continue;
            }
            let from = directory_edge_endpoint_at_depth(project, rel, &file.rel, endpoint_depth);
            let to = directory_edge_endpoint_at_depth(project, rel, target, endpoint_depth);
            if from != to {
                add_directory_edge(
                    &mut grouped,
                    from,
                    to,
                    "outgoing_import",
                    "resolved_import",
                    EvidenceStrength::High,
                );
            }
        }
        if let Some(importers) = project.reverse_imports.get(&file.rel) {
            for importer in importers {
                if path_under_scope(importer, rel) {
                    continue;
                }
                if !include_hidden
                    && !scope_is_support
                    && (is_support_artifact_path(&file.rel) || is_support_artifact_path(importer))
                {
                    continue;
                }
                let from = directory_edge_endpoint_at_depth(project, rel, importer, endpoint_depth);
                let to = directory_edge_endpoint_at_depth(project, rel, &file.rel, endpoint_depth);
                if from != to {
                    add_directory_edge(
                        &mut grouped,
                        from,
                        to,
                        "incoming_import",
                        "reverse_import",
                        EvidenceStrength::High,
                    );
                }
            }
        }
    }
    for edge in &project.package_edges {
        if !include_hidden
            && !scope_is_support
            && (is_support_artifact_path(&edge.from_manifest)
                || edge
                    .to_manifest
                    .as_ref()
                    .map(|to| is_support_artifact_path(to))
                    .unwrap_or_else(|| is_support_artifact_path(&edge.to)))
        {
            continue;
        }
        let from_in = path_under_scope(&edge.from_manifest, rel);
        let to_in = edge
            .to_manifest
            .as_ref()
            .map(|to| path_under_scope(to, rel))
            .unwrap_or_else(|| path_under_scope(&edge.to, rel));
        if from_in || to_in {
            add_directory_edge(
                &mut grouped,
                directory_edge_endpoint_at_depth(project, rel, &edge.from_manifest, endpoint_depth),
                directory_edge_endpoint_at_depth(
                    project,
                    rel,
                    &edge.to_manifest.clone().unwrap_or_else(|| edge.to.clone()),
                    endpoint_depth,
                ),
                if from_in && to_in {
                    "package_internal"
                } else if from_in {
                    "package_outgoing"
                } else {
                    "package_incoming"
                },
                &format!("package_manifest:{}", edge.dependency),
                EvidenceStrength::High,
            );
        }
    }
    let mut edges = grouped
        .into_iter()
        .map(|((from, to, edge_type, evidence, strength), count)| {
            edge_with_aggregate_location(
                from,
                to,
                edge_type,
                if count > 1 {
                    format!("{evidence}:{count}")
                } else {
                    evidence
                },
                strength,
                "directory_edge_aggregate",
            )
        })
        .collect::<Vec<_>>();
    edges.extend(current_level_owner_edges(
        project,
        rel,
        include_hidden,
        endpoint_depth,
    ));
    sort_edges(&mut edges);
    edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    edges
}

pub(crate) fn add_directory_edge(
    grouped: &mut BTreeMap<(String, String, String, String, EvidenceStrength), usize>,
    from: String,
    to: String,
    edge_type: &str,
    evidence: &str,
    strength: EvidenceStrength,
) {
    if from == to {
        return;
    }
    *grouped
        .entry((
            from,
            to,
            edge_type.to_string(),
            evidence.to_string(),
            strength,
        ))
        .or_insert(0) += 1;
}
