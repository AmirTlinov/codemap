pub fn impact(report: &ImpactReport) {
    println!("# Structural Impact\n");
    map_snapshot_line();
    if report.changed.is_empty() && report.clusters.is_empty() {
        println!("No changed anchors detected. Use `--files a,b` or run with a git diff selector.");
        return;
    }
    if !report.changed.is_empty() {
        render_file_summaries("Changed Anchors", &report.changed);
    }
    impact_summary_section(&report.clusters);
    for cluster in &report.clusters {
        render_impact_cluster(cluster);
    }
    hidden_section(&report.hidden);
    unknown_section(&report.unknowns);
    section("Expand", &report.expand);
}

fn impact_summary_section(clusters: &[ImpactCluster]) {
    if clusters.is_empty() {
        return;
    }
    println!("\n## Impact\n");
    render_impact_summary_lines(clusters);
}

fn render_impact_summary_lines(clusters: &[ImpactCluster]) {
    for cluster in clusters {
        println!(
            "- `{}` [direct={}; cross={}; contract={}; proof={}]",
            cluster.id,
            cluster.direct_consumers.len(),
            cluster.cross_boundary_consumers.len(),
            cluster.contract_links.len(),
            cluster.proof.len()
        );
        if !cluster.reasons.is_empty() {
            println!("  reasons: {}", cluster.reasons.join("; "));
        }
    }
}

fn render_impact_cluster(cluster: &ImpactCluster) {
    println!("\n## Cluster `{}`", cluster.id);
    if !cluster.changed.is_empty() {
        println!("changed:");
        println!("{}", bullet(&cluster.changed, true, Some(10)));
    }
    if !cluster.reasons.is_empty() {
        println!("reasons:");
        println!("{}", bullet(&cluster.reasons, false, Some(6)));
    }
    grouped_edge_list("direct consumers", &cluster.direct_consumers, 12);
    grouped_edge_list("cross-boundary consumers", &cluster.cross_boundary_consumers, 12);
    grouped_edge_list("contract links", &cluster.contract_links, 12);
    grouped_edge_list("proof", &cluster.proof, 12);
}

pub fn proof(report: &ProofReport, section_filter: Option<&str>) {
    println!("# Proof Plan\n");
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
    if report.proofs.is_empty()
        && report.fallback.is_empty()
        && report.unknowns.is_empty()
        && report.expand.is_empty()
    {
        if proof_anchor_count(report) == 0 {
            println!(
                "\nNo changed anchors selected. `codemap proof changed` has no proof scope in a clean repo."
            );
        } else {
            println!("\nNo proof surface found. Use `codemap cone <path>` to inspect edges first.");
        }
        println!("\n{}", report.run_hint);
        return;
    }
    if let Some(coverage) = &report.coverage {
        proof_coverage_section(coverage);
    }
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

fn render_proof_filtered_section(report: &ProofReport, section: &str) {
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

fn proof_large_changed_compact(report: &ProofReport) -> bool {
    report.target.is_none()
        && (report.changed.len() > 5
            || (report.changed.len() > 3
                && report
                    .hidden
                    .iter()
                    .any(|group| group.reason.contains("proof wiring") && group.count > 50))
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
    println!("\n## Proof\n");
    println!("- runnable deterministic sensors: `{runnable}`");
    println!("- evidence-only sensors: `{evidence_only}`");
    println!("- setup/support sensors: `{setup}`");
    println!("- soft evidence sensors: `{soft}`");
    if let Some(expand) = proof_detail_expand(report, report.proofs.len()) {
        println!("- expand: `{}`", root_aware_expand(&expand));
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
    println!("- proof surfaces: `{}`", report.proofs.len());
    if let Some(coverage) = &report.coverage {
        println!("- coverage changed files: `{}`", coverage.changed_count);
        println!(
            "- coverage missing direct proof: `{}`",
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
            "No proof surface links were emitted by proof detectors for this report.",
        );
        return;
    }
    proof_wiring_section(&report.wiring, false, proof_wiring_expand(report).as_deref());
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
            "- {path} [{}; {}] {} - {}",
            public_evidence_label(&proof.evidence),
            format!("{:?}", proof.strength).to_ascii_lowercase(),
            proof_location_summary(&proof.locations),
            proof.reason
        );
    }
    let hidden = report.proofs.len().saturating_sub(20);
    if hidden > 0 {
        println!("- hidden proof links: `{hidden}`");
        if let Some(expand) = proof_detail_expand(report, report.proofs.len()) {
            println!("  expand: `{}`", root_aware_expand(&expand));
        }
    }
}

fn proof_roles_section(report: &ProofReport) {
    println!("## Surface Hints\n");
    println!("Derived from deterministic path/name/extension/manifest patterns. Not intent, correctness, or ownership truth.\n");
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

fn proof_wiring_expand(report: &ProofReport) -> Option<String> {
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
            "Proof",
            "No proof surfaces or fallback commands were emitted by proof detectors for this report.",
        );
    }
}

fn proof_unknown_section(report: &ProofReport) {
    if report.unknowns.is_empty() {
        let detail = if proof_anchor_count(report) == 0 {
            "No proof anchors selected; proof Unknown checks did not run."
        } else {
            "No Unknown entries were emitted by proof detectors for this report."
        };
        proof_empty_section("Unknown", detail);
        return;
    }
    proof_unknowns_section(report);
}

fn proof_unknowns_section(report: &ProofReport) {
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
        grouped.entry(unknown.kind.as_str()).or_default().push(unknown);
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

fn proof_empty_section(title: &str, detail: &str) {
    println!("## {title}\n");
    println!("{detail}");
}

fn proof_anchor_count(report: &ProofReport) -> usize {
    report
        .target
        .as_ref()
        .map(|_| 1)
        .unwrap_or(report.changed.len())
}
