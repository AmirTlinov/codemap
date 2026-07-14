// Responsibility: changed-proof-section
use crate::model::{ChangedReport, ProofSurface};
use crate::render::{
    COMPACT_CHANGED_PROOF_COMMAND_LIMIT, ChangedProofGroupClass, changed_proof_command_groups,
    changed_proof_evidence_only_surfaces, changed_proof_groups_by_class,
    changed_proof_render_evidence_surfaces, changed_proof_render_groups,
    changed_proof_render_soft_summary, changed_proof_sensor_counts, changed_proof_surface_groups,
    changed_selector_suffix, code_block, proof_wiring_section, proof_wiring_summary_section,
    root_aware_expand,
};

pub(crate) fn changed_proof_section(report: &ChangedReport, compact: bool, detailed_wiring: bool) {
    println!("\n## Verification Surfaces");
    let runnable_grouped = changed_proof_command_groups(report);
    let setup_grouped = changed_proof_surface_groups(report.proof.setup_support.iter());
    let soft_grouped = changed_proof_surface_groups(report.proof.soft_evidence.iter());
    let runnable =
        changed_proof_groups_by_class(&runnable_grouped, ChangedProofGroupClass::Runnable);
    let evidence_only = changed_proof_evidence_only_surfaces(report);
    let setup = changed_proof_groups_by_class(&setup_grouped, ChangedProofGroupClass::Setup);
    let soft = changed_proof_groups_by_class(&soft_grouped, ChangedProofGroupClass::Soft);
    if compact {
        changed_proof_large_compact_summary(report, &runnable, &evidence_only, &setup, &soft);
        return;
    }
    if runnable.is_empty() && report.proof.fallback.is_empty() {
        println!("No runnable command surface found for this slice.");
    }
    changed_proof_render_groups(&runnable, compact, &report.selector);
    if !evidence_only.is_empty() {
        println!("\n## Linked Surfaces");
        changed_proof_render_evidence_surfaces(&evidence_only, compact, &report.selector);
        println!(
            "\nLinked surfaces are source-backed relations without a runnable command. They do not replace runnable command surfaces or remove Unknown entries."
        );
    }
    if !setup.is_empty() {
        println!("\n## Setup / Support Surfaces");
        changed_proof_render_groups(&setup, compact, &report.selector);
        println!(
            "\nSetup/support surfaces are connected rails such as install, codegen, migration, seed, deploy, release, watch, or dev-server steps. They are not verification command surfaces and are not run by `--run`."
        );
    }
    if !soft.is_empty() {
        println!("\n## Soft Surface Matches");
        changed_proof_render_soft_summary(&soft, &report.selector);
        println!(
            "\nSoft surface matches are token/name/path overlap. They do not create a direct linked verification surface or remove Unknown entries."
        );
    }
    if !report.proof.fallback.is_empty() {
        println!("\n### Fallback");
        println!("{}", code_block("bash", &report.proof.fallback));
    }
    let wiring_expand = format!(
        "codemap proof {} --section links",
        changed_proof_selector(&report.selector)
    );
    if detailed_wiring {
        proof_wiring_section(&report.proof.wiring, false, Some(&wiring_expand));
    } else {
        proof_wiring_summary_section(&report.proof.wiring, Some(&wiring_expand));
    }
    changed_proof_sensor_counts(report, compact);
}

fn changed_proof_large_compact_summary(
    report: &ChangedReport,
    runnable: &[(&String, &(Vec<&ProofSurface>, usize))],
    evidence_only: &[&ProofSurface],
    setup: &[(&String, &(Vec<&ProofSurface>, usize))],
    soft: &[(&String, &(Vec<&ProofSurface>, usize))],
) {
    println!(
        "- groups: runnable=`{}`; evidence_only=`{}`; setup_support=`{}`; soft=`{}`; fallback=`{}`",
        runnable.len(),
        evidence_only.len(),
        setup.len(),
        soft.len(),
        report.proof.fallback.len()
    );
    if !report.proof.wiring.is_empty() {
        changed_proof_wiring_counts(report);
    }
    let visible_group_count = runnable.len().min(COMPACT_CHANGED_PROOF_COMMAND_LIMIT);
    if visible_group_count > 0 {
        let commands = runnable
            .iter()
            .take(visible_group_count)
            .map(|(command, _)| format!("`{command}`"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("- runnable command surfaces: {commands}");
    }
    if runnable.len() > visible_group_count {
        let hidden_command_groups = runnable.len() - visible_group_count;
        println!("- hidden runnable command surface groups: `{hidden_command_groups}`");
    }
    println!(
        "- expand: `{}`",
        root_aware_expand(&format!(
            "codemap changed{} --section proof",
            changed_selector_suffix(&report.selector)
        ))
    );
    changed_proof_sensor_counts(report, true);
}

fn changed_proof_wiring_counts(report: &ChangedReport) {
    let mut counts = std::collections::BTreeMap::new();
    for fact in &report.proof.wiring {
        *counts.entry(fact.status.as_str()).or_insert(0usize) += 1;
    }
    let summary = counts
        .into_iter()
        .map(|(status, count)| format!("`{status}: {count}`"))
        .collect::<Vec<_>>()
        .join(", ");
    println!("- wiring: {summary}");
    println!(
        "- wiring expand: `{}`",
        root_aware_expand(&format!(
            "codemap proof {} --section links",
            changed_proof_selector(&report.selector)
        ))
    );
}

pub(crate) fn changed_proof_selector(selector: &str) -> String {
    if selector == "--changed" {
        "changed".to_string()
    } else {
        selector.to_string()
    }
}
