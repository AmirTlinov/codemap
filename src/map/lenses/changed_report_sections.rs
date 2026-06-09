pub fn changed_report_for_section(
    project: &Project,
    changed: Vec<String>,
    selector: String,
    mode: DiffMapMode,
    git_state: Vec<GitChange>,
    limit: usize,
    section: &str,
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
    let section_paths = changed_section_paths(project, &changed_paths, limit);
    let mut report =
        changed_report_shell(&selector, limit, total_changed_count, git_state.clone());
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
            report.coupling =
                changed_coupling(project, &section_paths, &changed_paths, &proof_map, &selector);
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
            let diff = diff_map_report(project, section_paths.clone(), selector.clone(), limit, mode);
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
            structural_events.extend(changed_diff_structural_events(project, &section_paths, &mode));
            sort_changed_structural_events(&mut structural_events);
            let diff = diff_map_report(project, section_paths.clone(), selector.clone(), limit, mode);
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

pub fn clean_changed_report(selector: String, limit: usize) -> ChangedReport {
    ChangedReport {
        kind: "changed_report",
        schema_version: "8",
        selector: selector.clone(),
        display_limit: limit.max(1),
        proof_plan_cache: None,
        proof_map_cache: None,
        total_changed_count: 0,
        changed: Vec::new(),
        git_state: Vec::new(),
        structural_events: Vec::new(),
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
        boundary_facts: BoundaryFacts::default(),
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
    }
}

fn changed_report_shell(
    selector: &str,
    limit: usize,
    total_changed_count: usize,
    git_state: Vec<GitChange>,
) -> ChangedReport {
    ChangedReport {
        kind: "changed_report",
        schema_version: "8",
        selector: selector.to_string(),
        display_limit: limit,
        proof_plan_cache: None,
        proof_map_cache: None,
        total_changed_count,
        changed: Vec::new(),
        git_state,
        structural_events: Vec::new(),
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
        boundary_facts: BoundaryFacts::default(),
        impact: Vec::new(),
        proof: empty_changed_proof_summary(),
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: changed_expand(selector),
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

fn changed_section_paths(project: &Project, changed_paths: &[String], limit: usize) -> Vec<String> {
    let mut ranked = changed_paths
        .iter()
        .enumerate()
        .map(|(index, path)| (changed_section_path_cost(project, path), index, path.clone()))
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, _, path)| path)
        .collect()
}

fn changed_section_path_cost(project: &Project, path: &str) -> usize {
    if changed_manifest_for_lockfile(path).is_some()
        || changed_path_is_generated(project, path)
        || changed_path_is_large_binary(project, path)
        || changed_path_is_model_weight_like(path)
    {
        return 4;
    }
    if project.files.get(path).is_some_and(|file| {
        file.has_role("schema_contract")
            || file.has_role("manifest")
            || file.has_role("env_config")
            || file.has_role("build_ci")
    }) || changed_map_path_is_manifest(path)
        || changed_map_path_is_config(path)
        || changed_path_is_runner_like(path)
        || changed_lens_path_looks_like_source(&path.to_ascii_lowercase())
    {
        return 0;
    }
    if changed_path_is_protected_looking(path) {
        return 3;
    }
    1
}

fn empty_changed_proof_summary() -> ChangedProofSummary {
    ChangedProofSummary {
        commands: Vec::new(),
        fallback: Vec::new(),
        hard: Vec::new(),
        direct_evidence: Vec::new(),
        mediated_evidence: Vec::new(),
        soft_evidence: Vec::new(),
        setup_support: Vec::new(),
        missing_direct: Vec::new(),
        wiring: Vec::new(),
    }
}

fn empty_proof_map_report(selector: String, changed: Vec<String>) -> ProofMapReport {
    ProofMapReport {
        kind: "proof_map_report",
        schema_version: "4",
        selector: selector.clone(),
        scope: None,
        changed,
        hard: Vec::new(),
        direct_evidence: Vec::new(),
        mediated_evidence: Vec::new(),
        soft_evidence: Vec::new(),
        setup_support: Vec::new(),
        missing_direct: Vec::new(),
        commands: Vec::new(),
        wiring: Vec::new(),
        fallback: Vec::new(),
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: vec![format!("codemap proof {}", changed_proof_selector(&selector))],
    }
}

fn changed_map_delta_from_diff(diff: &DiffMapReport) -> ChangedMapDelta {
    ChangedMapDelta {
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
    }
}
