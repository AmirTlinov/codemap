// Responsibility: proof-map-lens-rendering
use crate::model::ProofMapReport;
use crate::render::{
    code_block, hidden_section, map_snapshot_line, proof_command_summary_section,
    proof_surface_section, proof_wiring_summary_section, root_aware_expand, section,
    shell_quote_for_markdown, surface_section, unknown_section,
};

pub fn proof_map(report: &ProofMapReport) {
    println!("# Verification Surface Map\n");
    map_snapshot_line();
    if let Some(scope) = &report.scope {
        println!("Scope: `{scope}`");
    }
    if !report.changed.is_empty() {
        proof_map_changed_summary(report);
    }
    if proof_map_should_compact(report) {
        proof_map_compact_surface_summary(report);
    } else {
        proof_map_surface_sections(report);
    }
    surface_section("No Direct Linked Surface", &report.missing_direct);
    let runnable_commands = report
        .commands
        .iter()
        .filter(|proof| crate::proof_classification::proof_surface_is_runnable_validation(proof))
        .cloned()
        .collect::<Vec<_>>();
    proof_command_summary_section("Runnable Command Surfaces", &runnable_commands);
    proof_wiring_summary_section(&report.wiring, Some(&proof_map_wiring_expand(report)));
    if !report.fallback.is_empty() {
        println!("\n## Fallback\n");
        println!("{}", code_block("bash", &report.fallback));
    }
    unknown_section(&report.unknowns);
    hidden_section(&report.hidden);
    section("Expand", &report.expand);
}

fn proof_map_changed_summary(report: &ProofMapReport) {
    let sample = report
        .changed
        .iter()
        .take(5)
        .map(|path| format!("`{path}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = report.changed.len().saturating_sub(5);
    if hidden == 0 {
        println!("Changed: {sample}");
    } else {
        println!("Changed: sample: {sample}; hidden: `{hidden}` anchors");
    }
    println!(
        "Changed expand: `{}`",
        root_aware_expand(&format!(
            "codemap changed{} --section observed --limit {}",
            proof_map_changed_selector_suffix(&report.selector),
            report.changed.len()
        ))
    );
}

fn proof_map_changed_selector_suffix(selector: &str) -> String {
    match selector {
        "" | "changed" | "--changed" => String::new(),
        selector => format!(" {selector}"),
    }
}

fn proof_map_wiring_expand(report: &ProofMapReport) -> String {
    if let Some(scope) = &report.scope {
        return format!(
            "codemap proof-map {} --raw-sensors",
            shell_quote_for_markdown(scope)
        );
    }
    "codemap proof-map --changed --raw-sensors".to_string()
}

fn proof_map_should_compact(report: &ProofMapReport) -> bool {
    report.scope.as_deref() == Some(".") && proof_map_surface_count(report) > 24
}

fn proof_map_surface_count(report: &ProofMapReport) -> usize {
    report.hard.len()
        + report.direct_evidence.len()
        + report.mediated_evidence.len()
        + report.soft_evidence.len()
        + report.setup_support.len()
        + report.missing_direct.len()
}

fn proof_map_compact_surface_summary(report: &ProofMapReport) {
    println!("\n## Verification Surfaces\n");
    println!("- runnable verification surfaces: `{}`", report.hard.len());
    println!(
        "- direct linked surfaces: `{}`",
        report.direct_evidence.len()
    );
    println!(
        "- mediated linked surfaces: `{}`",
        report.mediated_evidence.len()
    );
    println!("- soft surface matches: `{}`", report.soft_evidence.len());
    println!("- setup/support: `{}`", report.setup_support.len());
    println!(
        "- no direct linked surface: `{}`",
        report.missing_direct.len()
    );
    println!(
        "- expand: `{}`",
        root_aware_expand(&proof_map_wiring_expand(report))
    );
    if !report.soft_evidence.is_empty() {
        println!(
            "- soft surface matches are token/name/path overlap; they do not create direct linked verification surfaces."
        );
    }
    if !report.setup_support.is_empty() {
        println!("- setup/support surfaces are rails, not verification command surfaces.");
    }
}

fn proof_map_surface_sections(report: &ProofMapReport) {
    proof_surface_section("Runnable Verification Surfaces", &report.hard);
    proof_surface_section("Direct Linked Surfaces", &report.direct_evidence);
    proof_surface_section("Mediated Linked Surfaces", &report.mediated_evidence);
    proof_surface_section("Soft Surface Matches", &report.soft_evidence);
    proof_surface_section("Setup / Support Surfaces", &report.setup_support);
    if !report.mediated_evidence.is_empty() {
        println!(
            "\nMediated linked surfaces are connected through a direct consumer, dependency, symbol consumer, barrel, or runtime bridge. They do not replace a direct linked verification surface or remove Unknown entries."
        );
    }
    if !report.soft_evidence.is_empty() {
        println!(
            "\nSoft surface matches are token/name/path overlap. They do not create a direct linked verification surface or remove Unknown entries."
        );
    }
    if !report.setup_support.is_empty() {
        println!(
            "\nSetup/support surfaces are connected rails such as install, codegen, migration, seed, deploy, release, watch, or dev-server steps. They are not verification command surfaces."
        );
    }
}
