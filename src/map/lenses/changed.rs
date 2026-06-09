pub fn changed_report(
    project: &Project,
    changed: Vec<String>,
    selector: String,
    mode: DiffMapMode,
    git_state: Vec<GitChange>,
    limit: usize,
    proof_cache_limit: usize,
) -> ChangedReport {
    let limit = limit.max(1);
    let total_changed_count = changed
        .iter()
        .map(|file| repo::normalize_rel_path(file))
        .filter(|file| file != ".")
        .collect::<BTreeSet<_>>()
        .len();
    let changed_paths = changed
        .iter()
        .map(|file| repo::normalize_rel_path(file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let mut structural_events = changed_structural_events(&git_state, &selector);
    structural_events.extend(changed_diff_structural_events(project, &changed_paths, &mode));
    sort_changed_structural_events(&mut structural_events);
    if total_changed_count == 0 && git_state.is_empty() {
        return ChangedReport {
            kind: "changed_report",
            schema_version: "8",
            selector: selector.clone(),
            display_limit: limit,
            proof_plan_cache: None,
            proof_map_cache: None,
            total_changed_count,
            changed: Vec::new(),
            git_state,
            structural_events,
            map_delta: ChangedMapDelta {
                added_edges: 0,
                removed_edges: 0,
                changed_symbols: 0,
                added_exports: 0,
                removed_exports: 0,
                added_runtime_routes: 0,
                removed_runtime_routes: 0,
                added_env: 0,
                removed_env: 0,
                added_proof_surfaces: 0,
                removed_proof_surfaces: 0,
                new_unknowns: 0,
            },
            risks: Vec::new(),
            coupling: Vec::new(),
            boundary_facts: boundary_facts_for_changed(project, &changed_paths),
            impact: Vec::new(),
            proof: ChangedProofSummary {
                commands: Vec::new(),
                fallback: Vec::new(),
                hard: Vec::new(),
                direct_evidence: Vec::new(),
                mediated_evidence: Vec::new(),
                soft_evidence: Vec::new(),
                setup_support: Vec::new(),
                missing_direct: Vec::new(),
                wiring: Vec::new(),
            },
            unknowns: Vec::new(),
            hidden: Vec::new(),
            expand: changed_expand(&selector),
        };
    }
    let diff = diff_map_report(project, changed.clone(), selector.clone(), limit, mode);
    let impact = impact_report(project, changed.clone(), selector.clone(), 1, limit);
    let proof_map = proof_map_report(project, None, changed, selector.clone(), limit, false);
    let proof_map_cache = Some(Box::new(proof_map.clone()));
    let proof_plan_cache = if changed_paths.len() <= 5 {
        Some(Box::new(proof_report(
            project,
            None,
            changed_paths.clone(),
            changed_proof_selector(&selector),
            1,
            proof_cache_limit,
        )))
    } else {
        None
    };
    let risks = changed_risks(
        project,
        &changed_paths,
        &changed_paths,
        &git_state,
        &proof_map,
        &selector,
    );
    let coupling = changed_coupling(project, &changed_paths, &changed_paths, &proof_map, &selector);
    let mut hidden = Vec::new();
    hidden.extend(prefix_hidden("observed", &diff.hidden, &selector, limit));
    hidden.extend(prefix_hidden("links", &impact.hidden, &selector, limit));
    hidden.extend(prefix_hidden("proof", &proof_map.hidden, &selector, limit));
    let mut unknowns = Vec::new();
    unknowns.extend(diff.new_unknowns.clone());
    unknowns.extend(impact.unknowns.clone());
    unknowns.extend(proof_map.unknowns.clone());
    if let Some(proof) = proof_plan_cache.as_deref() {
        unknowns.extend(proof.unknowns.clone());
    }
    unknowns.extend(changed_fail_open_unknowns(project, &changed_paths));
    dedupe_unknowns(&mut unknowns);
    let unknown_expand = format!(
        "codemap changed{} --section unknown --limit {}",
        changed_self_selector_suffix(&selector),
        unknowns.len()
    );
    truncate_with_hidden(
        &mut unknowns,
        limit,
        &mut hidden,
        "changed unknowns hidden by limit",
        &unknown_expand,
    );
    ChangedReport {
        kind: "changed_report",
        schema_version: "8",
        selector: selector.clone(),
        display_limit: limit,
        proof_plan_cache,
        proof_map_cache,
        total_changed_count,
        changed: impact.changed.clone(),
        git_state,
        structural_events,
        map_delta: ChangedMapDelta {
            added_edges: count_with_hidden(
                diff.added_edges.len(),
                &diff.hidden,
                "added structural edges hidden by limit",
            ),
            removed_edges: count_with_hidden(
                diff.removed_edges.len(),
                &diff.hidden,
                "removed structural edges hidden by limit",
            ),
            changed_symbols: count_with_hidden(
                diff.changed_symbols.len(),
                &diff.hidden,
                "changed symbol surfaces hidden by limit",
            ),
            added_exports: count_with_hidden(
                diff.added_exports.len(),
                &diff.hidden,
                "added export surfaces hidden by limit",
            ),
            removed_exports: count_with_hidden(
                diff.removed_exports.len(),
                &diff.hidden,
                "removed export surfaces hidden by limit",
            ),
            added_runtime_routes: count_with_hidden(
                diff.added_runtime_routes.len(),
                &diff.hidden,
                "added runtime routes hidden by limit",
            ),
            removed_runtime_routes: count_with_hidden(
                diff.removed_runtime_routes.len(),
                &diff.hidden,
                "removed runtime routes hidden by limit",
            ),
            added_env: count_with_hidden(
                diff.added_env.len(),
                &diff.hidden,
                "added env dependencies hidden by limit",
            ),
            removed_env: count_with_hidden(
                diff.removed_env.len(),
                &diff.hidden,
                "removed env dependencies hidden by limit",
            ),
            added_proof_surfaces: count_with_hidden(
                diff.added_proof_surfaces.len(),
                &diff.hidden,
                "added proof surfaces hidden by limit",
            ),
            removed_proof_surfaces: count_with_hidden(
                diff.removed_proof_surfaces.len(),
                &diff.hidden,
                "removed proof surfaces hidden by limit",
            ),
            new_unknowns: count_with_hidden(
                diff.new_unknowns.len(),
                &diff.hidden,
                "new unknowns hidden by limit",
            ),
        },
        risks,
        coupling,
        boundary_facts: boundary_facts_for_changed(project, &changed_paths),
        impact: impact.clusters,
        proof: changed_proof_summary(proof_map, limit),
        unknowns,
        hidden,
        expand: changed_expand(&selector),
    }
}

fn changed_proof_selector(selector: &str) -> String {
    if selector == "--changed" {
        "changed".to_string()
    } else {
        selector.to_string()
    }
}

fn count_with_hidden(visible: usize, hidden: &[HiddenGroup], reason: &str) -> usize {
    visible
        + hidden
            .iter()
            .filter(|group| group.reason == reason)
            .map(|group| group.count)
            .sum::<usize>()
}

fn changed_proof_summary(report: ProofMapReport, limit: usize) -> ChangedProofSummary {
    let command_sensor_limit = limit.min(8);
    let mut by_command: BTreeMap<String, Vec<ProofSurface>> = BTreeMap::new();
    for proof in report.hard.iter() {
        if let Some(command) = &proof.command {
            by_command
                .entry(command.clone())
                .or_default()
                .push(proof.clone());
        }
    }
    let commands = by_command
        .into_iter()
        .map(|(command, mut sensors)| {
            sensors.sort_by(|a, b| {
                a.path
                    .cmp(&b.path)
                    .then_with(|| a.evidence.cmp(&b.evidence))
                    .then_with(|| a.reason.cmp(&b.reason))
            });
            sensors.dedup_by(|a, b| {
                a.path == b.path && a.evidence == b.evidence && a.reason == b.reason
            });
            let hidden_count = sensors.len().saturating_sub(command_sensor_limit);
            sensors.truncate(command_sensor_limit);
            ChangedProofCommand {
                command,
                sensors,
                hidden_count,
            }
        })
        .collect();
    ChangedProofSummary {
        commands,
        fallback: report.fallback,
        hard: report.hard,
        direct_evidence: report.direct_evidence,
        mediated_evidence: report.mediated_evidence,
        soft_evidence: report.soft_evidence,
        setup_support: report.setup_support,
        missing_direct: report.missing_direct,
        wiring: report.wiring,
    }
}

fn changed_expand(selector: &str) -> Vec<String> {
    let changed_suffix = changed_self_selector_suffix(selector);
    let proof_selector = if selector == "--changed" {
        "changed"
    } else {
        selector
    };
    vec![
        format!("codemap changed{changed_suffix} --section observed"),
        format!("codemap changed{changed_suffix} --section links"),
        format!("codemap changed{changed_suffix} --section roles"),
        format!("codemap changed{changed_suffix} --section proof"),
        format!("codemap changed{changed_suffix} --section unknown"),
        format!("codemap diff-map {selector}"),
        format!("codemap impact {selector}"),
        format!("codemap proof-map {selector}"),
        format!("codemap proof {proof_selector}"),
    ]
}

fn changed_self_selector_suffix(selector: &str) -> String {
    if selector == "--changed" {
        String::new()
    } else {
        format!(" {selector}")
    }
}

fn prefix_hidden(
    prefix: &str,
    hidden: &[HiddenGroup],
    selector: &str,
    current_limit: usize,
) -> Vec<HiddenGroup> {
    let selector_suffix = changed_self_selector_suffix(selector);
    hidden
        .iter()
        .map(|group| HiddenGroup {
            reason: format!("{prefix}: {}", group.reason),
            count: group.count,
            expand: if group.expand.len() > 160 || group.expand.contains("--files ") {
                format!(
                    "codemap changed{selector_suffix} --section {prefix} --limit {}",
                    current_limit + group.count
                )
            } else {
                group.expand.clone()
            },
        })
        .collect()
}

fn dedupe_unknowns(unknowns: &mut Vec<Unknown>) {
    let mut seen = BTreeSet::new();
    unknowns.retain(|unknown| {
        seen.insert((
            unknown.kind.clone(),
            unknown.path.clone(),
            unknown.line_start,
            unknown.reason.clone(),
        ))
    });
    unknowns.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| a.path.cmp(&b.path))
            .then_with(|| a.line_start.cmp(&b.line_start))
    });
}
