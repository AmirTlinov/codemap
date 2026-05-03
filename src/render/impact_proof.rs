pub fn impact(report: &ImpactReport) {
    println!("# Structural Impact\n");
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
    if let Some(section) = section_filter {
        render_proof_filtered_section(report, section);
        return;
    }
    if let Some(target) = &report.target {
        println!("Target: `{target}`\n");
    }
    if !report.changed.is_empty() {
        println!("Changed anchors:");
        println!("{}", bullet(&report.changed, true, Some(20)));
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
        println!("\nNo proof surface found. Use `codemap cone <path>` to inspect edges first.");
        println!("\n{}", report.run_hint);
        return;
    }
    if !report.proofs.is_empty() {
        proof_plan_surface_section("Proofs", report);
    }
    if !report.fallback.is_empty() {
        println!("\n## Fallback\n");
        println!("{}", code_block("bash", &report.fallback));
    }
    unknown_section(&report.unknowns);
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
    println!("- fallback commands: `{}`", report.fallback.len());
    println!("- unknown entries: `{}`", report.unknowns.len());
    println!("- hidden groups: `{}`", report.hidden.len());
}

fn proof_links_section(report: &ProofReport) {
    if report.proofs.is_empty() {
        proof_empty_section(
            "Links",
            "No proof surface links were emitted by proof detectors for this report.",
        );
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
            proof.evidence,
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
    println!("## Roles\n");
    println!("- proof_surface: `{}`", report.proofs.len());
    println!("- fallback_command: `{}`", report.fallback.len());
    println!("- unknown_gap: `{}`", report.unknowns.len());
    println!("- hidden_group: `{}`", report.hidden.len());
}

fn proof_plan_section(report: &ProofReport, force: bool) {
    if !report.proofs.is_empty() {
        proof_plan_surface_section("Proof", report);
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
    unknown_section(&report.unknowns);
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

fn proof_plan_surface_section(title: &str, report: &ProofReport) {
    println!("\n## {title}");
    let mut grouped: std::collections::BTreeMap<String, Vec<&ProofSurface>> =
        std::collections::BTreeMap::new();
    for proof in &report.proofs {
        grouped
            .entry(proof_display_command(proof))
            .or_default()
            .push(proof);
    }
    for (command, proofs) in grouped {
        println!("\n### `{command}`");
        println!("- sensors: `{}`", proofs.len());
        proof_count_line("evidence", evidence_counts(&proofs));
        proof_count_line("strength", strength_counts(&proofs));
        let sample_limit = if proofs.len() <= 6 { proofs.len() } else { 5 };
        if sample_limit > 0 {
            println!("- sample:");
            for proof in proofs.iter().take(sample_limit) {
                let path = proof
                    .path
                    .as_ref()
                    .map(|path| code(path))
                    .unwrap_or_else(|| "`none`".to_string());
                println!(
                    "  - {path} [{}; {}] {} - {}",
                    proof.evidence,
                    format!("{:?}", proof.strength).to_ascii_lowercase(),
                    proof_location_summary(&proof.locations),
                    proof.reason
                );
            }
        }
        let hidden_details = proofs.len().saturating_sub(sample_limit);
        if hidden_details > 0 {
            println!("- hidden details: `{hidden_details}` sensors");
            if let Some(expand) = proof_detail_expand(report, proofs.len()) {
                println!("  expand: `{}`", root_aware_expand(&expand));
            }
        }
    }
}

fn proof_display_command(proof: &ProofSurface) -> String {
    let Some(command) = &proof.command else {
        return "no command".to_string();
    };
    let Some(path) = proof.path.as_deref() else {
        return command.clone();
    };
    let mut candidates = Vec::new();
    candidates.push(path.to_string());
    let parts = path.split('/').collect::<Vec<_>>();
    for index in 1..parts.len() {
        candidates.push(parts[index..].join("/"));
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.len()));
    for candidate in candidates {
        for suffix in [candidate.clone(), shell_quote_for_markdown(&candidate)] {
            let suffix = format!(" {suffix}");
            if let Some(stripped) = command.strip_suffix(&suffix) {
                return stripped.trim_end_matches(" --").trim_end().to_string();
            }
        }
    }
    command.clone()
}

fn evidence_counts(proofs: &[&ProofSurface]) -> Vec<(String, usize)> {
    let mut counts = std::collections::BTreeMap::new();
    for proof in proofs {
        *counts.entry(proof.evidence.clone()).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

fn strength_counts(proofs: &[&ProofSurface]) -> Vec<(String, usize)> {
    let mut counts = std::collections::BTreeMap::new();
    for proof in proofs {
        *counts
            .entry(format!("{:?}", proof.strength).to_ascii_lowercase())
            .or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

fn proof_count_line(label: &str, counts: Vec<(String, usize)>) {
    if counts.is_empty() {
        return;
    }
    let values = counts
        .into_iter()
        .map(|(kind, count)| format!("`{kind}: {count}`"))
        .collect::<Vec<_>>()
        .join(", ");
    println!("- {label}: {values}");
}

fn proof_detail_expand(report: &ProofReport, limit: usize) -> Option<String> {
    if let Some(target) = &report.target {
        return Some(format!(
            "codemap proof-map {} --raw-sensors --limit {limit}",
            shell_quote_for_markdown(target)
        ));
    }
    if !report.changed.is_empty() {
        return Some(format!(
            "codemap proof-map --changed --raw-sensors --limit {limit}"
        ));
    }
    None
}

fn shell_quote_for_markdown(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | '#'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn proof_location_summary(locations: &[EvidenceLocation]) -> String {
    let Some(first) = locations.first() else {
        return "unknown".to_string();
    };
    let suffix = if locations.len() > 1 {
        format!(" +{}", locations.len() - 1)
    } else {
        String::new()
    };
    let base = if first.path == "aggregate" {
        "aggregate".to_string()
    } else if let Some(line) = first.line_start {
        format!("{}:{line}", first.path)
    } else {
        first.path.clone()
    };
    format!("{}{}", code(&base), suffix)
}
