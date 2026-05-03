pub fn changed(report: &ChangedReport, section_filter: Option<&str>) {
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
        if section_filter == Some("hidden") {
            changed_hidden_section(report, &changed_render_hidden(report, false), true, false);
            return;
        }
        if section_filter.is_some() {
            return;
        }
        section("Expand", &report.expand);
        return;
    }
    if section_filter == Some("hidden") {
        let hidden = changed_render_hidden(report, false);
        changed_hidden_section(report, &hidden, true, false);
        return;
    }
    let show_all = section_filter.is_none();
    let compact = show_all && changed_should_compact(report);
    let hidden = changed_render_hidden(report, compact);
    if matches!(section_filter, None | Some("observed")) {
        changed_observed_section(report, true, compact);
    }
    if matches!(section_filter, None | Some("roles")) {
        changed_roles_section(report, true, compact);
    }
    if matches!(section_filter, None | Some("links")) {
        changed_links_section(report, show_all, true);
    }
    if matches!(section_filter, None | Some("proof")) {
        changed_proof_section(report, compact);
    }
    if matches!(section_filter, None | Some("unknown")) {
        changed_unknown_section(report, true, compact);
    }
    if show_all {
        changed_hidden_section(report, &hidden, false, compact);
        section("Expand", &report.expand);
    }
}

const COMPACT_CHANGED_PROOF_COMMAND_LIMIT: usize = 3;

fn changed_render_hidden(report: &ChangedReport, compact: bool) -> Vec<crate::model::HiddenGroup> {
    let mut hidden = report.hidden.clone();
    let render_limit = changed_render_limit(report, compact);
    if report.git_state.len() > render_limit {
        hidden.push(crate::model::HiddenGroup {
            reason: "git state rows hidden by limit".to_string(),
            count: report.git_state.len() - render_limit,
            expand: format!(
                "codemap changed{} --section observed --limit {}",
                changed_selector_suffix(&report.selector),
                report.git_state.len()
            ),
        });
    }
    let structural_group_count = changed_structural_event_groups(report).len();
    if structural_group_count > render_limit {
        hidden.push(crate::model::HiddenGroup {
            reason: "structural event groups hidden by limit".to_string(),
            count: structural_group_count - render_limit,
            expand: format!(
                "codemap changed{} --section observed --limit {}",
                changed_selector_suffix(&report.selector),
                structural_group_count
            ),
        });
    }
    if compact {
        let proof_group_count = changed_proof_command_groups(report).len();
        if proof_group_count > COMPACT_CHANGED_PROOF_COMMAND_LIMIT {
            hidden.push(crate::model::HiddenGroup {
                reason: "proof command groups hidden by compact changed view".to_string(),
                count: proof_group_count - COMPACT_CHANGED_PROOF_COMMAND_LIMIT,
                expand: format!(
                    "codemap changed{} --section proof",
                    changed_selector_suffix(&report.selector)
                ),
            });
        }
    }
    hidden
}

fn changed_render_limit(report: &ChangedReport, compact: bool) -> usize {
    if compact && report.display_limit >= 30 {
        report.display_limit.min(5)
    } else {
        report.display_limit
    }
}

fn changed_should_compact(report: &ChangedReport) -> bool {
    report.display_limit >= 30 && (report.total_changed_count > 20 || report.changed.len() > 5)
}

fn changed_observed_section(report: &ChangedReport, force: bool, compact: bool) {
    if report.git_state.is_empty()
        && report.changed.is_empty()
        && report.structural_events.is_empty()
        && changed_map_delta_is_empty(&report.map_delta)
    {
        if force {
            println!("\n## Observed\n");
            println!("No observed changed surfaces.");
        }
        return;
    }
    println!("\n## Observed\n");
    if !report.git_state.is_empty() {
        println!("git state:");
        let visible_changes = report
            .git_state
            .iter()
            .take(visible_git_state_count(report, compact))
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
    changed_structural_events_section(report, compact);
    changed_anchor_section(report, compact);
    changed_delta_section(report, compact);
}

fn visible_git_state_count(report: &ChangedReport, compact: bool) -> usize {
    changed_render_limit(report, compact).min(report.git_state.len())
}

fn changed_selector_suffix(selector: &str) -> String {
    if selector == "--changed" {
        String::new()
    } else {
        format!(" {selector}")
    }
}

fn changed_anchor_section(report: &ChangedReport, compact: bool) {
    let files = &report.changed;
    if files.is_empty() {
        return;
    }
    println!("\nchanged anchors:");
    let prefix = changed_common_dir_prefix(
        &files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
    );
    if let Some(prefix) = &prefix {
        println!("prefix: `{prefix}`");
    }
    let limit = changed_render_limit(report, compact);
    for file in files.iter().take(limit) {
        let package = file.package.as_deref().unwrap_or("none");
        let path = changed_relative_path(&file.path, prefix.as_deref());
        if compact {
            let role_hint = if file.roles.is_empty() {
                String::new()
            } else {
                format!("; hints={}", changed_preview_list(&file.roles, 3))
            };
            println!("- `{path}` [{}; {}{}]", file.kind, file.language, role_hint);
        } else {
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
        }
        if !compact && !file.roles.is_empty() {
            println!("  surface hints: {}", file.roles.join(", "));
        }
        if !compact && !file.exports.is_empty() {
            println!("  exports: {}", changed_preview_list(&file.exports, 6));
        }
    }
    let hidden = files.len().saturating_sub(limit);
    if hidden > 0 {
        println!("- hidden changed anchors: `{hidden}`");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section observed --limit {}",
                changed_selector_suffix(&report.selector),
                report.total_changed_count
            ))
        );
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

fn changed_delta_section(report: &ChangedReport, compact: bool) {
    let delta = &report.map_delta;
    println!("\nmap delta:");
    let entries = [
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
    ];
    let mut printed = 0;
    for (label, count) in entries {
        if compact && count == 0 {
            continue;
        }
        println!("- {label}: `{count}`");
        printed += 1;
    }
    if compact && printed == 0 {
        println!("- no structural delta detected");
    }
}

fn changed_map_delta_is_empty(delta: &crate::model::ChangedMapDelta) -> bool {
    delta.added_edges == 0
        && delta.removed_edges == 0
        && delta.changed_symbols == 0
        && delta.added_exports == 0
        && delta.removed_exports == 0
        && delta.added_runtime_routes == 0
        && delta.removed_runtime_routes == 0
        && delta.added_env == 0
        && delta.removed_env == 0
        && delta.added_proof_surfaces == 0
        && delta.removed_proof_surfaces == 0
        && delta.new_unknowns == 0
}
