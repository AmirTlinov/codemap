// Responsibility: map-cone-owner-ci-edges
use crate::map::{
    ci_execution_edges, ci_file_workflow_dispatch_edges, ci_workflow_name, first_line_containing,
    sort_edges, structural_edge_with_locations,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge};
use std::path::Path;

pub(crate) fn owner_ci_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(text) = project.read_indexed_text(rel) else {
        return Vec::new();
    };
    let mut edges = ci_execution_edges(project, rel, &text);
    edges.extend(workflow_documentation_edges(project, rel, &text));
    sort_edges(&mut edges);
    edges
}

pub(crate) fn owner_ci_incoming_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    let target_name = Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(rel);
    for file in project
        .files
        .values()
        .filter(|file| file.rel != rel && file.has_role("build_ci"))
    {
        let Some(text) = project.read_indexed_text(&file.rel) else {
            continue;
        };
        if !text.contains(target_name) {
            continue;
        }
        let dispatches = ci_file_workflow_dispatch_edges(project, &file.rel)
            .into_iter()
            .filter(|edge| edge.to == rel)
            .collect::<Vec<_>>();
        if dispatches.is_empty() {
            edges.push(structural_edge_with_locations(
                file.rel.clone(),
                rel.to_string(),
                "references_workflow",
                "exact_workflow_filename_reference",
                EvidenceStrength::High,
                vec![EvidenceLocation::line(
                    &file.rel,
                    first_line_containing(project, &file.rel, &[target_name]).unwrap_or(1),
                    "workflow_reference",
                )],
            ));
        } else {
            edges.extend(dispatches);
        }
    }
    sort_edges(&mut edges);
    edges
}

fn workflow_documentation_edges(
    project: &Project,
    rel: &str,
    workflow_text: &str,
) -> Vec<StructuralEdge> {
    let Some(name) = ci_workflow_name(workflow_text) else {
        return Vec::new();
    };
    project
        .files
        .values()
        .filter(|file| file.has_role("docs") && file.rel.ends_with("/github-actions.md"))
        .filter_map(|docs| {
            let text = project.read_indexed_text(&docs.rel)?;
            text.contains(&name).then(|| {
                structural_edge_with_locations(
                    rel.to_string(),
                    docs.rel.clone(),
                    "documented_by",
                    "exact_workflow_display_name",
                    EvidenceStrength::High,
                    vec![EvidenceLocation::line(
                        &docs.rel,
                        first_line_containing(project, &docs.rel, &[&name]).unwrap_or(1),
                        "workflow_documentation",
                    )],
                )
            })
        })
        .collect()
}
