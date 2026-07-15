// Responsibility: render-observation-visibility
use crate::model::{CoverageHorizon, ObservationLedger};
use crate::render::root_aware_expand;

pub(crate) fn render_visibility_section(observations: &ObservationLedger) {
    render_visibility_section_for_groups(observations, &[]);
}

pub(crate) fn render_visibility_section_for_groups(
    observations: &ObservationLedger,
    groups: &[&str],
) {
    let horizons = observations
        .horizons
        .iter()
        .filter(|horizon| groups.is_empty() || groups.contains(&horizon.group.as_str()))
        .collect::<Vec<_>>();
    if horizons.is_empty() {
        return;
    }
    println!("\n## Visibility\n");
    for horizon in horizons {
        render_visibility_horizon(horizon);
    }
}

pub(crate) fn render_definition_visibility(observations: &ObservationLedger) {
    if observations.horizons.is_empty() {
        return;
    }
    println!("Visibility:");
    render_visibility_rows(observations);
}

pub(crate) fn render_definition_visibility_compact(
    observations: &ObservationLedger,
    include_expand: bool,
) {
    if observations.horizons.is_empty() {
        return;
    }
    println!("Visibility:");
    let mut rendered_expands = std::collections::BTreeSet::new();
    for horizon in &observations.horizons {
        render_visibility_horizon_row(horizon);
        if include_expand
            && horizon.hidden > 0
            && let Some(expand) = horizon.expand.as_deref()
            && rendered_expands.insert(expand)
        {
            println!("  expand: `{}`", root_aware_expand(expand));
        }
    }
}

fn render_visibility_rows(observations: &ObservationLedger) {
    for horizon in &observations.horizons {
        render_visibility_horizon(horizon);
    }
}

fn render_visibility_horizon(horizon: &CoverageHorizon) {
    render_visibility_horizon_row(horizon);
    if horizon.hidden > 0
        && let Some(expand) = horizon.expand.as_deref()
    {
        println!("  expand: `{}`", root_aware_expand(expand));
    }
}

fn render_visibility_horizon_row(horizon: &CoverageHorizon) {
    println!(
        "- {}: {}; shown={} hidden={}; cert=`{}`",
        horizon.group,
        horizon.count.display(),
        horizon.shown,
        horizon.hidden,
        readable_certificate_id(&horizon.count.certificate_id)
    );
}

pub(crate) fn readable_certificate_id(id: &str) -> String {
    const DIGEST_PREVIEW: usize = 12;
    let Some(digest) = id.strip_prefix("coverage-v1:") else {
        return id.to_string();
    };
    if digest.chars().count() <= DIGEST_PREVIEW {
        return id.to_string();
    }
    format!(
        "v1:{}",
        digest.chars().take(DIGEST_PREVIEW).collect::<String>()
    )
}
