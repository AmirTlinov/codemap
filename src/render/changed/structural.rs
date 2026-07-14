// Responsibility: render-changed-structural
use crate::model::ChangedReport;
use crate::render::{
    changed_preview_paths, changed_render_limit, changed_selector_suffix, proof_location_summary,
    public_evidence_label, root_aware_expand,
};

type ChangedStructuralEventGroupKey = (String, String, String, Option<String>, Option<String>);
type ChangedStructuralEventGroups<'a> = std::collections::BTreeMap<
    ChangedStructuralEventGroupKey,
    Vec<&'a crate::model::ChangedStructuralEvent>,
>;

pub(crate) fn changed_structural_events_section(report: &ChangedReport, compact: bool) {
    if report.structural_events.is_empty() {
        return;
    }
    if compact {
        changed_structural_events_compact_section(report);
        return;
    }
    println!("\nstructural events:");
    for events in changed_structural_event_groups(report)
        .values()
        .take(changed_render_limit(report, compact))
    {
        let Some(event) = events.first().copied() else {
            continue;
        };
        let count_hint = if events.len() > 1 {
            format!("; count={}", events.len())
        } else {
            String::new()
        };
        println!(
            "- `{}` [{}; evidence={}{}]",
            event.path,
            event.kind,
            public_evidence_label(&event.evidence),
            count_hint
        );
        if !event.locations.is_empty() {
            println!("  at: {}", proof_location_summary(&event.locations));
            let hidden_locations = events
                .iter()
                .map(|event| event.locations.len())
                .sum::<usize>()
                .saturating_sub(event.locations.len());
            if hidden_locations > 0 && !compact {
                println!("  hidden locations: `{hidden_locations}`");
            }
        }
        if let Some(old_path) = &event.old_path {
            println!("  old: `{old_path}`");
        }
        changed_structural_effects(events, compact);
        if let Some(expand) = &event.expand {
            println!("  expand: `{}`", root_aware_expand(expand));
        }
    }
}

type ChangedStructuralCompactKey = (String, String, String, Option<String>);

fn changed_structural_events_compact_section(report: &ChangedReport) {
    println!("\nstructural events:");
    let mut groups: std::collections::BTreeMap<
        ChangedStructuralCompactKey,
        Vec<&crate::model::ChangedStructuralEvent>,
    > = std::collections::BTreeMap::new();
    for event in &report.structural_events {
        groups
            .entry((
                event.kind.clone(),
                event.evidence.clone(),
                event.effect.clone(),
                event.old_path.clone(),
            ))
            .or_default()
            .push(event);
    }
    for ((kind, evidence, effect, old_path), events) in
        groups.iter().take(changed_render_limit(report, true))
    {
        let paths = events
            .iter()
            .map(|event| event.path.clone())
            .collect::<Vec<_>>();
        println!(
            "- `{kind}` [evidence={}; count={}]; sample: {}; effect: {}",
            public_evidence_label(evidence),
            events.len(),
            changed_preview_paths(&paths, 3),
            effect
        );
        if let Some(old_path) = old_path {
            println!("  old sample: `{old_path}`");
        }
    }
    let hidden = groups
        .len()
        .saturating_sub(changed_render_limit(report, true));
    if hidden > 0 {
        println!("- hidden structural event groups: `{hidden}`");
    }
    println!(
        "  expand: `{}`",
        root_aware_expand(&format!(
            "codemap changed{} --section observed --limit {}",
            changed_selector_suffix(&report.selector),
            report.structural_events.len()
        ))
    );
}

pub(crate) fn changed_structural_event_groups(
    report: &ChangedReport,
) -> ChangedStructuralEventGroups<'_> {
    let mut groups = std::collections::BTreeMap::new();
    for event in &report.structural_events {
        groups
            .entry((
                event.path.clone(),
                event.kind.clone(),
                event.evidence.clone(),
                event.old_path.clone(),
                event.expand.clone(),
            ))
            .or_insert_with(Vec::new)
            .push(event);
    }
    groups
}

fn changed_structural_effects(events: &[&crate::model::ChangedStructuralEvent], compact: bool) {
    let effects = events
        .iter()
        .map(|event| event.effect.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if effects.len() <= 1 {
        let effect = effects
            .first()
            .copied()
            .unwrap_or("changed structural fact");
        println!("  effect: {effect}");
        return;
    }
    if compact {
        let sample = effects
            .iter()
            .take(2)
            .copied()
            .collect::<Vec<_>>()
            .join("; ");
        let hidden = effects.len().saturating_sub(2);
        if hidden > 0 {
            println!("  effects: {sample}; +{hidden} hidden");
        } else {
            println!("  effects: {sample}");
        }
        return;
    }
    println!("  effects:");
    for effect in effects.iter().take(4) {
        println!("  - {effect}");
    }
    let hidden = effects.len().saturating_sub(4);
    if hidden > 0 {
        println!("  hidden effects: `{hidden}`");
    }
}
