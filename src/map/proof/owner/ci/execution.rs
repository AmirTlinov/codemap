// Responsibility: workflow-to-owner-execution-projection
use crate::map::{
    ci_command_execution_edges, ci_owner_command_is_shell_syntax_only,
    ci_owner_step_kind_for_project, ci_receipt_edges, ci_script_execution_edges,
    ci_step_smoke_edge, ci_workflow, structural_edge_with_locations, unknown,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge, Unknown};

pub(crate) fn ci_execution_edges(project: &Project, rel: &str, text: &str) -> Vec<StructuralEdge> {
    let Some(workflow) = ci_workflow(text) else {
        return Vec::new();
    };
    let mut edges = Vec::new();
    for job in workflow.jobs {
        let job_id = ci_job_coordinate(rel, &job.id);
        edges.push(edge(
            rel,
            &job_id,
            "declares_job",
            "workflow_job",
            rel,
            job.line,
        ));
        for step in job.steps {
            let step_id = ci_step_coordinate(rel, &job.id, step.index, &step.name);
            edges.push(edge(
                &job_id,
                &step_id,
                "contains_step",
                "workflow_step",
                rel,
                step.line,
            ));
            if let Some(uses) = &step.uses {
                let (edge_type, target, strength) = ci_action_target(project, uses);
                edges.push(structural_edge_with_locations(
                    &step_id,
                    target,
                    edge_type,
                    "workflow_uses",
                    strength,
                    vec![EvidenceLocation::line(rel, step.line, "ci_step")],
                ));
            }
            if let Some(smoke) = ci_step_smoke_edge(rel, &step_id, &step) {
                edges.push(smoke);
            }
            edges.extend(ci_receipt_edges(rel, &step_id, &step));
            for command in &step.commands {
                if let Some(kind) = ci_owner_step_kind_for_project(project, &command.command) {
                    edges.push(structural_edge_with_locations(
                        &step_id,
                        &command.command,
                        kind.edge_type(),
                        kind.evidence(),
                        EvidenceStrength::Hard,
                        vec![EvidenceLocation::line(rel, command.line, "ci_step")],
                    ));
                }
                let command_edges = ci_command_execution_edges(
                    project,
                    rel,
                    &step_id,
                    &command.command,
                    command.line,
                );
                let scripts = command_edges
                    .iter()
                    .filter(|edge| edge.edge_type == "invokes_script")
                    .map(|edge| edge.to.clone())
                    .collect::<Vec<_>>();
                edges.extend(command_edges);
                for script in scripts {
                    edges.extend(ci_script_execution_edges(project, &script));
                }
            }
        }
    }
    edges
}

pub(crate) fn ci_execution_unknowns(rel: &str, text: &str) -> Vec<Unknown> {
    let Some(workflow) = ci_workflow(text) else {
        return vec![unknown(
            "workflow_structure_unresolved",
            Some(rel),
            None,
            "CI file has no statically parseable jobs mapping",
            "workflow job and step execution boundaries are unavailable",
            Some(format!("codemap cone {rel} --all")),
        )];
    };
    let mut unknowns = Vec::new();
    for job in workflow.jobs {
        for step in job.steps {
            if let Some(uses) = step.uses.as_deref()
                && !uses.starts_with("./")
            {
                unknowns.push(unknown(
                    "external_action_execution",
                    Some(rel),
                    Some(step.line),
                    format!("external action `{uses}` is outside repository static truth"),
                    "execution inside the action is not expanded",
                    Some(format!("codemap cone {rel} --all")),
                ));
            }
            if let Some(command) = step
                .commands
                .iter()
                .find(|command| command.command.contains("<<"))
            {
                unknowns.push(unknown(
                    "heredoc_execution_boundary",
                    Some(rel),
                    Some(command.line),
                    "heredoc body is kept inside its workflow step instead of interpreted as shell actions",
                    "static writes can remain visible, but arbitrary embedded execution is not expanded",
                    Some(format!("codemap cone {rel} --all")),
                ));
            }
            if let Some(command) = step
                .commands
                .iter()
                .find(|command| command.command.contains("$(") || command.command.contains("eval "))
            {
                unknowns.push(unknown(
                    "computed_shell_execution",
                    Some(rel),
                    Some(command.line),
                    "computed shell command or substitution crosses the static command boundary",
                    "the computed command target is not asserted as an execution edge",
                    Some(format!("codemap cone {rel} --all")),
                ));
            }
            if let Some(command) = step
                .commands
                .iter()
                .find(|command| ci_owner_command_is_shell_syntax_only(&command.command))
            {
                unknowns.push(unknown(
                    "shell_control_execution",
                    Some(rel),
                    Some(command.line),
                    "shell branch or control fragment is kept inside its workflow step",
                    "commands selected by shell control flow are not asserted as a linear execution path",
                    Some(format!("codemap cone {rel} --all")),
                ));
            }
        }
    }
    unknowns
}

fn ci_action_target(project: &Project, uses: &str) -> (&'static str, String, EvidenceStrength) {
    if let Some(path) = uses.strip_prefix("./") {
        let normalized = path.trim_end_matches('/');
        let exists = project.files.contains_key(normalized)
            || project
                .files
                .keys()
                .any(|rel| rel.starts_with(&format!("{normalized}/")));
        return (
            if normalized.starts_with(".github/workflows/") {
                "invokes_workflow"
            } else {
                "invokes_action"
            },
            normalized.to_string(),
            if exists {
                EvidenceStrength::Hard
            } else {
                EvidenceStrength::Medium
            },
        );
    }
    (
        "uses_external_action",
        format!("action:{uses}"),
        EvidenceStrength::Hard,
    )
}

fn ci_job_coordinate(rel: &str, job: &str) -> String {
    format!("job:{rel}#{job}")
}

fn ci_step_coordinate(rel: &str, job: &str, index: usize, name: &str) -> String {
    format!("step:{rel}#{job}/{index:04}:{name}")
}

fn edge(
    from: &str,
    to: &str,
    edge_type: &str,
    evidence: &str,
    rel: &str,
    line: usize,
) -> StructuralEdge {
    structural_edge_with_locations(
        from,
        to,
        edge_type,
        evidence,
        EvidenceStrength::Hard,
        vec![EvidenceLocation::line(rel, line, evidence)],
    )
}
