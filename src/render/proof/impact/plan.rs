// Responsibility: proof-plan-rendering
use crate::model::ProofReport;
use crate::render::{
    bullet, code_block, hidden_section, map_prelude_line_or_snapshot_line, proof_anchor_count,
    proof_changed_command_selector_suffix, proof_coverage_section, proof_detail_expand,
    proof_most_direct_section, proof_plan_surface_sections, proof_unknowns_section,
    proof_wiring_expand, proof_wiring_summary_section, render_proof_filtered_section,
    root_aware_expand, section, verification_topology_section,
};

pub fn proof(report: &ProofReport, section_filter: Option<&str>) {
    println!("# Verification Surface Plan\n");
    map_prelude_line_or_snapshot_line();
    if let Some(section) = section_filter {
        render_proof_filtered_section(report, section);
        return;
    }
    if let Some(target) = &report.target {
        println!("Target: `{target}`\n");
    }
    if !report.changed.is_empty() {
        println!("Changed anchors:");
        if proof_large_changed_compact(report) {
            proof_changed_anchor_summary(report);
        } else {
            println!("{}", bullet(&report.changed, true, Some(20)));
        }
        println!();
    }
    println!("\n## Summary\n");
    if !report.changed.is_empty() {
        println!("- changed anchors: `{}`", report.changed.len());
    } else if report.target.is_some() {
        println!("- target anchors: `1`");
    } else {
        println!("- target anchors: `0`");
    }
    verification_topology_section(&report.verification_topology);
    if report.proofs.is_empty()
        && report.fallback.is_empty()
        && report.unknowns.is_empty()
        && report.expand.is_empty()
    {
        if proof_anchor_count(report) == 0 {
            println!(
                "\nNo changed anchors selected. `codemap proof changed` has no verification surface scope in a clean repo."
            );
        } else {
            println!(
                "\nNo verification surface found. Use `codemap cone <path>` to inspect edges first."
            );
        }
        println!("\n{}", report.run_hint);
        return;
    }
    if let Some(coverage) = &report.coverage {
        proof_coverage_section(coverage);
    }
    proof_most_direct_section(report);
    if proof_large_changed_compact(report) {
        proof_large_changed_summary(report);
    } else if !report.proofs.is_empty() {
        proof_plan_surface_sections(report, false);
    }
    if !report.fallback.is_empty() {
        println!("\n## Fallback\n");
        println!("{}", code_block("bash", &report.fallback));
    }
    proof_wiring_summary_section(&report.wiring, proof_wiring_expand(report).as_deref());
    proof_unknowns_section(report);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
    println!("\n{}", report.run_hint);
}

fn proof_large_changed_compact(report: &ProofReport) -> bool {
    report.target.is_none()
        && (report.changed.len() > 5
            || (report.changed.len() > 3
                && report
                    .hidden
                    .iter()
                    .any(|group| group.reason.contains("verification wiring") && group.count > 50))
            || report
                .unknowns
                .iter()
                .filter(|unknown| unknown.kind == "predicate_not_found")
                .count()
                > 12)
}

fn proof_changed_anchor_summary(report: &ProofReport) {
    let sample = report
        .changed
        .iter()
        .take(5)
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = report.changed.len().saturating_sub(5);
    if hidden == 0 {
        println!("- sample: {sample}");
    } else {
        println!("- sample: {sample}; hidden: `{hidden}` anchors");
    }
    let changed_suffix = proof_changed_command_selector_suffix(report);
    println!(
        "- expand: `{}`",
        root_aware_expand(&format!(
            "codemap changed{changed_suffix} --section observed --limit {}",
            report.changed.len()
        ))
    );
}

fn proof_large_changed_summary(report: &ProofReport) {
    let runnable = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_runnable_validation(proof))
        .count();
    let evidence_only = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_evidence_only(proof))
        .count();
    let setup = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_setup_or_support(proof))
        .count();
    let soft = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_soft_evidence(proof))
        .count();
    println!("\n## Verification Surfaces\n");
    println!("- runnable command sensors: `{runnable}`");
    println!("- linked-only sensors: `{evidence_only}`");
    println!("- setup/support sensors: `{setup}`");
    println!("- soft-match sensors: `{soft}`");
    if let Some(expand) = proof_detail_expand(report, report.proofs.len()) {
        println!("- expand: `{}`", root_aware_expand(&expand));
    }
}
