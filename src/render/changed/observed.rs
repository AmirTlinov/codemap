// Responsibility: changed-observed-section
use crate::model::ChangedReport;
use crate::render::{
    changed_common_dir_prefix, changed_preview_list, changed_relative_path, changed_render_limit,
    changed_selector_suffix, changed_structural_events_section, root_aware_expand,
};

pub(crate) fn changed_observed_section(report: &ChangedReport, force: bool, compact: bool) {
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
                file.imported_by.display()
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
        ("added verification sensors", delta.added_proof_surfaces),
        ("removed verification sensors", delta.removed_proof_surfaces),
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
