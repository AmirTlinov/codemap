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
    let mut clusters = Vec::new();
    let changed_count = changed.len();
    for rel in changed.iter().take(limit) {
        if let Some(file) = project.files.get(rel) {
            changed_summaries.push(file_summary(project, file, false, 12));
            let (cluster, cluster_hidden) = impact_cluster(project, rel, depth, limit);
            hidden.extend(cluster_hidden);
            clusters.push(cluster);
        } else {
            unknowns.push(unknown_unindexed_anchor(rel));
            changed_summaries.push(missing_file_summary(project, rel));
            clusters.push(ImpactCluster {
                id: format!("changed:{rel}"),
                risk: Risk::Medium.as_str().to_string(),
                changed: vec![rel.clone()],
                direct_consumers: Vec::new(),
                cross_boundary_consumers: Vec::new(),
                contract_risks: Vec::new(),
                proof: Vec::new(),
                reasons: vec!["changed file is not indexed".to_string()],
            });
        }
    }
    if changed_count > changed_summaries.len() {
        hidden.push(HiddenGroup {
            reason: "changed anchors hidden by limit".to_string(),
            count: changed_count - changed_summaries.len(),
            expand: "codemap impact --changed --limit <larger-number>".to_string(),
        });
    }
    ImpactReport {
        kind: "impact_report",
        schema_version: "3",
        changed: changed_summaries,
        clusters,
        hidden,
        unknowns,
        expand: impact_expand_commands(&changed),
    }
}

pub fn proof_report(
    project: &Project,
    target: Option<String>,
    changed: Vec<String>,
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
    let mut risk = Risk::Low;
    if target.is_none() && !changed.is_empty() {
        let impact = impact_report(project, changed.clone(), depth, limit);
        for cluster in &impact.clusters {
            risk = risk.max(risk_from_str(&cluster.risk));
            proofs.extend(proof_surfaces_from_edges(
                project,
                &cluster.proof,
                "impact cluster",
            ));
        }
    } else {
        for anchor in &anchors {
            if let Some((file_rel, symbol_name)) = split_symbol_anchor(anchor) {
                risk = risk.max(
                    project
                        .files
                        .get(&file_rel)
                        .map(|_| structural_risk_for_file(project, &file_rel, depth).0)
                        .unwrap_or(Risk::Medium),
                );
                proofs.extend(proof_surfaces_for_symbol_anchor(
                    project,
                    &file_rel,
                    &symbol_name,
                    depth,
                    limit,
                ));
            } else {
                if project.files.contains_key(anchor) {
                    risk = risk.max(structural_risk_for_file(project, anchor, depth).0);
                    proofs.extend(proof_surfaces_for_anchor(project, anchor, depth, limit));
                } else if anchor != "." && directory_has_files(project, anchor) {
                    risk = risk.max(risk_for_directory(project, anchor, depth));
                    proofs.extend(proof_surfaces_for_directory(project, anchor, depth, limit));
                } else {
                    risk = risk.max(Risk::Medium);
                    proofs.extend(proof_surfaces_for_anchor(project, anchor, depth, limit));
                }
            }
        }
    }
    proofs = unique_proof_surfaces(proofs);
    if proofs.len() > limit {
        proofs.truncate(limit);
    }
    let fallback = proof_fallback_commands(project, &anchors, &changed, &proofs);
    ProofReport {
        kind: "proof_plan",
        schema_version: "2",
        target,
        changed,
        risk: risk.as_str().to_string(),
        proofs,
        fallback,
        run_hint: "codemap proof prints only by default; use --run to execute proof commands"
            .to_string(),
    }
}

fn risk_from_str(value: &str) -> Risk {
    match value {
        "critical" => Risk::Critical,
        "high" => Risk::High,
        "medium-high" => Risk::MediumHigh,
        "medium" => Risk::Medium,
        _ => Risk::Low,
    }
}
