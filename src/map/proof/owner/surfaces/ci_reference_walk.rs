// Responsibility: map-proof-owner-surfaces-ci-reference-walk
use crate::map::{
    ci_inline_run_command, ci_run_steps, strip_inline_shell_comment, unique_proof_surfaces,
};
use crate::model::{EvidenceLocation, EvidenceStrength, FileInfo, Project, ProofSurface};

pub(crate) fn ci_run_reference_proof_surfaces<F>(
    project: &Project,
    file: &FileInfo,
    evidence: &str,
    matcher: F,
) -> Vec<ProofSurface>
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = Vec::new();
    for ci in project
        .files
        .values()
        .filter(|candidate| candidate.has_role("build_ci"))
    {
        let Ok(text) = std::fs::read_to_string(project.root.join(&ci.rel)) else {
            continue;
        };
        for step in ci_run_steps(&text) {
            let Some(reason) = matcher(&step.command) else {
                continue;
            };
            out.push(ProofSurface {
                command: Some(step.command.clone()),
                path: Some(ci.rel.clone()),
                target_anchor: Some(file.rel.clone()),
                evidence: evidence.to_string(),
                strength: EvidenceStrength::High,
                reason: format!("{reason} for {}", file.rel),
                locations: vec![EvidenceLocation::line(&ci.rel, step.line, "ci_step")],
            });
        }
    }
    unique_proof_surfaces(out)
}

pub(crate) fn ci_line_reference_proof_surfaces<F>(
    project: &Project,
    file: &FileInfo,
    evidence: &str,
    matcher: F,
) -> Vec<ProofSurface>
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = Vec::new();
    for ci in project
        .files
        .values()
        .filter(|candidate| candidate.has_role("build_ci"))
    {
        let Ok(text) = std::fs::read_to_string(project.root.join(&ci.rel)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let line_without_comments = strip_inline_shell_comment(line);
            if line_without_comments.trim_start().starts_with('#') {
                continue;
            }
            let Some(reason) = matcher(&line_without_comments) else {
                continue;
            };
            out.push(ProofSurface {
                command: ci_inline_run_command(line),
                path: Some(ci.rel.clone()),
                target_anchor: Some(file.rel.clone()),
                evidence: evidence.to_string(),
                strength: EvidenceStrength::High,
                reason: format!("{reason} for {}", file.rel),
                locations: vec![EvidenceLocation::line(&ci.rel, index + 1, "ci_step")],
            });
        }
    }
    unique_proof_surfaces(out)
}
