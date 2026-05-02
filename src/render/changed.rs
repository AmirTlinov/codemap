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
        for change in report.git_state.iter().take(visible_git_state_count(report)) {
            println!(
                "- `{}` [{}; staged={}; unstaged={}]",
                change.path, change.status, change.staged, change.unstaged
            );
            if let Some(old_path) = &change.old_path {
                println!("  old: `{old_path}`");
            }
        }
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

fn changed_delta_section(report: &ChangedReport) {
    let delta = &report.map_delta;
    println!("\n## Map Delta\n");
    for (label, count) in [
        ("added imports/exports", delta.added_edges),
        ("removed imports/exports", delta.removed_edges),
        ("changed symbols", delta.changed_symbols),
        ("added exports", delta.added_exports),
        ("removed exports", delta.removed_exports),
        ("added runtime routes", delta.added_runtime_routes),
        ("removed runtime routes", delta.removed_runtime_routes),
        ("added env", delta.added_env),
        ("removed env", delta.removed_env),
        ("added proof sensors", delta.added_proof_surfaces),
        ("removed proof sensors", delta.removed_proof_surfaces),
        ("new unknowns", delta.new_unknowns),
    ] {
        println!("- {label}: `{count}`");
    }
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
