// Responsibility: bounded-ci-execution-path-projection
use crate::model::StructuralEdge;

pub(crate) fn limit_ci_execution_projection(edges: &mut Vec<StructuralEdge>, limit: usize) {
    if edges.len() <= limit {
        return;
    }
    let complete = edges.clone();
    let mut selected = Vec::new();
    let chain = complete
        .iter()
        .find(|edge| {
            edge.edge_type == "invokes_script"
                && complete
                    .iter()
                    .any(|candidate| candidate.from == edge.to && candidate.edge_type == "deploys")
        })
        .or_else(|| {
            complete
                .iter()
                .find(|edge| edge.edge_type == "invokes_script")
        });
    if let Some(invocation) = chain {
        if let Some(contains) = complete
            .iter()
            .find(|edge| edge.edge_type == "contains_step" && edge.to == invocation.from)
        {
            if let Some(job) = complete
                .iter()
                .find(|edge| edge.edge_type == "declares_job" && edge.to == contains.from)
            {
                push_unique_edge(&mut selected, job);
            }
            push_unique_edge(&mut selected, contains);
        }
        push_unique_edge(&mut selected, invocation);
        for edge_type in [
            "invokes_process",
            "deploys",
            "smoke_checks",
            "produces_receipt",
        ] {
            if let Some(edge) = complete
                .iter()
                .find(|edge| edge.from == invocation.to && edge.edge_type == edge_type)
            {
                push_unique_edge(&mut selected, edge);
            }
        }
    }
    for edge_type in [
        "declares_job",
        "contains_step",
        "invokes_workflow",
        "invokes_action",
        "invokes_script",
        "invokes_process",
        "deploys",
        "smoke_checks",
        "produces_receipt",
        "documented_by",
        "ci_validation_step",
        "ci_release_step",
        "ci_setup_step",
        "ci_control_step",
        "uses_external_action",
    ] {
        if selected.iter().any(|edge| edge.edge_type == edge_type) {
            continue;
        }
        let edge = if edge_type == "produces_receipt" {
            complete
                .iter()
                .find(|edge| {
                    edge.edge_type == edge_type && edge.to.contains("release-prod-receipt.json")
                })
                .or_else(|| complete.iter().find(|edge| edge.edge_type == edge_type))
        } else {
            complete.iter().find(|edge| edge.edge_type == edge_type)
        };
        if let Some(edge) = edge {
            push_unique_edge(&mut selected, edge);
        }
    }
    for edge in &complete {
        if selected.len() == limit {
            break;
        }
        push_unique_edge(&mut selected, edge);
    }
    selected.truncate(limit);
    *edges = selected;
}

fn push_unique_edge(edges: &mut Vec<StructuralEdge>, candidate: &StructuralEdge) {
    if !edges.iter().any(|edge| {
        edge.from == candidate.from
            && edge.to == candidate.to
            && edge.edge_type == candidate.edge_type
            && edge.evidence == candidate.evidence
    }) {
        edges.push(candidate.clone());
    }
}
