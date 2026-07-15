// Responsibility: map-proof-entry
mod impact;
mod report_unknowns;

pub(crate) use impact::*;
pub(crate) use report_unknowns::*;

use crate::map::{
    VerificationTopologyInput, balanced_proof_surface_prefix, changed_fail_open_unknowns,
    changed_should_check_direct_proof, codemap_changed_proof_surfaces, dedupe_unknowns,
    directory_has_files, file_uses_ci_run_step_syntax, impact_level_for_directory,
    nearest_proof_scope, nearest_proof_scope_unknown, proof_ci_run_step_is_validation,
    proof_coverage_summary, proof_fallback_commands, proof_missing_should_surface,
    proof_surface_is_soft_structural_match, proof_surface_satisfies_specific_proof,
    proof_surfaces_for_anchor, proof_surfaces_for_directory, proof_surfaces_for_symbol_anchor,
    proof_wiring_facts_limited, proof_wiring_unknowns, shell_quote, split_symbol_anchor,
    strict_test_edges_for_file, structural_impact_level_for_file, truncate_with_hidden,
    unique_proof_surfaces, unknown, unknown_ci_validation_step_not_found,
    unknown_missing_deterministic_proof, verification_topology,
};
use crate::model::{EvidenceStrength, Project, ProofReport, Risk};
use crate::repo;
use std::collections::BTreeMap;

pub(crate) fn is_support_artifact_path(rel: &str) -> bool {
    let rel = repo::normalize_rel_path(rel);
    rel.split('/').any(|part| {
        matches!(
            part,
            "fixtures"
                | "examples"
                | "samples"
                | "artifacts"
                | "receipts"
                | "witnesses"
                | ".agents"
                | ".codex"
                | ".claude"
        )
    })
}

pub fn proof_report(
    project: &Project,
    target: Option<String>,
    changed: Vec<String>,
    selector: String,
    depth: usize,
    limit: usize,
) -> ProofReport {
    let limit = limit.max(1);
    let target = target.map(|path| repo::normalize_rel_path(&path));
    let changed = changed
        .into_iter()
        .map(|file| repo::normalize_rel_path(&file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let anchors = if let Some(target) = target.as_ref() {
        vec![target.clone()]
    } else {
        changed.clone()
    };
    let mut proofs = Vec::new();
    let mut hidden = Vec::new();
    let mut risk = Risk::Low;
    let discovery_limit = usize::MAX;
    let mut coverage = None;
    if target.is_none() && !changed.is_empty() {
        let impact = impact_report(
            project,
            changed.clone(),
            selector.clone(),
            depth,
            limit.max(changed.len()),
        );
        for cluster in &impact.clusters {
            risk = risk.max(impact_level_from_str(&cluster.risk));
        }
        let mut proofs_by_anchor = BTreeMap::new();
        for anchor in &changed {
            let anchor_proofs = proof_surfaces_for_anchor(project, anchor, depth, discovery_limit);
            proofs_by_anchor.insert(anchor.clone(), anchor_proofs.clone());
            proofs.extend(anchor_proofs);
        }
        proofs.extend(codemap_changed_proof_surfaces(project));
        coverage = Some(proof_coverage_summary(&changed, &proofs_by_anchor));
    } else {
        for anchor in &anchors {
            if let Some((file_rel, symbol_name)) = split_symbol_anchor(anchor) {
                risk = risk.max(
                    project
                        .files
                        .get(&file_rel)
                        .map(|_| structural_impact_level_for_file(project, &file_rel, depth).0)
                        .unwrap_or(Risk::Medium),
                );
                proofs.extend(proof_surfaces_for_symbol_anchor(
                    project,
                    &file_rel,
                    &symbol_name,
                    depth,
                    discovery_limit,
                ));
            } else {
                if project.files.contains_key(anchor) {
                    risk = risk.max(structural_impact_level_for_file(project, anchor, depth).0);
                    proofs.extend(proof_surfaces_for_anchor(
                        project,
                        anchor,
                        depth,
                        discovery_limit,
                    ));
                } else if anchor != "." && directory_has_files(project, anchor) {
                    risk = risk.max(impact_level_for_directory(project, anchor, depth));
                    proofs.extend(proof_surfaces_for_directory(
                        project,
                        anchor,
                        depth,
                        discovery_limit,
                    ));
                } else {
                    risk = risk.max(Risk::Medium);
                    proofs.extend(proof_surfaces_for_anchor(
                        project,
                        anchor,
                        depth,
                        discovery_limit,
                    ));
                }
            }
        }
    }
    let all_proofs = unique_proof_surfaces(proofs);
    let fallback = proof_fallback_commands(project, &anchors, &changed, &all_proofs);
    let (mut wiring, _) = proof_wiring_facts_limited(
        project,
        &anchors,
        &changed,
        &all_proofs,
        &fallback,
        usize::MAX,
    );
    let mut unknowns = Vec::new();
    if target.is_none() && !changed.is_empty() {
        unknowns.extend(changed_fail_open_unknowns(project, &changed));
        unknowns.extend(changed_missing_deterministic_proof_unknowns(
            project,
            &changed,
            &fallback,
            &all_proofs,
        ));
    }
    if let Some(target) = target.as_ref()
        && let Some(file) = project.files.get(target)
    {
        unknowns.extend(proof_target_owner_unknowns(project, target, file));
    }
    if let Some(target) = target.as_ref()
        && let Some(file) = project.files.get(target)
        && changed_should_check_direct_proof(file)
        && !strict_test_edges_for_file(project, target, usize::MAX)
            .iter()
            .any(|(_, _, strength)| *strength >= EvidenceStrength::High)
    {
        unknowns.push(unknown(
            "direct_test_import_not_found",
            Some(target),
            None,
            "no direct test import, symbol reference, support import, or e2e route visit was found for this proof anchor",
            "verification surfaces may still include scripts, CI, contract checks, or soft matches, but no direct linked test surface was found",
            Some(format!("codemap proof-map {} --raw-sensors", shell_quote(target))),
        ));
    }
    if let Some(target) = target.as_ref()
        && let Some((file_rel, _symbol_name)) = split_symbol_anchor(target)
        && let Some(file) = project.files.get(&file_rel)
        && changed_should_check_direct_proof(file)
        && !all_proofs
            .iter()
            .any(proof_surface_satisfies_specific_proof)
    {
        unknowns.push(unknown(
            "direct_test_import_not_found",
            Some(target),
            None,
            "no direct test import, symbol reference, support import, or e2e route visit was found for this symbol proof anchor",
            "mediated symbol-consumer surfaces may still be visible, but they do not create a direct linked test surface for the selected symbol",
            Some(format!("codemap proof-map {} --raw-sensors", shell_quote(target))),
        ));
    }
    if let Some(target) = target.as_ref()
        && let Some(file) = project.files.get(target)
        && file_uses_ci_run_step_syntax(file)
        && !all_proofs.iter().any(proof_ci_run_step_is_validation)
    {
        unknowns.push(unknown_ci_validation_step_not_found(target));
    }
    if let Some(target) = target.as_ref()
        && (project.files.contains_key(target) || directory_has_files(project, target))
        && (proof_missing_should_surface(project, target)
            || all_proofs
                .iter()
                .any(proof_surface_is_soft_structural_match))
        && !all_proofs.is_empty()
        && !all_proofs
            .iter()
            .any(proof_surface_satisfies_specific_proof)
    {
        unknowns.push(unknown_missing_deterministic_proof(
            target,
            format!("codemap proof-map {}", shell_quote(target)),
        ));
    }
    unknowns.extend(proof_wiring_unknowns(&wiring));
    dedupe_unknowns(&mut unknowns);
    proofs = all_proofs;
    let proof_count = proofs.len();
    if proof_count > limit {
        let selected = balanced_proof_surface_prefix(&proofs, limit);
        let projection = crate::map::BoundedProjection::selected(
            "verification surfaces hidden by limit",
            proof_count,
            selected,
            &format!("codemap proof {} --depth {depth}", selector),
        );
        let (shown, hidden_group) = projection.into_parts();
        proofs = shown;
        hidden.extend(hidden_group);
    }
    truncate_with_hidden(
        &mut wiring,
        limit.saturating_mul(2).max(6),
        &mut hidden,
        "verification wiring facts hidden by limit",
        &format!("codemap proof {} --section links", selector),
    );
    let mut expand = Vec::new();
    if proofs.is_empty()
        && let Some(target) = target.as_ref()
        && (project.files.contains_key(target) || directory_has_files(project, target))
        && let Some(nearest) = nearest_proof_scope(project, target)
    {
        let command = format!("codemap proof {}", shell_quote(&nearest));
        unknowns.push(nearest_proof_scope_unknown(
            target,
            &nearest,
            command.clone(),
        ));
        expand.push(command);
    }
    let verification_topology = verification_topology(VerificationTopologyInput {
        project,
        proofs: &proofs,
        missing: &[],
        wiring: &wiring,
        unknowns: &unknowns,
        hidden: &hidden,
        expand: &expand,
    });
    ProofReport {
        kind: "proof_plan",
        schema_version: crate::model::ProofReport::SCHEMA_VERSION,
        target,
        changed,
        selector,
        risk: risk.as_str().to_string(),
        proofs,
        coverage,
        wiring,
        verification_topology,
        fallback,
        unknowns,
        hidden,
        expand,
        run_hint: "codemap proof prints a verification surface plan by default; use --run to execute rendered commands"
            .to_string(),
    }
}

pub fn clean_proof_report(selector: String) -> ProofReport {
    ProofReport {
        kind: "proof_plan",
        schema_version: crate::model::ProofReport::SCHEMA_VERSION,
        target: None,
        changed: Vec::new(),
        selector,
        risk: "low".to_string(),
        proofs: Vec::new(),
        coverage: None,
        wiring: Vec::new(),
        verification_topology: crate::map::unavailable_verification_topology(
            "clean_worktree_has_no_verification_selection",
            Vec::new(),
        ),
        fallback: Vec::new(),
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: Vec::new(),
        run_hint: "codemap proof prints a verification surface plan by default; use --run to execute rendered commands"
            .to_string(),
    }
}
