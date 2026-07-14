// Responsibility: changed-proof-evidence-only
use crate::model::{ChangedReport, ProofSurface};
use crate::render::{
    COMPACT_CHANGED_PROOF_COMMAND_LIMIT, changed_proof_command_group_details,
    changed_selector_suffix, public_evidence_label, root_aware_expand,
};

pub(crate) fn changed_proof_evidence_only_surfaces(report: &ChangedReport) -> Vec<&ProofSurface> {
    let mut seen = std::collections::BTreeSet::new();
    let mut surfaces = Vec::new();
    for sensor in report
        .proof
        .hard
        .iter()
        .chain(report.proof.direct_evidence.iter())
        .chain(report.proof.mediated_evidence.iter())
        .chain(report.proof.soft_evidence.iter())
        .chain(report.proof.setup_support.iter())
    {
        if !crate::proof_classification::proof_surface_is_evidence_only(sensor) {
            continue;
        }
        let location_key = sensor
            .locations
            .iter()
            .map(|location| {
                format!(
                    "{}:{}:{}:{}",
                    location.path,
                    location.line_start.unwrap_or_default(),
                    location.line_end.unwrap_or_default(),
                    location.kind
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        let key = (
            sensor.path.clone(),
            sensor.evidence.clone(),
            sensor.reason.clone(),
            location_key,
        );
        if seen.insert(key) {
            surfaces.push(sensor);
        }
    }
    surfaces.sort_by(|a, b| {
        a.evidence
            .cmp(&b.evidence)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.reason.cmp(&b.reason))
    });
    surfaces
}

pub(crate) fn changed_proof_render_evidence_surfaces(
    surfaces: &[&ProofSurface],
    compact: bool,
    selector: &str,
) {
    let visible_group_limit = if compact {
        COMPACT_CHANGED_PROOF_COMMAND_LIMIT
    } else {
        usize::MAX
    };
    let mut grouped: std::collections::BTreeMap<String, Vec<&ProofSurface>> =
        std::collections::BTreeMap::new();
    for surface in surfaces {
        grouped
            .entry(public_evidence_label(&surface.evidence))
            .or_default()
            .push(*surface);
    }
    let total_group_count = grouped.len();
    let visible_group_count = total_group_count.min(visible_group_limit);
    for (evidence, sensors) in grouped.iter().take(visible_group_count) {
        println!("\n### `{evidence}`");
        changed_proof_command_group_details(sensors, 0, compact, selector);
    }
    if compact && total_group_count > visible_group_count {
        let hidden_groups = total_group_count - visible_group_count;
        println!("\n- hidden evidence groups: `{hidden_groups}`");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section proof",
                changed_selector_suffix(selector)
            ))
        );
    }
}
