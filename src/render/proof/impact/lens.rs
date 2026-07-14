// Responsibility: impact-cluster-rendering
use crate::model::{ImpactCluster, ImpactReport};
use crate::render::{
    bullet, grouped_edge_list, hidden_section, map_snapshot_line, render_file_summaries, section,
    unknown_section,
};

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
        let verification_count = cluster
            .proof
            .iter()
            .filter(|edge| {
                !crate::proof_classification::proof_evidence_is_soft_match(&edge.evidence)
            })
            .count();
        let soft_count = cluster.proof.len().saturating_sub(verification_count);
        let soft_suffix = if soft_count > 0 {
            format!("; soft={soft_count}")
        } else {
            String::new()
        };
        println!(
            "- `{}` [direct={}; cross={}; contract={}; verification={}{}]",
            cluster.id,
            cluster.direct_consumers.len(),
            cluster.cross_boundary_consumers.len(),
            cluster.contract_links.len(),
            verification_count,
            soft_suffix
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
    grouped_edge_list(
        "cross-boundary consumers",
        &cluster.cross_boundary_consumers,
        12,
    );
    grouped_edge_list("contract links", &cluster.contract_links, 12);
    grouped_edge_list("verification surfaces", &cluster.proof, 12);
}
