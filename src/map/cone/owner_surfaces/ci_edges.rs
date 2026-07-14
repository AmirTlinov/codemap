// Responsibility: map-cone-owner-ci-edges
use crate::map::{ci_owner_step_kind_for_project, ci_run_steps, structural_edge_with_locations};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge};

pub(crate) fn owner_ci_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return Vec::new();
    };
    ci_run_steps(&text)
        .into_iter()
        .filter_map(|step| {
            let kind = ci_owner_step_kind_for_project(project, &step.command)?;
            Some(structural_edge_with_locations(
                rel.to_string(),
                step.command,
                kind.edge_type(),
                kind.evidence(),
                EvidenceStrength::Hard,
                vec![EvidenceLocation::line(rel, step.line, "ci_step")],
            ))
        })
        .collect()
}
