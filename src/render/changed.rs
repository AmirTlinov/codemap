pub fn changed(report: &ChangedReport, section_filter: &str) {
    println!("# Changed Map\n");
    println!("Selector: `{}`", report.selector);
    if report.total_changed_count > report.changed.len() {
        println!(
            "Changed: `{}` shown / `{}` total files",
            report.changed.len(),
            report.total_changed_count
        );
    } else {
        println!("Changed: `{}` files", report.changed.len());
    }
    if report.changed.is_empty() && report.git_state.is_empty() {
        println!("\nNo changed anchors detected.");
        section("Expand", &report.expand);
        return;
    }
    if section_filter == "overview" {
        changed_summary(report);
    }
    if matches!(section_filter, "overview" | "diff") {
        changed_delta_section(report);
    }
    if matches!(section_filter, "overview" | "impact") {
        changed_impact_section(report, section_filter == "overview");
    }
    if matches!(section_filter, "overview" | "proof") {
        changed_proof_section(report);
    }
    if matches!(section_filter, "overview" | "unknowns") {
        unknown_section(&report.unknowns);
    }
    let mut hidden = report.hidden.clone();
    if report.git_state.len() > report.display_limit {
        hidden.push(crate::model::HiddenGroup {
            reason: "git state rows hidden by limit".to_string(),
            count: report.git_state.len() - report.display_limit,
            expand: format!(
                "codemap changed{} --limit {}",
                changed_selector_suffix(&report.selector),
                report.git_state.len()
            ),
        });
    }
    hidden_section(&hidden);
    section("Expand", &report.expand);
}

fn changed_summary(report: &ChangedReport) {
    if !report.git_state.is_empty() {
        println!("\n## Git State\n");
        let rows = report
            .git_state
            .iter()
            .take(visible_git_state_count(report))
            .map(git_change_row)
            .collect();
        println!("{}", table(&["Status", "Path", "Old", "Staged", "Unstaged"], rows));
    }
    render_file_summaries("Changed Anchors", &report.changed);
}

fn visible_git_state_count(report: &ChangedReport) -> usize {
    report.display_limit.min(report.git_state.len())
}

fn changed_selector_suffix(selector: &str) -> String {
    if selector == "--changed" {
        String::new()
    } else {
        format!(" {selector}")
    }
}

fn git_change_row(change: &GitChange) -> Vec<String> {
    vec![
        change.status.clone(),
        code(&change.path),
        change
            .old_path
            .as_ref()
            .map(|path| code(path))
            .unwrap_or_else(|| "none".to_string()),
        change.staged.to_string(),
        change.unstaged.to_string(),
    ]
}

fn changed_delta_section(report: &ChangedReport) {
    let delta = &report.map_delta;
    println!("\n## Map Delta\n");
    println!(
        "{}",
        table(
            &["Surface", "Count"],
            vec![
                vec!["added imports/exports".to_string(), delta.added_edges.to_string()],
                vec!["removed imports/exports".to_string(), delta.removed_edges.to_string()],
                vec!["changed symbols".to_string(), delta.changed_symbols.to_string()],
                vec!["added exports".to_string(), delta.added_exports.to_string()],
                vec!["removed exports".to_string(), delta.removed_exports.to_string()],
                vec!["added runtime routes".to_string(), delta.added_runtime_routes.to_string()],
                vec!["removed runtime routes".to_string(), delta.removed_runtime_routes.to_string()],
                vec!["added env".to_string(), delta.added_env.to_string()],
                vec!["removed env".to_string(), delta.removed_env.to_string()],
                vec!["added proof sensors".to_string(), delta.added_proof_surfaces.to_string()],
                vec!["removed proof sensors".to_string(), delta.removed_proof_surfaces.to_string()],
                vec!["new unknowns".to_string(), delta.new_unknowns.to_string()],
            ],
        )
    );
}

fn changed_impact_section(report: &ChangedReport, compact: bool) {
    if report.impact.is_empty() {
        return;
    }
    println!("\n## Impact\n");
    if compact {
        let rows = report
            .impact
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
        return;
    }
    for cluster in &report.impact {
        println!("\n### `{}`", cluster.id);
        println!("risk: `{}`", cluster.risk);
        if !cluster.changed.is_empty() {
            println!("changed:");
            println!("{}", bullet(&cluster.changed, true, Some(10)));
        }
        if !cluster.reasons.is_empty() {
            println!("reasons:");
            println!("{}", bullet(&cluster.reasons, false, Some(6)));
        }
        grouped_edge_list("direct consumers", &cluster.direct_consumers, 8);
        grouped_edge_list("cross-boundary consumers", &cluster.cross_boundary_consumers, 8);
        grouped_edge_list("contract risks", &cluster.contract_risks, 8);
        if !cluster.proof.is_empty() {
            println!("proof: {} edges (see Proof section)", cluster.proof.len());
        }
    }
}

fn changed_proof_section(report: &ChangedReport) {
    println!("\n## Proof\n");
    if report.proof.commands.is_empty() && report.proof.fallback.is_empty() {
        println!("No proof command inferred.");
    }
    for command in &report.proof.commands {
        println!("\n### `{}`", command.command);
        if command.sensors.is_empty() {
            println!("- no sensor details");
        } else {
            for sensor in &command.sensors {
                let path = sensor.path.as_deref().unwrap_or("none");
                println!(
                    "- `{}` [{}; {}] {}",
                    path,
                    sensor.evidence,
                    format!("{:?}", sensor.strength).to_ascii_lowercase(),
                    proof_location_summary(&sensor.locations)
                );
            }
        }
        if command.hidden_count > 0 {
            println!("- hidden: {} sensors", command.hidden_count);
        }
    }
    if !report.proof.fallback.is_empty() {
        println!("\n### Fallback");
        println!("{}", code_block("bash", &report.proof.fallback));
    }
    println!("\n### Sensor Counts");
    println!(
        "{}",
        table(
            &["Kind", "Count"],
            vec![
                vec!["direct".to_string(), report.proof.direct.len().to_string()],
                vec!["indirect".to_string(), report.proof.indirect.len().to_string()],
                vec!["e2e".to_string(), report.proof.e2e.len().to_string()],
                vec!["contract".to_string(), report.proof.contract.len().to_string()],
                vec![
                    "missing_direct".to_string(),
                    report.proof.missing_direct.len().to_string(),
                ],
            ],
        )
    );
}

fn grouped_edge_list(title: &str, edges: &[StructuralEdge], limit: usize) {
    if edges.is_empty() {
        return;
    }
    println!("{title}:");
    let visible_count = edges.len().min(limit);
    let mut grouped: std::collections::BTreeMap<&str, Vec<&StructuralEdge>> =
        std::collections::BTreeMap::new();
    for edge in edges.iter().take(visible_count) {
        grouped.entry(edge.from.as_str()).or_default().push(edge);
    }
    for (from, edges) in grouped {
        println!("- `{from}`");
        for edge in edges {
            println!(
                "  - {} -> `{}` [{}; {}] {}",
                edge.edge_type,
                edge.to,
                edge.evidence,
                format!("{:?}", edge.strength).to_ascii_lowercase(),
                edge_location_summary(edge)
            );
        }
    }
    let hidden_count = edges.len().saturating_sub(visible_count);
    if hidden_count > 0 {
        println!("- hidden: {hidden_count} {title} edges");
    }
}
