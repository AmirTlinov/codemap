// Responsibility: exact-workflow-dispatch-crossings
use crate::map::{ci_workflow, clean_path_token, shell_words, structural_edge_with_locations};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, StructuralEdge};
use std::path::Path;

pub(crate) fn ci_command_workflow_dispatch_edges(
    project: &Project,
    rel: &str,
    from: &str,
    command: &str,
    line: usize,
) -> Vec<StructuralEdge> {
    let words = shell_words(command);
    let target = words.windows(4).find_map(|window| {
        (clean_path_token(&window[0]) == "gh"
            && clean_path_token(&window[1]) == "workflow"
            && clean_path_token(&window[2]) == "run")
            .then(|| clean_path_token(&window[3]))
    });
    let Some(target) = target.and_then(|target| workflow_path(project, &target)) else {
        return Vec::new();
    };
    vec![structural_edge_with_locations(
        from.to_string(),
        target,
        "invokes_workflow",
        "gh_workflow_run",
        EvidenceStrength::Hard,
        vec![EvidenceLocation::line(rel, line, "workflow_dispatch")],
    )]
}

pub(crate) fn ci_file_workflow_dispatch_edges(project: &Project, rel: &str) -> Vec<StructuralEdge> {
    let Some(text) = project.read_indexed_text(rel) else {
        return Vec::new();
    };
    let Some(workflow) = ci_workflow(&text) else {
        return Vec::new();
    };
    workflow
        .jobs
        .into_iter()
        .flat_map(|job| {
            job.steps.into_iter().flat_map(move |step| {
                let step_id = format!("step:{rel}#{}/{:04}:{}", job.id, step.index, step.name);
                step.commands.into_iter().flat_map(move |command| {
                    ci_command_workflow_dispatch_edges(
                        project,
                        rel,
                        &step_id,
                        &command.command,
                        command.line,
                    )
                })
            })
        })
        .collect()
}

fn workflow_path(project: &Project, target: &str) -> Option<String> {
    let name = Path::new(target).file_name()?.to_str()?;
    let candidate = format!(".github/workflows/{name}");
    project.files.contains_key(&candidate).then_some(candidate)
}
