// Responsibility: changed-proof-group-details
use crate::model::ProofSurface;
use crate::render::{
    COMPACT_CHANGED_PROOF_COMMAND_LIMIT, changed_selector_suffix, evidence_counts,
    proof_count_line, proof_location_summary, proof_target_suffix, public_evidence_label,
    root_aware_expand, strength_counts,
};

pub(crate) fn changed_proof_render_groups(
    grouped: &[(&String, &(Vec<&ProofSurface>, usize))],
    compact: bool,
    selector: &str,
) {
    let visible_group_limit = if compact {
        COMPACT_CHANGED_PROOF_COMMAND_LIMIT
    } else {
        usize::MAX
    };
    let total_group_count = grouped.len();
    let visible_group_count = total_group_count.min(visible_group_limit);
    for (command, (sensors, hidden_count)) in grouped.iter().take(visible_group_count) {
        println!("\n### `{command}`");
        changed_proof_command_group_details(sensors, *hidden_count, compact, selector);
    }
    if compact && total_group_count > visible_group_count {
        let hidden_command_groups = total_group_count - visible_group_count;
        println!("\n- hidden runnable command surface groups: `{hidden_command_groups}`");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section proof",
                changed_selector_suffix(selector)
            ))
        );
    }
}

pub(crate) fn changed_proof_command_group_details(
    sensors: &[&ProofSurface],
    hidden_count: usize,
    compact: bool,
    selector: &str,
) {
    if sensors.is_empty() {
        println!("- no sensor details");
    } else if compact
        && sensors
            .iter()
            .all(|sensor| crate::proof_classification::proof_surface_is_soft_evidence(sensor))
    {
        changed_proof_soft_group_summary(sensors);
    } else {
        println!("- sensors: `{}`", sensors.len());
        changed_proof_class_line(sensors);
        proof_count_line("evidence", evidence_counts(sensors));
        proof_count_line("strength", strength_counts(sensors));
        changed_proof_samples(sensors, compact);
    }
    if hidden_count > 0 {
        println!("- hidden: {hidden_count} sensors");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap proof-map {selector} --raw-sensors --limit {}",
                sensors.len() + hidden_count
            ))
        );
    }
}

fn changed_proof_soft_group_summary(sensors: &[&ProofSurface]) {
    println!(
        "- sensors: `{}`; evidence: {}; strength: {}",
        sensors.len(),
        changed_proof_inline_counts(evidence_counts(sensors)),
        changed_proof_inline_counts(strength_counts(sensors))
    );
    if let Some(sensor) = sensors.first() {
        let path = sensor.path.as_deref().unwrap_or("none");
        println!(
            "- sample: `{path}` [{}; {}] {}",
            public_evidence_label(&sensor.evidence),
            format!("{:?}", sensor.strength).to_ascii_lowercase(),
            proof_location_summary(&sensor.locations)
        );
    }
}

fn changed_proof_inline_counts(counts: Vec<(String, usize)>) -> String {
    counts
        .into_iter()
        .map(|(kind, count)| format!("`{kind}: {count}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn changed_proof_class_line(sensors: &[&ProofSurface]) {
    let runnable = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_runnable_validation(sensor))
        .count();
    let evidence = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_evidence_only(sensor))
        .count();
    let setup = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_setup_or_support(sensor))
        .count();
    let soft = sensors
        .iter()
        .filter(|sensor| crate::proof_classification::proof_surface_is_soft_evidence(sensor))
        .count();
    if evidence > 0 || setup > 0 || soft > 0 {
        println!(
            "- surface class: `runnable: {runnable}`, `linked: {evidence}`, `setup_support: {setup}`, `soft_match: {soft}`"
        );
    }
}

fn changed_proof_samples(sensors: &[&ProofSurface], compact: bool) {
    let sample_limit = if compact {
        sensors.len().min(3)
    } else if sensors.len() <= 6 {
        sensors.len()
    } else {
        5
    };
    println!("- sample:");
    for sensor in sensors.iter().take(sample_limit) {
        let path = sensor.path.as_deref().unwrap_or("none");
        println!(
            "  - `{}`{} [{}; {}] {}",
            path,
            proof_target_suffix(sensor),
            public_evidence_label(&sensor.evidence),
            format!("{:?}", sensor.strength).to_ascii_lowercase(),
            proof_location_summary(&sensor.locations)
        );
    }
    let hidden_details = sensors.len().saturating_sub(sample_limit);
    if hidden_details > 0 {
        println!("- hidden details: `{hidden_details}` sensors");
    }
}
