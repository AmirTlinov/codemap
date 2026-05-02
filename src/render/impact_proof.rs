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
    let rows = clusters
        .iter()
        .map(|cluster| {
            vec![
                code(&cluster.id),
                code(&cluster.risk),
                cluster.reasons.join("; "),
                format!(
                    "direct={} cross={} contract={} proof={}",
                    cluster.direct_consumers.len(),
                    cluster.cross_boundary_consumers.len(),
                    cluster.contract_risks.len(),
                    cluster.proof.len()
                ),
            ]
        })
        .collect();
    println!("{}", table(&["Cluster", "Risk", "Reasons", "Edges"], rows));
}

fn render_impact_cluster(cluster: &ImpactCluster) {
    println!("\n## Cluster `{}`", cluster.id);
    println!("risk: `{}`", cluster.risk);
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
    grouped_edge_list("contract risks", &cluster.contract_risks, 12);
    grouped_edge_list("proof", &cluster.proof, 12);
}

pub fn proof(report: &ProofReport) {
    println!("# Proof Plan\n");
    if let Some(target) = &report.target {
        println!("Target: `{target}`\n");
    }
    if !report.changed.is_empty() {
        println!("Changed anchors:");
        println!("{}", bullet(&report.changed, true, Some(20)));
        println!();
    }
    println!(
        "{}",
        table(
            &["Field", "Value"],
            vec![vec!["Risk".to_string(), report.risk.clone()]]
        )
    );
    if report.proofs.is_empty() && report.fallback.is_empty() {
        println!("\nNo proof surface found. Use `codemap cone <path>` to inspect edges first.");
        println!("\n{}", report.run_hint);
        return;
    }
    if !report.proofs.is_empty() {
        proof_surface_section("Proofs", &report.proofs);
    }
    if !report.fallback.is_empty() {
        println!("\n## Fallback\n");
        println!("{}", code_block("bash", &report.fallback));
    }
    hidden_section(&report.hidden);
    println!("\n{}", report.run_hint);
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
