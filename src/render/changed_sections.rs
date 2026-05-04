fn changed_links_section(report: &ChangedReport, compact: bool, force: bool) {
    if report.impact.is_empty() {
        if force {
            println!("\n## Links\n");
            println!("No deterministic links found.");
        }
        return;
    }
    println!("\n## Links\n");
    if compact && report.total_changed_count > 20 {
        changed_link_large_compact_summary(report);
        return;
    }
    if compact {
        changed_link_summary_lines(report, changed_render_limit(report, true));
        return;
    }
    for cluster in &report.impact {
        println!("\n### `{}`", cluster.id);
        if !cluster.changed.is_empty() {
            println!("changed:");
            println!("{}", bullet(&cluster.changed, true, Some(10)));
        }
        if !cluster.reasons.is_empty() {
            println!("facts:");
            println!("{}", bullet(&cluster.reasons, false, Some(6)));
        }
        grouped_edge_list("direct consumers", &cluster.direct_consumers, 8);
        grouped_edge_list("cross-boundary consumers", &cluster.cross_boundary_consumers, 8);
        grouped_edge_list("contract links", &cluster.contract_links, 8);
        if !cluster.proof.is_empty() {
            println!("proof links: {}", cluster.proof.len());
        }
    }
}

fn changed_link_large_compact_summary(report: &ChangedReport) {
    let direct = report
        .impact
        .iter()
        .map(|cluster| cluster.direct_consumers.len())
        .sum::<usize>();
    let cross = report
        .impact
        .iter()
        .map(|cluster| cluster.cross_boundary_consumers.len())
        .sum::<usize>();
    let contract = report
        .impact
        .iter()
        .map(|cluster| cluster.contract_links.len())
        .sum::<usize>();
    let proof = report
        .impact
        .iter()
        .map(|cluster| cluster.proof.len())
        .sum::<usize>();
    println!(
        "- clusters: `{}` [direct={direct}; cross={cross}; contract={contract}; proof={proof}]",
        report.impact.len()
    );
    println!(
        "- expand: `{}`",
        root_aware_expand(&format!(
            "codemap changed{} --section links",
            changed_selector_suffix(&report.selector)
        ))
    );
}

fn changed_link_summary_lines(report: &ChangedReport, limit: usize) {
    let clusters = &report.impact;
    let paths = clusters
        .iter()
        .filter_map(|cluster| cluster.id.strip_prefix("changed:"))
        .collect::<Vec<_>>();
    let prefix = changed_common_dir_prefix(&paths);
    if let Some(prefix) = &prefix {
        println!("prefix: `{prefix}`");
    }
    for cluster in clusters.iter().take(limit) {
        let label = cluster
            .id
            .strip_prefix("changed:")
            .map(|path| changed_relative_path(path, prefix.as_deref()))
            .unwrap_or_else(|| cluster.id.clone());
        println!(
            "- `{}` [direct={}; cross={}; contract={}; proof={}]",
            label,
            cluster.direct_consumers.len(),
            cluster.cross_boundary_consumers.len(),
            cluster.contract_links.len(),
            cluster.proof.len()
        );
        if !cluster.reasons.is_empty() {
            println!("  facts: {}", cluster.reasons.join("; "));
        }
    }
    let hidden = clusters.len().saturating_sub(limit);
    if hidden > 0 {
        println!("- hidden link clusters: `{hidden}`");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section links --limit {}",
                changed_selector_suffix(&report.selector),
                clusters.len()
            ))
        );
    }
}

fn changed_unknown_section(report: &ChangedReport, force: bool, compact: bool) {
    let values = &report.unknowns;
    if values.is_empty() {
        if force {
            println!("\n## Unknown\n");
            println!("No Unknown entries recorded for this selector.");
        }
        return;
    }
    let compact = compact || changed_unknowns_should_compact(values, report.display_limit);
    if !compact {
        unknown_section(values);
        return;
    }
    println!("\n## Unknown\n");
    let limit = changed_render_limit(report, true);
    let mut grouped: std::collections::BTreeMap<&str, Vec<&Unknown>> =
        std::collections::BTreeMap::new();
    for unknown in values {
        grouped.entry(unknown.kind.as_str()).or_default().push(unknown);
    }
    for (kind, unknowns) in grouped {
        let sample = unknowns
            .iter()
            .take(limit)
            .map(|unknown| unknown_where(unknown))
            .collect::<Vec<_>>()
            .join(", ");
        if sample.is_empty() {
            println!("- `{kind}`: `{}`", unknowns.len());
        } else {
            println!("- `{kind}`: `{}`; sample: {sample}", unknowns.len());
        }
        let hidden = unknowns.len().saturating_sub(limit);
        if hidden > 0 {
            let expand = root_aware_expand(&format!(
                "codemap changed{} --section unknown --limit {}",
                changed_selector_suffix(&report.selector),
                unknowns.len()
            ));
            println!("  hidden: `{hidden}` unknowns; expand: `{expand}`");
        }
    }
}

fn changed_unknowns_should_compact(values: &[Unknown], display_limit: usize) -> bool {
    if changed_display_limit_is_expanded(display_limit) {
        return false;
    }
    if values.len() > display_limit {
        return true;
    }
    let mut grouped: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for unknown in values {
        *grouped.entry(unknown.kind.as_str()).or_default() += 1;
    }
    grouped.values().any(|count| *count > 5)
}

fn changed_display_limit_is_expanded(display_limit: usize) -> bool {
    display_limit > 10_000
}
