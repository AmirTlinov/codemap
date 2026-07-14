fn is_support_artifact_path(rel: &str) -> bool {
    let rel = repo::normalize_rel_path(rel);
    rel.split('/').any(|part| {
        matches!(
            part,
            "fixtures"
                | "examples"
                | "samples"
                | "artifacts"
                | "receipts"
                | "witnesses"
                | ".agents"
                | ".codex"
                | ".claude"
        )
    })
}

pub fn impact_report(
    project: &Project,
    changed: Vec<String>,
    selector: String,
    depth: usize,
    limit: usize,
) -> ImpactReport {
    let limit = limit.max(1);
    let changed = changed
        .into_iter()
        .map(|file| repo::normalize_rel_path(&file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let selector = normalized_impact_selector(&changed, selector);
    let mut hidden = Vec::new();
    let mut unknowns = Vec::new();
    let mut changed_summaries = Vec::new();
    let mut cluster_reports = Vec::new();
    let changed_count = changed.len();
    for rel in &changed {
        if let Some(file) = project.files.get(rel) {
            changed_summaries.push(file_summary(project, file, false, 12));
            let (cluster, cluster_hidden) = impact_cluster(project, rel, depth, limit);
            cluster_reports.push((cluster, cluster_hidden));
        } else {
            unknowns.push(unknown_unindexed_anchor(rel));
            changed_summaries.push(missing_file_summary(project, rel));
            cluster_reports.push((ImpactCluster {
                id: format!("changed:{rel}"),
                risk: Risk::Medium.as_str().to_string(),
                changed: vec![rel.clone()],
                direct_consumers: Vec::new(),
                cross_boundary_consumers: Vec::new(),
                contract_links: Vec::new(),
                proof: Vec::new(),
                reasons: vec!["changed file is not indexed".to_string()],
            }, Vec::new()));
        }
    }
    if changed_count > limit {
        hidden.push(HiddenGroup {
            reason: "changed anchors hidden by limit".to_string(),
            count: changed_count - limit,
            expand: impact_hidden_changed_expand(&selector, depth, changed_count),
        });
        hidden.push(HiddenGroup {
            reason: "impact clusters hidden by limit".to_string(),
            count: cluster_reports.len().saturating_sub(limit),
            expand: impact_hidden_changed_expand(&selector, depth, changed_count),
        });
        changed_summaries.truncate(limit);
    }
    let mut clusters = Vec::new();
    for (cluster, cluster_hidden) in cluster_reports.into_iter().take(limit) {
        hidden.extend(cluster_hidden);
        clusters.push(cluster);
    }
    ImpactReport {
        kind: "impact_report",
        schema_version: "4",
        selector: selector.clone(),
        changed: changed_summaries,
        clusters,
        hidden,
        unknowns,
        expand: impact_expand_commands(&changed, &selector),
    }
}

fn normalized_impact_selector(changed: &[String], selector: String) -> String {
    if !selector.trim().is_empty() {
        return selector;
    }
    if changed.is_empty() {
        return String::new();
    }
    let files = changed
        .iter()
        .map(|file| shell_quote(file))
        .collect::<Vec<_>>()
        .join(",");
    format!("--files {files}")
}

fn impact_hidden_changed_expand(selector: &str, depth: usize, limit: usize) -> String {
    let selector = selector.trim();
    if selector.is_empty() {
        return format!("codemap impact --changed --depth {depth} --limit {limit}");
    }
    format!("codemap impact {selector} --depth {depth} --limit {limit}")
}

pub fn proof_report(
    project: &Project,
    target: Option<String>,
    changed: Vec<String>,
    selector: String,
    depth: usize,
    limit: usize,
) -> ProofReport {
    let limit = limit.max(1);
    let target = target.map(|path| repo::normalize_rel_path(&path));
    let changed = changed
        .into_iter()
        .map(|file| repo::normalize_rel_path(&file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
    let anchors = if let Some(target) = target.as_ref() {
        vec![target.clone()]
    } else {
        changed.clone()
    };
    let mut proofs = Vec::new();
    let mut hidden = Vec::new();
    let mut risk = Risk::Low;
    let discovery_limit = usize::MAX;
    let mut coverage = None;
    if target.is_none() && !changed.is_empty() {
        let impact = impact_report(
            project,
            changed.clone(),
            selector.clone(),
            depth,
            limit.max(changed.len()),
        );
        for cluster in &impact.clusters {
            risk = risk.max(impact_level_from_str(&cluster.risk));
        }
        let mut proofs_by_anchor = BTreeMap::new();
        for anchor in &changed {
            let anchor_proofs = proof_surfaces_for_anchor(
                project,
                anchor,
                depth,
                discovery_limit,
            );
            proofs_by_anchor.insert(anchor.clone(), anchor_proofs.clone());
            proofs.extend(anchor_proofs);
        }
        proofs.extend(codemap_changed_proof_surfaces(project));
        coverage = Some(proof_coverage_summary(&changed, &proofs_by_anchor));
    } else {
        for anchor in &anchors {
            if let Some((file_rel, symbol_name)) = split_symbol_anchor(anchor) {
                risk = risk.max(
                    project
                        .files
                        .get(&file_rel)
                        .map(|_| structural_impact_level_for_file(project, &file_rel, depth).0)
                        .unwrap_or(Risk::Medium),
                );
                proofs.extend(proof_surfaces_for_symbol_anchor(
                    project,
                    &file_rel,
                    &symbol_name,
                    depth,
                    discovery_limit,
                ));
            } else {
                if project.files.contains_key(anchor) {
                    risk = risk.max(structural_impact_level_for_file(project, anchor, depth).0);
                    proofs.extend(proof_surfaces_for_anchor(
                        project,
                        anchor,
                        depth,
                        discovery_limit,
                    ));
                } else if anchor != "." && directory_has_files(project, anchor) {
                    risk = risk.max(impact_level_for_directory(project, anchor, depth));
                    proofs.extend(proof_surfaces_for_directory(
                        project,
                        anchor,
                        depth,
                        discovery_limit,
                    ));
                } else {
                    risk = risk.max(Risk::Medium);
                    proofs.extend(proof_surfaces_for_anchor(
                        project,
                        anchor,
                        depth,
                        discovery_limit,
                    ));
                }
            }
        }
    }
    let all_proofs = unique_proof_surfaces(proofs);
    let fallback = proof_fallback_commands(project, &anchors, &changed, &all_proofs);
    let wiring_discovery_limit = limit.saturating_mul(2).max(6);
    let (mut wiring, wiring_clipped) = proof_wiring_facts_limited(
        project,
        &anchors,
        &changed,
        &all_proofs,
        &fallback,
        wiring_discovery_limit,
    );
    let mut unknowns = Vec::new();
    if target.is_none() && !changed.is_empty() {
        unknowns.extend(changed_fail_open_unknowns(project, &changed));
        unknowns.extend(changed_missing_deterministic_proof_unknowns(
            project,
            &changed,
            &fallback,
            &all_proofs,
        ));
    }
    if let Some(target) = target.as_ref()
        && let Some(file) = project.files.get(target)
    {
        unknowns.extend(proof_target_owner_unknowns(project, target, file));
    }
    if let Some(target) = target.as_ref()
        && let Some(file) = project.files.get(target)
        && changed_should_check_direct_proof(file)
        && !strict_test_edges_for_file(project, target, usize::MAX)
            .iter()
            .any(|(_, _, strength)| *strength >= EvidenceStrength::High)
    {
        unknowns.push(unknown(
            "direct_test_import_not_found",
            Some(target),
            None,
            "no direct test import, symbol reference, support import, or e2e route visit was found for this proof anchor",
            "verification surfaces may still include scripts, CI, contract checks, or soft matches, but no direct linked test surface was found",
            Some(format!("codemap proof-map {} --raw-sensors", shell_quote(target))),
        ));
    }
    if let Some(target) = target.as_ref()
        && let Some((file_rel, _symbol_name)) = split_symbol_anchor(target)
        && let Some(file) = project.files.get(&file_rel)
        && changed_should_check_direct_proof(file)
        && !all_proofs.iter().any(proof_surface_satisfies_specific_proof)
    {
        unknowns.push(unknown(
            "direct_test_import_not_found",
            Some(target),
            None,
            "no direct test import, symbol reference, support import, or e2e route visit was found for this symbol proof anchor",
            "mediated symbol-consumer surfaces may still be visible, but they do not create a direct linked test surface for the selected symbol",
            Some(format!("codemap proof-map {} --raw-sensors", shell_quote(target))),
        ));
    }
    if let Some(target) = target.as_ref()
        && let Some(file) = project.files.get(target)
        && file_uses_ci_run_step_syntax(file)
        && !all_proofs
            .iter()
            .any(proof_ci_run_step_is_validation)
    {
        unknowns.push(unknown_ci_validation_step_not_found(target));
    }
    if let Some(target) = target.as_ref()
        && (project.files.contains_key(target) || directory_has_files(project, target))
        && (proof_missing_should_surface(project, target)
            || all_proofs.iter().any(proof_surface_is_soft_structural_match))
        && !all_proofs.is_empty()
        && !all_proofs.iter().any(proof_surface_satisfies_specific_proof)
    {
        unknowns.push(unknown_missing_deterministic_proof(
            target,
            format!("codemap proof-map {}", shell_quote(target)),
        ));
    }
    unknowns.extend(proof_wiring_unknowns(&wiring));
    dedupe_unknowns(&mut unknowns);
    proofs = all_proofs;
    let proof_count = proofs.len();
    if proof_count > limit {
        hidden.push(HiddenGroup {
            reason: "verification surfaces hidden by limit".to_string(),
            count: proof_count - limit,
            expand: format!(
                "codemap proof {} --depth {depth} --limit {}",
                selector, proof_count
            ),
        });
        proofs = balanced_proof_surface_prefix(&proofs, limit);
    }
    truncate_with_hidden(
        &mut wiring,
        limit.saturating_mul(2).max(6),
        &mut hidden,
        "verification wiring facts hidden by limit",
        &format!("codemap proof {} --section links", selector),
    );
    if wiring_clipped {
        hidden.push(HiddenGroup {
            reason: "verification wiring facts hidden by discovery limit".to_string(),
            count: 1,
            expand: format!(
                "codemap proof {} --section links --limit {}",
                selector,
                wiring_discovery_limit.saturating_mul(2)
            ),
        });
    }
    let mut expand = Vec::new();
    if proofs.is_empty()
        && let Some(target) = target.as_ref()
        && (project.files.contains_key(target) || directory_has_files(project, target))
        && let Some(nearest) = nearest_proof_scope(project, target)
    {
        let command = format!("codemap proof {}", shell_quote(&nearest));
        unknowns.push(nearest_proof_scope_unknown(target, &nearest, command.clone()));
        expand.push(command);
    }
    ProofReport {
        kind: "proof_plan",
        schema_version: "9",
        target,
        changed,
        selector,
        risk: risk.as_str().to_string(),
        proofs,
        coverage,
        wiring,
        fallback,
        unknowns,
        hidden,
        expand,
        run_hint: "codemap proof prints a verification surface plan by default; use --run to execute rendered commands"
            .to_string(),
    }
}

fn proof_target_owner_unknowns(project: &Project, target: &str, file: &FileInfo) -> Vec<Unknown> {
    let mut unknowns = Vec::new();
    if file.has_role("manifest") {
        unknowns.extend(changed_manifest_unknowns(project, target));
    }
    if file.has_role("schema_contract") || schema_owner_path(target) {
        unknowns.extend(changed_schema_unknowns(project, target));
    }
    if file.has_role("env_config") {
        unknowns.extend(owner_env_unknowns(project, target));
        unknowns.extend(changed_env_unknowns(project, target));
    }
    unknowns
}

fn changed_missing_deterministic_proof_unknowns(
    project: &Project,
    changed: &[String],
    fallback: &[String],
    proofs: &[ProofSurface],
) -> Vec<Unknown> {
    if !fallback.is_empty() || proofs.iter().any(proof_surface_satisfies_specific_proof) {
        return Vec::new();
    }
    let has_soft_or_empty = proofs.is_empty()
        || proofs
            .iter()
            .any(crate::proof_classification::proof_surface_is_soft_evidence);
    if !has_soft_or_empty {
        return Vec::new();
    }
    changed
        .iter()
        .filter(|rel| changed_path_needs_missing_deterministic_proof_unknown(project, rel))
        .map(|rel| {
            unknown_missing_deterministic_proof(
                rel,
                format!("codemap proof-map --files {}", shell_quote(rel)),
            )
        })
        .collect()
}

fn changed_path_needs_missing_deterministic_proof_unknown(project: &Project, rel: &str) -> bool {
    if is_support_artifact_path(rel) {
        return true;
    }
    project.files.get(rel).is_some_and(|file| {
        proof_missing_should_surface(project, rel)
            || [
                "receipt",
                "witness",
                "fixture",
                "generated",
                "archive",
                "build_output",
            ]
            .iter()
            .any(|role| file.has_role(role))
    })
}

pub fn clean_proof_report(selector: String) -> ProofReport {
    ProofReport {
        kind: "proof_plan",
        schema_version: "9",
        target: None,
        changed: Vec::new(),
        selector,
        risk: "low".to_string(),
        proofs: Vec::new(),
        coverage: None,
        wiring: Vec::new(),
        fallback: Vec::new(),
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: Vec::new(),
        run_hint: "codemap proof prints a verification surface plan by default; use --run to execute rendered commands"
            .to_string(),
    }
}

fn impact_level_from_str(value: &str) -> Risk {
    match value {
        "critical" => Risk::Critical,
        "high" => Risk::High,
        "medium-high" => Risk::MediumHigh,
        "medium" => Risk::Medium,
        _ => Risk::Low,
    }
}
