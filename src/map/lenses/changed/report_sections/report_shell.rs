// Responsibility: changed-report-shell
use crate::map::{changed_expand, changed_proof_selector};
use crate::model::{
    BoundaryFacts, ChangedMapDelta, ChangedProofSummary, ChangedReport, GitChange, ProofMapReport,
};

pub fn clean_changed_report(selector: String, limit: usize) -> ChangedReport {
    ChangedReport {
        kind: "changed_report",
        schema_version: "8",
        selector: selector.clone(),
        display_limit: limit.max(1),
        proof_plan_cache: None,
        proof_map_cache: None,
        total_changed_count: 0,
        changed: Vec::new(),
        git_state: Vec::new(),
        structural_events: Vec::new(),
        map_delta: ChangedMapDelta {
            added_edges: 0,
            removed_edges: 0,
            changed_symbols: 0,
            added_exports: 0,
            removed_exports: 0,
            added_runtime_routes: 0,
            removed_runtime_routes: 0,
            added_env: 0,
            removed_env: 0,
            added_proof_surfaces: 0,
            removed_proof_surfaces: 0,
            new_unknowns: 0,
        },
        risks: Vec::new(),
        coupling: Vec::new(),
        boundary_facts: BoundaryFacts::default(),
        impact: Vec::new(),
        proof: ChangedProofSummary {
            commands: Vec::new(),
            fallback: Vec::new(),
            hard: Vec::new(),
            direct_evidence: Vec::new(),
            mediated_evidence: Vec::new(),
            soft_evidence: Vec::new(),
            setup_support: Vec::new(),
            missing_direct: Vec::new(),
            wiring: Vec::new(),
        },
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: changed_expand(&selector),
    }
}

pub(crate) fn changed_report_shell(
    selector: &str,
    limit: usize,
    total_changed_count: usize,
    git_state: Vec<GitChange>,
) -> ChangedReport {
    ChangedReport {
        kind: "changed_report",
        schema_version: "8",
        selector: selector.to_string(),
        display_limit: limit,
        proof_plan_cache: None,
        proof_map_cache: None,
        total_changed_count,
        changed: Vec::new(),
        git_state,
        structural_events: Vec::new(),
        map_delta: ChangedMapDelta {
            added_edges: 0,
            removed_edges: 0,
            changed_symbols: 0,
            added_exports: 0,
            removed_exports: 0,
            added_runtime_routes: 0,
            removed_runtime_routes: 0,
            added_env: 0,
            removed_env: 0,
            added_proof_surfaces: 0,
            removed_proof_surfaces: 0,
            new_unknowns: 0,
        },
        risks: Vec::new(),
        coupling: Vec::new(),
        boundary_facts: BoundaryFacts::default(),
        impact: Vec::new(),
        proof: empty_changed_proof_summary(),
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: changed_expand(selector),
    }
}

fn empty_changed_proof_summary() -> ChangedProofSummary {
    ChangedProofSummary {
        commands: Vec::new(),
        fallback: Vec::new(),
        hard: Vec::new(),
        direct_evidence: Vec::new(),
        mediated_evidence: Vec::new(),
        soft_evidence: Vec::new(),
        setup_support: Vec::new(),
        missing_direct: Vec::new(),
        wiring: Vec::new(),
    }
}

pub(crate) fn empty_proof_map_report(selector: String, changed: Vec<String>) -> ProofMapReport {
    ProofMapReport {
        kind: "proof_map_report",
        schema_version: "4",
        selector: selector.clone(),
        scope: None,
        changed,
        hard: Vec::new(),
        direct_evidence: Vec::new(),
        mediated_evidence: Vec::new(),
        soft_evidence: Vec::new(),
        setup_support: Vec::new(),
        missing_direct: Vec::new(),
        commands: Vec::new(),
        wiring: Vec::new(),
        fallback: Vec::new(),
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: vec![format!(
            "codemap proof {}",
            changed_proof_selector(&selector)
        )],
    }
}
