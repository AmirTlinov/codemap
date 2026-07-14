// Responsibility: map-command-inference-roles-surfaces
use crate::map::{
    ProofRoleContext, anchor_file_rel, proof_role_context, role_aware_script_rank,
    script_search_text, script_text_has_token, unique,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, ProofSurface};
use std::collections::BTreeSet;

pub(crate) fn role_aware_minimal_commands(
    project: &Project,
    files: &[String],
    changed: &[String],
) -> Vec<String> {
    if !changed.is_empty() && !project.anchors.proof.changed.is_empty() {
        return project.anchors.proof.changed.clone();
    }
    let anchors = if changed.is_empty() { files } else { changed };
    let Some(context) = proof_role_context(project, anchors) else {
        return Vec::new();
    };
    let mut candidates = project
        .scripts
        .iter()
        .filter_map(|script| role_aware_script_rank(script, &context).map(|rank| (rank, script)))
        .collect::<Vec<_>>();
    candidates.sort_by(|(left_rank, left), (right_rank, right)| {
        left_rank
            .cmp(right_rank)
            .then_with(|| left.command.cmp(&right.command))
    });
    unique(
        candidates
            .into_iter()
            .map(|(_, script)| script.command.clone())
            .collect(),
    )
    .into_iter()
    .take(3)
    .collect()
}

pub(crate) fn codemap_changed_proof_surfaces(project: &Project) -> Vec<ProofSurface> {
    project
        .anchors
        .proof
        .changed
        .iter()
        .filter_map(|command| {
            let command = command.trim();
            if command.is_empty() {
                return None;
            }
            Some(ProofSurface {
                command: Some(command.to_string()),
                path: project.config_path.clone(),
                target_anchor: None,
                evidence: "codemap_proof_changed".to_string(),
                strength: EvidenceStrength::Hard,
                reason: ".codemap.yml proof.changed command".to_string(),
                locations: codemap_config_locations(project, "codemap_proof_changed"),
            })
        })
        .collect()
}

fn codemap_config_locations(project: &Project, kind: &str) -> Vec<EvidenceLocation> {
    project
        .config_path
        .as_ref()
        .map(|path| vec![EvidenceLocation::path(path, kind)])
        .unwrap_or_default()
}

pub(crate) fn role_aware_command_proof_surfaces(
    project: &Project,
    anchor: &str,
) -> Vec<ProofSurface> {
    let rel = anchor_file_rel(anchor);
    let Some(context) = proof_role_context(project, std::slice::from_ref(&rel)) else {
        return Vec::new();
    };
    let mut candidates = project
        .scripts
        .iter()
        .filter_map(|script| {
            let rank = role_aware_script_rank(script, &context)?;
            let (evidence, strength) = role_aware_script_evidence(script, &context);
            Some((rank, script.command.clone(), script, evidence, strength))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let mut seen = BTreeSet::new();
    candidates
        .into_iter()
        .filter(|(_, command, _, _, _)| seen.insert(command.clone()))
        .take(3)
        .map(|(_, command, script, evidence, strength)| ProofSurface {
            command: Some(command),
            path: script.path.clone().or_else(|| Some(rel.clone())),
            target_anchor: Some(rel.clone()),
            evidence: evidence.to_string(),
            strength,
            reason: format!(
                "{} matches structural proof context for {}",
                script.reason, rel
            ),
            locations: role_aware_script_locations(script, evidence),
        })
        .collect()
}

fn role_aware_script_evidence(
    script: &crate::model::ScriptInfo,
    context: &ProofRoleContext,
) -> (&'static str, EvidenceStrength) {
    let text = script_search_text(script);
    let token_hits = context
        .tokens
        .iter()
        .filter(|token| script_text_has_token(&text, token))
        .count();
    if token_hits >= 2 {
        ("script_path_token", EvidenceStrength::Medium)
    } else {
        ("script_surface_match", EvidenceStrength::Medium)
    }
}

pub(crate) fn role_aware_script_locations(
    script: &crate::model::ScriptInfo,
    evidence: &str,
) -> Vec<EvidenceLocation> {
    let Some(path) = &script.path else {
        return Vec::new();
    };
    match script.line_start {
        Some(line) => vec![EvidenceLocation::line(path, line, evidence)],
        None => vec![EvidenceLocation::path(path, evidence)],
    }
}
