// Responsibility: changed-compaction-policy
use crate::model::ChangedReport;
use crate::render::{
    changed_proof_command_groups, changed_proof_evidence_only_surfaces,
    changed_proof_surface_groups, changed_selector_suffix, changed_structural_event_groups,
};

pub(crate) const COMPACT_CHANGED_PROOF_COMMAND_LIMIT: usize = 3;

pub(crate) fn changed_render_hidden(
    report: &ChangedReport,
    compact: bool,
) -> Vec<crate::model::HiddenGroup> {
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
                reason: "runnable command surface groups hidden by compact changed view"
                    .to_string(),
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

pub(crate) fn changed_render_limit(report: &ChangedReport, compact: bool) -> usize {
    if compact && report.display_limit >= 30 {
        report.display_limit.min(5)
    } else {
        report.display_limit
    }
}

pub(crate) fn changed_should_compact(report: &ChangedReport) -> bool {
    let setup_group_count = changed_proof_surface_groups(report.proof.setup_support.iter()).len();
    let soft_group_count = changed_proof_surface_groups(report.proof.soft_evidence.iter()).len();
    let evidence_only_count = changed_proof_evidence_only_surfaces(report).len();
    report.display_limit >= 30
        && (report.total_changed_count > 1
            || report.changed.len() > 5
            || changed_proof_command_groups(report).len() > COMPACT_CHANGED_PROOF_COMMAND_LIMIT
            || setup_group_count > COMPACT_CHANGED_PROOF_COMMAND_LIMIT
            || soft_group_count > COMPACT_CHANGED_PROOF_COMMAND_LIMIT
            || evidence_only_count > COMPACT_CHANGED_PROOF_COMMAND_LIMIT
            || report.unknowns.len() > 5
            || report
                .hidden
                .iter()
                .any(|group| group.reason.contains("verification wiring") && group.count > 50)
            || report.unknowns.len() > report.display_limit)
}
