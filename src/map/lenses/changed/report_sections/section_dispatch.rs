// Responsibility: changed-section-dispatch
use crate::map::{
    boundary_facts_for_changed, changed_coupling, changed_diff_structural_events,
    changed_fail_open_unknowns, changed_map_delta_from_diff, changed_proof_summary,
    changed_report_shell, changed_risks, changed_section_paths, changed_self_selector_suffix,
    changed_structural_events, current_session_snapshot, dedupe_unknowns, diff_map_report,
    empty_proof_map_report, file_summary, impact_report, missing_file_summary, prefix_hidden,
    proof_map_report, sort_changed_structural_events, truncate_with_hidden,
};
use crate::model::{ChangedReport, FileSummary, GitChange, HiddenGroup, Project};
use crate::repo;
use std::collections::BTreeSet;

pub fn changed_report_for_section(
    project: &Project,
    changed: Vec<String>,
    selector: String,
    context: crate::map::ChangedDiffContext,
    git_state: Vec<GitChange>,
    limit: usize,
    section: &str,
) -> ChangedReport {
    let crate::map::ChangedDiffContext {
        mode,
        mut selection,
    } = context;
    let limit = limit.max(1);
    let total_changed_count = changed
        .iter()
        .map(|file| repo::normalize_rel_path(file))
        .filter(|file| file != ".")
        .collect::<BTreeSet<_>>()
        .len();
    selection.selected_files = total_changed_count;
    let changed_paths = changed
        .iter()
        .map(|file| repo::normalize_rel_path(file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let section_paths = changed_section_paths(project, &changed_paths, limit);
    let mut report = changed_report_shell(
        &selector,
        limit,
        total_changed_count,
        git_state.clone(),
        current_session_snapshot(project),
        selection,
    );
    report.changed = changed_file_summaries(
        project,
        &changed_paths,
        &selector,
        section,
        limit,
        &mut report.hidden,
    );
    if total_changed_count == 0 && git_state.is_empty() {
        return report;
    }

    match section {
        "roles" => report,
        "links" => {
            let impact = impact_report(project, section_paths.clone(), selector.clone(), 1, limit);
            let proof_map = proof_map_report(
                project,
                None,
                changed_paths.clone(),
                selector.clone(),
                limit,
                false,
            );
            report.changed = impact.changed;
            report.impact = impact.clusters;
            report
                .hidden
                .extend(prefix_hidden("links", &impact.hidden, &selector, limit));
            report.coupling = changed_coupling(
                project,
                &section_paths,
                &changed_paths,
                &proof_map,
                &selector,
            );
            report.proof_map_cache = Some(Box::new(proof_map));
            report
        }
        "proof" => {
            let proof_map = proof_map_report(
                project,
                None,
                changed_paths.clone(),
                selector.clone(),
                limit,
                false,
            );
            report
                .unknowns
                .extend(changed_fail_open_unknowns(project, &section_paths));
            dedupe_unknowns(&mut report.unknowns);
            report.proof = changed_proof_summary(proof_map.clone(), limit);
            report
                .hidden
                .extend(prefix_hidden("proof", &proof_map.hidden, &selector, limit));
            report.proof_map_cache = Some(Box::new(proof_map));
            report
        }
        "unknown" => {
            let diff = diff_map_report(
                project,
                section_paths.clone(),
                selector.clone(),
                limit,
                mode,
            );
            let proof_map = proof_map_report(
                project,
                None,
                changed_paths.clone(),
                selector.clone(),
                limit,
                false,
            );
            report.changed = diff.changed;
            report
                .hidden
                .extend(prefix_hidden("observed", &diff.hidden, &selector, limit));
            report
                .hidden
                .extend(prefix_hidden("proof", &proof_map.hidden, &selector, limit));
            report.unknowns.extend(diff.new_unknowns);
            report.unknowns.extend(proof_map.unknowns.clone());
            report
                .unknowns
                .extend(changed_fail_open_unknowns(project, &section_paths));
            dedupe_unknowns(&mut report.unknowns);
            let unknown_expand = format!(
                "codemap changed{} --section unknown --limit {}",
                changed_self_selector_suffix(&selector),
                report.unknowns.len()
            );
            truncate_with_hidden(
                &mut report.unknowns,
                limit,
                &mut report.hidden,
                "changed unknowns hidden by limit",
                &unknown_expand,
            );
            report.proof_map_cache = Some(Box::new(proof_map));
            report
        }
        _ => {
            let mut structural_events = changed_structural_events(&git_state, &selector);
            structural_events.extend(changed_diff_structural_events(
                project,
                &section_paths,
                &mode,
            ));
            sort_changed_structural_events(&mut structural_events);
            let diff = diff_map_report(
                project,
                section_paths.clone(),
                selector.clone(),
                limit,
                mode,
            );
            let proof_map = empty_proof_map_report(selector.clone(), changed_paths.clone());
            report.structural_events = structural_events;
            report.map_delta = changed_map_delta_from_diff(&diff);
            report.changed = diff.changed;
            report.risks = changed_risks(
                project,
                &section_paths,
                &changed_paths,
                &git_state,
                &proof_map,
                &selector,
            );
            report.boundary_facts = boundary_facts_for_changed(project, &section_paths);
            report
                .hidden
                .extend(prefix_hidden("observed", &diff.hidden, &selector, limit));
            report
        }
    }
}

fn changed_file_summaries(
    project: &Project,
    changed_paths: &[String],
    selector: &str,
    section: &str,
    limit: usize,
    hidden: &mut Vec<HiddenGroup>,
) -> Vec<FileSummary> {
    let mut summaries = changed_paths
        .iter()
        .map(|rel| {
            project
                .files
                .get(rel)
                .map(|file| file_summary(project, file, false, 12))
                .unwrap_or_else(|| missing_file_summary(project, rel))
        })
        .collect::<Vec<_>>();
    let expand = format!(
        "codemap changed{} --section {section} --limit {}",
        changed_self_selector_suffix(selector),
        changed_paths.len()
    );
    truncate_with_hidden(
        &mut summaries,
        limit,
        hidden,
        "changed file summaries hidden by limit",
        &expand,
    );
    summaries
}
