fn is_support_artifact_path(rel: &str) -> bool {
    let rel = repo::normalize_rel_path(rel);
    rel.split('/').any(|part| {
        matches!(
            part,
            "fixtures" | "examples" | "samples" | ".agents" | ".codex" | ".claude"
        )
    })
}

pub fn impact_report(
    project: &Project,
    changed: Vec<String>,
    depth: usize,
    limit: usize,
) -> ImpactReport {
    let limit = limit.max(1);
    let changed = changed
        .into_iter()
        .map(|file| repo::normalize_rel_path(&file))
        .filter(|file| file != ".")
        .collect::<Vec<_>>();
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
            expand: impact_hidden_changed_expand(&changed, depth, changed_count),
        });
        hidden.push(HiddenGroup {
            reason: "impact clusters hidden by limit".to_string(),
            count: cluster_reports.len().saturating_sub(limit),
            expand: impact_hidden_changed_expand(&changed, depth, changed_count),
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
        changed: changed_summaries,
        clusters,
        hidden,
        unknowns,
        expand: impact_expand_commands(&changed),
    }
}

fn impact_hidden_changed_expand(changed: &[String], depth: usize, limit: usize) -> String {
    if changed.is_empty() {
        return format!("codemap impact --changed --depth {depth} --limit {limit}");
    }
    let files = changed
        .iter()
        .map(|file| shell_quote(file))
        .collect::<Vec<_>>()
        .join(",");
    format!("codemap impact --files {files} --depth {depth} --limit {limit}")
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
    if target.is_none() && !changed.is_empty() {
        let impact = impact_report(project, changed.clone(), depth, limit.max(changed.len()));
        for cluster in &impact.clusters {
            risk = risk.max(impact_level_from_str(&cluster.risk));
        }
        for anchor in &changed {
            proofs.extend(proof_surfaces_for_anchor(
                project,
                anchor,
                depth,
                discovery_limit,
            ));
        }
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
    let mut unknowns = Vec::new();
    if let Some(target) = target.as_ref()
        && (project.files.contains_key(target) || directory_has_files(project, target))
        && proof_missing_should_surface(project, target)
        && !all_proofs.is_empty()
        && !all_proofs.iter().any(proof_surface_satisfies_specific_proof)
    {
        unknowns.push(unknown_missing_deterministic_proof(
            target,
            format!("codemap proof-map {}", shell_quote(target)),
        ));
    }
    proofs = all_proofs;
    if proofs.len() > limit {
        hidden.push(HiddenGroup {
            reason: "proof surfaces hidden by limit".to_string(),
            count: proofs.len() - limit,
            expand: format!(
                "codemap proof {} --depth {depth} --limit {}",
                selector,
                proofs.len()
            ),
        });
        proofs.truncate(limit);
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
        schema_version: "6",
        target,
        changed,
        risk: risk.as_str().to_string(),
        proofs,
        fallback,
        unknowns,
        hidden,
        expand,
        run_hint: "codemap proof prints only by default; use --run to execute proof commands"
            .to_string(),
    }
}

pub fn clean_proof_report(_selector: String) -> ProofReport {
    ProofReport {
        kind: "proof_plan",
        schema_version: "6",
        target: None,
        changed: Vec::new(),
        risk: "low".to_string(),
        proofs: Vec::new(),
        fallback: Vec::new(),
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: Vec::new(),
        run_hint: "codemap proof prints only by default; use --run to execute proof commands"
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
