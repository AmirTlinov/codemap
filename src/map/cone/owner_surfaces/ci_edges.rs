// Responsibility: map-cone-owner-ci-edges
use crate::map::ci_execution_edges;
use crate::model::{Project, StructuralEdge};

pub(crate) fn owner_ci_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(text) = project.read_indexed_text(rel) else {
        return Vec::new();
    };
    ci_execution_edges(project, rel, &text)
}
