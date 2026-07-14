// Responsibility: changed-lens-entry
use crate::model::ChangedReport;
use crate::render::{
    boundary_facts_section, changed_compact_expand_section, changed_coupling_section,
    changed_hidden_section, changed_links_section, changed_observed_section, changed_proof_section,
    changed_render_hidden, changed_risks_section, changed_roles_section, changed_should_compact,
    changed_unknown_section, changed_worktree_section, map_prelude_block_or_snapshot_line, section,
};

pub fn changed(report: &ChangedReport, section_filter: Option<&str>) {
    println!("# Changed Map\n");
    map_prelude_block_or_snapshot_line();
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
        if matches!(section_filter, None | Some("observed")) {
            changed_worktree_section(report, false);
        }
        println!("\nNo changed anchors detected.");
        // Surface fail-open notices (e.g. snapshot_not_found) even when the changed
        // set is empty, so a missing `--since` snapshot is never silent.
        if matches!(section_filter, None | Some("unknown")) {
            changed_unknown_section(report, true, false);
        }
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
        changed_worktree_section(report, compact);
        boundary_facts_section(&report.boundary_facts, false, compact);
    }
    if matches!(section_filter, None | Some("roles")) {
        changed_roles_section(report, true, compact);
    }
    if matches!(section_filter, None | Some("links")) {
        changed_coupling_section(report, true, compact);
    }
    if matches!(section_filter, None | Some("observed")) {
        changed_risks_section(report, true, compact);
        changed_observed_section(report, true, compact);
    }
    if matches!(section_filter, None | Some("links")) {
        changed_links_section(report, show_all, true);
    }
    if matches!(section_filter, None | Some("proof")) {
        changed_proof_section(report, compact, section_filter == Some("proof"));
    }
    if matches!(section_filter, None | Some("unknown")) {
        changed_unknown_section(report, true, compact);
    }
    if !show_all {
        changed_hidden_section(report, &hidden, false, false);
    }
    if show_all {
        changed_hidden_section(report, &hidden, false, compact);
        if compact {
            changed_compact_expand_section(report);
        } else {
            section("Expand", &report.expand);
        }
    }
}
