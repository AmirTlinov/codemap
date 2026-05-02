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
        let visible_changes = report
            .git_state
            .iter()
            .take(visible_git_state_count(report))
            .collect::<Vec<_>>();
        let prefix = changed_common_dir_prefix(
            &visible_changes
                .iter()
                .map(|change| change.path.as_str())
                .collect::<Vec<_>>(),
        );
        if let Some(prefix) = &prefix {
            println!("prefix: `{prefix}`");
        }
        for change in visible_changes {
            let path = changed_relative_path(&change.path, prefix.as_deref());
            println!(
                "- `{}` [{}; staged={}; unstaged={}]",
                path, change.status, change.staged, change.unstaged
            );
            if let Some(old_path) = &change.old_path {
                println!("  old: `{old_path}`");
            }
        }
    }
    changed_anchor_section(&report.changed);
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

fn changed_anchor_section(files: &[crate::model::FileSummary]) {
    if files.is_empty() {
        return;
    }
    println!("\n## Changed Anchors\n");
    let prefix = changed_common_dir_prefix(
        &files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
    );
    if let Some(prefix) = &prefix {
        println!("prefix: `{prefix}`");
    }
    for file in files {
        let package = file.package.as_deref().unwrap_or("none");
        let path = changed_relative_path(&file.path, prefix.as_deref());
        println!(
            "- `{}` [{}; {}; package={}; lines={}; symbols={}; exports={}; imports={}; imported_by={}]",
            path,
            file.kind,
            file.language,
            package,
            file.lines,
            file.symbols.len(),
            file.exports.len(),
            file.imports.len(),
            file.imported_by_count
        );
        if !file.roles.is_empty() {
            println!("  roles: {}", file.roles.join(", "));
        }
        if !file.exports.is_empty() {
            println!("  exports: {}", changed_preview_list(&file.exports, 6));
        }
    }
}

fn changed_common_dir_prefix(paths: &[&str]) -> Option<String> {
    if paths.len() < 2 {
        return None;
    }
    let mut common = paths
        .first()?
        .split('/')
        .collect::<Vec<_>>();
    common.pop();
    for path in paths.iter().skip(1) {
        let mut segments = path.split('/').collect::<Vec<_>>();
        segments.pop();
        let len = common
            .iter()
            .zip(segments.iter())
            .take_while(|(left, right)| left == right)
            .count();
        common.truncate(len);
        if common.is_empty() {
            return None;
        }
    }
    Some(format!("{}/", common.join("/")))
}

fn changed_relative_path(path: &str, prefix: Option<&str>) -> String {
    prefix
        .and_then(|prefix| path.strip_prefix(prefix))
        .unwrap_or(path)
        .to_string()
}

fn changed_preview_list(values: &[String], limit: usize) -> String {
    let shown = values
        .iter()
        .take(limit)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = values.len().saturating_sub(limit);
    if hidden == 0 {
        shown
    } else {
        format!("{shown} +{hidden} hidden")
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
        changed_impact_summary_lines(&report.impact);
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

fn changed_impact_summary_lines(clusters: &[ImpactCluster]) {
    let paths = clusters
        .iter()
        .filter_map(|cluster| cluster.id.strip_prefix("changed:"))
        .collect::<Vec<_>>();
    let prefix = changed_common_dir_prefix(&paths);
    if let Some(prefix) = &prefix {
        println!("prefix: `{prefix}`");
    }
    for cluster in clusters {
        let label = cluster
            .id
            .strip_prefix("changed:")
            .map(|path| changed_relative_path(path, prefix.as_deref()))
            .unwrap_or_else(|| cluster.id.clone());
        println!(
            "- `{}` [risk={}; direct={}; cross={}; contract={}; proof={}]",
            label,
            cluster.risk,
            cluster.direct_consumers.len(),
            cluster.cross_boundary_consumers.len(),
            cluster.contract_risks.len(),
            cluster.proof.len()
        );
        if !cluster.reasons.is_empty() {
            println!("  reasons: {}", cluster.reasons.join("; "));
        }
    }
}

fn changed_proof_section(report: &ChangedReport) {
    println!("\n## Proof\n");
    if report.proof.commands.is_empty() && report.proof.fallback.is_empty() {
        println!("No proof command inferred.");
    }
    let mut grouped: std::collections::BTreeMap<String, (Vec<&ProofSurface>, usize)> =
        std::collections::BTreeMap::new();
    for command in &report.proof.commands {
        if command.sensors.is_empty() {
            grouped
                .entry(command.command.clone())
                .or_insert_with(|| (Vec::new(), 0))
                .1 += command.hidden_count;
            continue;
        }
        let mut command_groups = std::collections::BTreeSet::new();
        for sensor in &command.sensors {
            let key = proof_display_command(sensor);
            command_groups.insert(key.clone());
            grouped
                .entry(key)
                .or_insert_with(|| (Vec::new(), 0))
                .0
                .push(sensor);
        }
        if command_groups.len() == 1
            && let Some(key) = command_groups.first()
        {
            grouped
                .entry(key.clone())
                .or_insert_with(|| (Vec::new(), 0))
                .1 += command.hidden_count;
        }
    }
    for (command, (sensors, hidden_count)) in grouped {
        println!("\n### `{command}`");
        if sensors.is_empty() {
            println!("- no sensor details");
        } else {
            println!("- sensors: `{}`", sensors.len());
            proof_count_line("evidence", evidence_counts(&sensors));
            proof_count_line("strength", strength_counts(&sensors));
            let sample_limit = if sensors.len() <= 6 { sensors.len() } else { 5 };
            println!("- sample:");
            for sensor in sensors.iter().take(sample_limit) {
                let path = sensor.path.as_deref().unwrap_or("none");
                println!(
                    "  - `{}` [{}; {}] {}",
                    path,
                    sensor.evidence,
                    format!("{:?}", sensor.strength).to_ascii_lowercase(),
                    proof_location_summary(&sensor.locations)
                );
            }
            let hidden_details = sensors.len().saturating_sub(sample_limit);
            if hidden_details > 0 {
                println!("- hidden details: `{hidden_details}` sensors");
            }
        }
        if hidden_count > 0 {
            println!("- hidden: {hidden_count} sensors");
            println!(
                "  expand: `{}`",
                root_aware_expand(&format!(
                    "codemap proof-map --changed --raw-sensors --limit {}",
                    sensors.len() + hidden_count
                ))
            );
        }
    }
    if !report.proof.fallback.is_empty() {
        println!("\n### Fallback");
        println!("{}", code_block("bash", &report.proof.fallback));
    }
    println!("\n### Sensor Counts");
    for (kind, count) in [
        ("direct", report.proof.direct.len()),
        ("indirect", report.proof.indirect.len()),
        ("e2e", report.proof.e2e.len()),
        ("contract", report.proof.contract.len()),
        ("missing_direct", report.proof.missing_direct.len()),
    ] {
        println!("- {kind}: `{count}`");
    }
}
