// Responsibility: proof-plan-filtered-sections
use crate::model::{ProofReport, Unknown};
use crate::render::{
    bullet, code, code_block, disclaimer, hidden_section, proof_detail_expand,
    proof_location_summary, proof_map_changed_selector, proof_plan_surface_sections,
    proof_target_suffix, proof_wiring_section, public_evidence_label, root_aware_expand,
    shell_quote_for_markdown, unknown_section, unknown_where,
};

pub(crate) fn render_proof_filtered_section(report: &ProofReport, section: &str) {
    match section {
        "observed" => proof_observed_section(report),
        "links" => proof_links_section(report),
        "roles" => proof_roles_section(report),
        "proof" => proof_plan_section(report, true),
        "unknown" => proof_unknown_section(report),
        "hidden" => proof_hidden_section(report),
        _ => {}
    }
}

fn proof_observed_section(report: &ProofReport) {
    println!("## Observed\n");
    if let Some(target) = &report.target {
        println!("- target: `{target}`");
    }
    if !report.changed.is_empty() {
        println!("- changed anchors: `{}`", report.changed.len());
        println!("{}", bullet(&report.changed, true, Some(20)));
    }
    if report.target.is_none() && report.changed.is_empty() {
        println!("- selected anchors: `0`");
    }
    println!("- verification surfaces: `{}`", report.proofs.len());
    if let Some(coverage) = &report.coverage {
        println!("- coverage changed files: `{}`", coverage.changed_count);
        println!(
            "- coverage without direct linked surface: `{}`",
            coverage.missing.len()
        );
    }
    println!("- fallback commands: `{}`", report.fallback.len());
    println!("- unknown entries: `{}`", report.unknowns.len());
    println!("- hidden groups: `{}`", report.hidden.len());
}

fn proof_links_section(report: &ProofReport) {
    if report.proofs.is_empty() && report.wiring.is_empty() {
        proof_empty_section(
            "Links",
            "No verification surface links were emitted by detectors for this report.",
        );
        return;
    }
    proof_wiring_section(
        &report.wiring,
        false,
        proof_wiring_expand(report).as_deref(),
    );
    if report.proofs.is_empty() {
        return;
    }
    println!("## Links\n");
    for proof in report.proofs.iter().take(20) {
        let path = proof
            .path
            .as_ref()
            .map(|path| code(path))
            .unwrap_or_else(|| "`none`".to_string());
        println!(
            "- {path}{} [{}; {}] {} - {}",
            proof_target_suffix(proof),
            public_evidence_label(&proof.evidence),
            format!("{:?}", proof.strength).to_ascii_lowercase(),
            proof_location_summary(&proof.locations),
            proof.reason
        );
    }
    let hidden = report.proofs.len().saturating_sub(20);
    if hidden > 0 {
        println!("- hidden verification links: `{hidden}`");
        if let Some(expand) = proof_detail_expand(report, report.proofs.len()) {
            println!("  expand: `{}`", root_aware_expand(&expand));
        }
    }
}

fn proof_roles_section(report: &ProofReport) {
    println!("## Surface Hints\n");
    disclaimer(
        "Derived from deterministic path/name/extension/manifest patterns. Not intent, correctness, or ownership truth.",
    );
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
    let setup_or_support = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_setup_or_support(proof))
        .count();
    let soft = report
        .proofs
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_soft_evidence(proof))
        .count();
    println!("- proof_surface: `{}`", report.proofs.len());
    println!("- runnable_proof: `{runnable}`");
    println!("- evidence_surface: `{evidence_only}`");
    println!("- setup_support_surface: `{setup_or_support}`");
    println!("- soft_evidence: `{soft}`");
    println!("- fallback_command: `{}`", report.fallback.len());
    println!("- unknown_gap: `{}`", report.unknowns.len());
    println!("- hidden_group: `{}`", report.hidden.len());
}

pub(crate) fn proof_wiring_expand(report: &ProofReport) -> Option<String> {
    if let Some(target) = &report.target {
        return Some(format!(
            "codemap proof {} --section links",
            shell_quote_for_markdown(target)
        ));
    }
    if !report.changed.is_empty() {
        return Some(format!(
            "codemap proof {} --section links",
            proof_map_changed_selector(report)
        ));
    }
    None
}

fn proof_plan_section(report: &ProofReport, force: bool) {
    if !report.proofs.is_empty() {
        proof_plan_surface_sections(report, force);
    }
    if !report.fallback.is_empty() {
        println!("\n## Fallback\n");
        println!("{}", code_block("bash", &report.fallback));
    }
    if force && report.proofs.is_empty() && report.fallback.is_empty() {
        proof_empty_section(
            "Verification Surfaces",
            "No verification surfaces or fallback commands were emitted by detectors for this report.",
        );
    }
}

fn proof_unknown_section(report: &ProofReport) {
    if report.unknowns.is_empty() {
        let detail = if proof_anchor_count(report) == 0 {
            "No proof anchors selected; verification Unknown checks did not run."
        } else {
            "No Unknown entries were emitted by verification surface detectors for this report."
        };
        proof_empty_section("Unknown", detail);
        return;
    }
    proof_unknowns_section(report);
}

pub(crate) fn proof_unknowns_section(report: &ProofReport) {
    if proof_unknowns_should_compact(report) {
        proof_compact_unknowns_section(report);
    } else {
        unknown_section(&report.unknowns);
    }
}

fn proof_unknowns_should_compact(report: &ProofReport) -> bool {
    report.target.is_none() && report.changed.len() > 5 && report.unknowns.len() > 5
}

fn proof_compact_unknowns_section(report: &ProofReport) {
    println!("\n## Unknown\n");
    let mut grouped: std::collections::BTreeMap<&str, Vec<&Unknown>> =
        std::collections::BTreeMap::new();
    for unknown in &report.unknowns {
        grouped
            .entry(unknown.kind.as_str())
            .or_default()
            .push(unknown);
    }
    for (kind, unknowns) in grouped {
        let sample = unknowns
            .iter()
            .take(5)
            .map(|unknown| unknown_where(unknown))
            .collect::<Vec<_>>()
            .join(", ");
        if sample.is_empty() {
            println!("- `{kind}`: `{}`", unknowns.len());
        } else {
            println!("- `{kind}`: `{}`; sample: {sample}", unknowns.len());
        }
        let hidden = unknowns.len().saturating_sub(5);
        if hidden > 0 {
            println!("  hidden: `{hidden}` unknowns");
            println!(
                "  expand: `{}`",
                root_aware_expand(&format!(
                    "codemap changed --section unknown --limit {}",
                    unknowns.len()
                ))
            );
        }
    }
}

fn proof_hidden_section(report: &ProofReport) {
    if report.hidden.is_empty() {
        proof_empty_section("Hidden", "No hidden proof material in this report.");
        return;
    }
    hidden_section(&report.hidden);
}

pub(crate) fn proof_empty_section(title: &str, detail: &str) {
    println!("## {title}\n");
    println!("{detail}");
}

pub(crate) fn proof_anchor_count(report: &ProofReport) -> usize {
    report
        .target
        .as_ref()
        .map(|_| 1)
        .unwrap_or(report.changed.len())
}
