// Responsibility: render-observation-visibility
use crate::model::{ConeReport, CoverageClosure, CoverageHorizon, ObservationLedger};
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
    let compact_xray = horizons
        .iter()
        .filter(|horizon| horizon.group.starts_with("xray_"))
        .count()
        == ConeReport::XRAY_GROUPS.len();
    for horizon in horizons
        .iter()
        .copied()
        .filter(|horizon| !compact_xray || !horizon.group.starts_with("xray_"))
    {
        render_visibility_horizon(horizon);
    }
    if compact_xray {
        render_compact_xray_visibility(
            &horizons
                .into_iter()
                .filter(|horizon| horizon.group.starts_with("xray_"))
                .collect::<Vec<_>>(),
        );
    }
}

fn render_compact_xray_visibility(horizons: &[&CoverageHorizon]) {
    let limited = horizons
        .iter()
        .copied()
        .filter(|horizon| horizon.hidden > 0)
        .collect::<Vec<_>>();
    for horizon in &limited {
        render_visibility_horizon_row(horizon);
    }
    let families = |closure| {
        horizons
            .iter()
            .filter(|horizon| horizon.count.closure == closure)
            .map(|horizon| xray_group_family(&horizon.group))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join("/")
    };
    let open = families(CoverageClosure::Open);
    let unavailable = families(CoverageClosure::Unavailable);
    let reasons = horizons
        .iter()
        .filter(|horizon| horizon.count.closure != CoverageClosure::Closed)
        .flat_map(|horizon| horizon.count.reasons.iter().map(|reason| reason.label()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "- xray ledger: certified={}; fully-shown={}; open={}; unavailable={}{}",
        horizons.len(),
        horizons.len() - limited.len(),
        if open.is_empty() { "0" } else { &open },
        if unavailable.is_empty() {
            "0"
        } else {
            &unavailable
        },
        if reasons.is_empty() {
            String::new()
        } else {
            format!("; gaps={reasons}")
        }
    );
    let mut expands = std::collections::BTreeSet::new();
    for horizon in limited {
        if let Some(expand) = horizon.expand.as_deref()
            && expands.insert(expand)
        {
            println!("  expand: `{}`", root_aware_expand(expand));
        }
    }
}

fn xray_group_family(group: &str) -> &str {
    match group {
        "xray_direct_consumers" | "xray_mediated_consumers" => "consumers",
        group if group.starts_with("xray_proof_") => "proof",
        group => group.strip_prefix("xray_").unwrap_or(group),
    }
}

pub(crate) fn render_runtime_visibility(observations: &ObservationLedger) {
    const GROUP_ORDER: [&str; 8] = [
        "entrypoints",
        "routes",
        "scripts",
        "env",
        "workers",
        "ci",
        "proof",
        "unknowns",
    ];
    if observations.horizons.is_empty() {
        return;
    }
    println!("\n## Visibility\n");
    for group in GROUP_ORDER {
        let Some(horizon) = observations
            .horizons
            .iter()
            .find(|horizon| horizon.group == group)
        else {
            continue;
        };
        if group == "routes" {
            let unsupported_files = horizon
                .unsupported
                .iter()
                .map(|observation| observation.file.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            println!(
                "- routes: {}; shown={} hidden={}; dynamic={} unsupported_files={}; cert=`{}`",
                horizon.count.display(),
                horizon.shown,
                horizon.hidden,
                horizon.dynamic.len(),
                unsupported_files,
                readable_certificate_id(&horizon.count.certificate_id)
            );
        } else {
            render_visibility_horizon_row(horizon);
        }
        if horizon.hidden > 0
            && let Some(expand) = horizon.expand.as_deref()
        {
            println!("  expand: `{}`", root_aware_expand(expand));
        }
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
