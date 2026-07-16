// Responsibility: changed-risk-coupling-facts
use crate::model::ChangedReport;
use crate::render::{
    changed_preview_paths, changed_render_limit, changed_selector_suffix, current_map_prelude,
    disclaimer, root_aware_expand,
};

pub(crate) fn changed_risks_section(report: &ChangedReport, force: bool, compact: bool) {
    let has_live_untracked = current_map_prelude()
        .is_some_and(|prelude| prelude.worktree.untracked > 0)
        && !report
            .risks
            .iter()
            .any(|risk| risk.kind == "untracked_files_present");
    let has_live_conflicts = current_map_prelude()
        .is_some_and(|prelude| prelude.worktree.conflicted > 0)
        && !report
            .risks
            .iter()
            .any(|risk| risk.kind == "conflicts_present");
    if report.risks.is_empty() && !has_live_untracked && !has_live_conflicts {
        if force {
            println!("\n## Risks\n");
            println!("No mechanical changed risks found.");
        }
        return;
    }
    if compact && report.total_changed_count > 20 {
        changed_risk_summary(report, has_live_untracked, has_live_conflicts);
        return;
    }
    println!("\n## Risks\n");
    disclaimer("Mechanical facts only. Not an edit verdict.");
    if let Some(prelude) = current_map_prelude() {
        if has_live_untracked {
            println!(
                "- `untracked_files_present` [low; count={}]",
                prelude.worktree.untracked
            );
            println!("  effect: untracked paths exist in the current worktree");
        }
        if has_live_conflicts {
            println!(
                "- `conflicts_present` [high; count={}]",
                prelude.worktree.conflicted
            );
            println!("  effect: conflicted paths exist in the current worktree");
        }
    }
    let limit = changed_render_limit(report, compact);
    for risk in report.risks.iter().take(limit) {
        changed_risk_line(risk, compact);
    }
    let hidden = report.risks.len().saturating_sub(limit);
    if hidden > 0 {
        println!("- hidden risk groups: `{hidden}`");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section observed --limit {}",
                changed_selector_suffix(&report.selector),
                report.risks.len()
            ))
        );
    }
}

fn changed_risk_summary(
    report: &ChangedReport,
    has_live_untracked: bool,
    has_live_conflicts: bool,
) {
    let mut counts = std::collections::BTreeMap::new();
    for risk in &report.risks {
        *counts.entry(risk.severity.as_str()).or_insert(0usize) += 1;
    }
    if has_live_untracked {
        *counts.entry("low").or_insert(0) += 1;
    }
    if has_live_conflicts {
        *counts.entry("high").or_insert(0) += 1;
    }
    let summary = counts
        .into_iter()
        .map(|(severity, count)| format!("{severity}={count}"))
        .collect::<Vec<_>>()
        .join("; ");
    let groups =
        report.risks.len() + usize::from(has_live_untracked) + usize::from(has_live_conflicts);
    println!("\n## Risks\n");
    println!(
        "- mechanical groups: `{groups}` [{summary}]; expand: `{}`",
        root_aware_expand(&format!(
            "codemap changed{} --section observed",
            changed_selector_suffix(&report.selector)
        ))
    );
}

fn changed_risk_line(risk: &crate::model::ChangedRisk, compact: bool) {
    if compact {
        println!(
            "- `{}` [{}; count={}]; sample: {}",
            risk.kind,
            risk.severity,
            risk.count,
            changed_preview_paths(&risk.paths, 3)
        );
        return;
    }
    println!(
        "- `{}` [{}; count={}]",
        risk.kind, risk.severity, risk.count
    );
    if !risk.paths.is_empty() {
        println!("  paths: {}", changed_preview_paths(&risk.paths, 8));
    }
    println!("  effect: {}", risk.effect);
    if let Some(expand) = &risk.expand {
        println!("  expand: `{}`", root_aware_expand(expand));
    }
}

pub(crate) fn changed_coupling_section(report: &ChangedReport, force: bool, compact: bool) {
    if report.coupling.is_empty() {
        if force {
            println!("\n## Coupling\n");
            println!("No deterministic coupling facts found.");
        }
        return;
    }
    if compact && report.total_changed_count > 20 {
        changed_coupling_summary(report);
        return;
    }
    println!("\n## Coupling\n");
    println!("Deterministic relationship facts only.\n");
    let limit = changed_render_limit(report, compact);
    for fact in report.coupling.iter().take(limit) {
        if compact {
            let sample = if fact.paths.is_empty() {
                String::new()
            } else {
                format!("; sample: {}", changed_preview_paths(&fact.paths, 3))
            };
            println!("- `{}` [{}]{sample}", fact.kind, fact.status);
            continue;
        }
        println!("- `{}` [{}]", fact.kind, fact.status);
        if !fact.paths.is_empty() {
            println!("  paths: {}", changed_preview_paths(&fact.paths, 8));
        }
        println!("  effect: {}", fact.effect);
        if let Some(expand) = &fact.expand {
            println!("  expand: `{}`", root_aware_expand(expand));
        }
    }
    let hidden = report.coupling.len().saturating_sub(limit);
    if hidden > 0 {
        println!("- hidden coupling facts: `{hidden}`");
        println!(
            "  expand: `{}`",
            root_aware_expand(&format!(
                "codemap changed{} --section links --limit {}",
                changed_selector_suffix(&report.selector),
                report.coupling.len()
            ))
        );
    }
}

fn changed_coupling_summary(report: &ChangedReport) {
    let mut counts = std::collections::BTreeMap::new();
    for fact in &report.coupling {
        *counts.entry(fact.status.as_str()).or_insert(0usize) += 1;
    }
    let summary = counts
        .into_iter()
        .map(|(status, count)| format!("{status}={count}"))
        .collect::<Vec<_>>()
        .join("; ");
    println!("\n## Coupling\n");
    println!(
        "- deterministic groups: `{}` [{summary}]; expand: `{}`",
        report.coupling.len(),
        root_aware_expand(&format!(
            "codemap changed{} --section links",
            changed_selector_suffix(&report.selector)
        ))
    );
}
