fn proof_surfaces_for_anchor(
    project: &Project,
    anchor: &str,
    depth: usize,
    limit: usize,
) -> Vec<ProofSurface> {
    let mut out = Vec::new();
    for (test, evidence, strength) in strict_test_edges_for_file(project, anchor, limit) {
        out.push(ProofSurface {
            command: proof_command_for_test(project, &test),
            locations: proof_surface_locations_for_test(&test, &evidence),
            path: Some(test),
            reason: proof_reason_for_evidence(&evidence, "anchor"),
            evidence,
            strength,
        });
    }
    if out.is_empty() {
        for edge in proof_edges_via_direct_dependencies(project, anchor, limit) {
            out.push(ProofSurface {
                command: proof_command_for_test(project, &edge.from),
                path: Some(edge.from),
                reason: proof_reason_for_evidence(&edge.evidence, "anchor"),
                locations: edge.locations,
                evidence: edge.evidence,
                strength: edge.strength,
            });
        }
    }
    if depth <= 1 && !out.is_empty() {
        return out;
    }
    let mut consumers = direct_consumer_edges(project, anchor);
    sort_edges(&mut consumers);
    for consumer in consumers.into_iter().take(limit) {
        for (test, evidence, strength) in strict_test_edges_for_file(project, &consumer.from, limit)
        {
            out.push(ProofSurface {
                command: proof_command_for_test(project, &test),
                locations: proof_surface_locations_for_test(&test, &evidence),
                path: Some(test),
                reason: proof_reason_for_evidence(&evidence, "direct consumer"),
                evidence,
                strength,
            });
        }
    }
    if depth > 1 {
        for consumer in direct_consumer_edges(project, anchor)
            .into_iter()
            .take(limit)
        {
            for second in direct_consumer_edges(project, &consumer.from)
                .into_iter()
                .take(limit)
            {
                for (test, evidence, strength) in
                    strict_test_edges_for_file(project, &second.from, limit)
                {
                    out.push(ProofSurface {
                        command: proof_command_for_test(project, &test),
                        locations: proof_surface_locations_for_test(&test, &evidence),
                        path: Some(test),
                        reason: proof_reason_for_evidence(&evidence, "depth-2 consumer"),
                        evidence,
                        strength,
                    });
                }
            }
        }
    }
    out
}

fn proof_surfaces_for_directory(
    project: &Project,
    rel: &str,
    depth: usize,
    limit: usize,
) -> Vec<ProofSurface> {
    let mut out = Vec::new();
    for file in directory_seed_file_paths(project, rel, false) {
        out.extend(proof_surfaces_for_anchor(project, &file, depth, limit));
    }
    out
}

fn proof_surfaces_for_symbol_anchor(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    depth: usize,
    limit: usize,
) -> Vec<ProofSurface> {
    let mut out = symbol_proof_edges_with_owning_file(project, file_rel, symbol_name, limit)
        .into_iter()
        .take(limit)
        .map(|edge| ProofSurface {
            command: proof_command_for_test(project, &edge.from),
            path: Some(edge.from),
            reason: proof_reason_for_evidence(&edge.evidence, "symbol anchor"),
            locations: edge.locations,
            evidence: edge.evidence,
            strength: edge.strength,
        })
        .collect::<Vec<_>>();
    if !out.is_empty() {
        return out;
    }
    if depth <= 1 {
        return out;
    }
    for consumer in symbol_reference_edges(project, file_rel, symbol_name, false)
        .into_iter()
        .take(limit)
    {
        let consumer_file = consumer.from;
        for (test, evidence, strength) in strict_test_edges_for_file(project, &consumer_file, limit)
        {
            out.push(ProofSurface {
                command: proof_command_for_test(project, &test),
                locations: proof_surface_locations_for_test(&test, &evidence),
                path: Some(test),
                reason: proof_reason_for_evidence(&evidence, "symbol consumer"),
                evidence,
                strength,
            });
        }
    }
    out
}

fn proof_surface_locations_for_test(test: &str, evidence: &str) -> Vec<EvidenceLocation> {
    vec![EvidenceLocation::path(test, evidence)]
}

fn risk_for_directory(project: &Project, rel: &str, depth: usize) -> Risk {
    files_under_directory(project, rel)
        .into_iter()
        .filter(|file| !file.has_role("generated") && !is_generic_noise(file))
        .map(|file| structural_risk_for_file(project, &file.rel, depth).0)
        .max()
        .unwrap_or(Risk::Medium)
}

fn structural_risk_for_file(project: &Project, rel: &str, depth: usize) -> (Risk, Vec<String>) {
    let (file_risk, mut reasons) = risk_for_file(project, rel);
    if !project.files.contains_key(rel) {
        return (file_risk, reasons);
    }
    let direct_consumers = direct_consumer_edges(project, rel);
    let cross_boundary_consumers =
        cross_boundary_consumer_edges(project, rel, &direct_consumers, depth.max(1));
    let contract_risks = contract_risk_edges(project, rel, &direct_consumers);
    let (structural_risk, structural_reasons) = structural_impact_risk(
        project,
        rel,
        &direct_consumers,
        &cross_boundary_consumers,
        &contract_risks,
    );
    reasons.extend(structural_reasons);
    (file_risk.max(structural_risk), unique(reasons))
}

fn proof_reason_for_evidence(evidence: &str, scope: &str) -> String {
    if let Some(base) = evidence.strip_suffix("_owning_file") {
        return format!(
            "{} on owning file; no exact symbol proof found",
            proof_reason_for_evidence(base, scope)
        );
    }
    if let Some(base) = evidence.strip_suffix("_via_direct_consumer") {
        return format!(
            "{} via direct consumer",
            proof_reason_for_evidence(base, scope)
        );
    }
    if let Some(base) = evidence.strip_suffix("_via_direct_dependency") {
        return format!(
            "{} via direct dependency",
            proof_reason_for_evidence(base, scope)
        );
    }
    if let Some(base) = evidence.strip_suffix("_via_local_symbol_consumer") {
        return format!(
            "{} via same-file symbol consumer",
            proof_reason_for_evidence(base, scope)
        );
    }
    match evidence {
        "test_import" => format!("test imports {scope}"),
        "test_imported_symbol_reference" => {
            format!("test imports and references {scope}")
        }
        "test_reexported_symbol_reference" => {
            format!("test imports and references re-exported {scope}")
        }
        "e2e_route" => format!("e2e visits route for {scope}"),
        "test_name" => format!("test name matches {scope}"),
        "test_support_import" => format!("test imports support code that imports {scope}"),
        "test_symbol_reference" => format!("test references an anchor symbol from {scope}"),
        "test_surface_phrase" => format!("test uses same UI/test surface as {scope}"),
        "e2e_surface_phrase" => format!("e2e uses same UI/test surface as {scope}"),
        "e2e_path_surface" => format!("e2e path/name surface matches {scope}"),
        "test_surface_tokens" => format!("test path/symbols match {scope} surface"),
        _ => format!("structural proof for {scope}"),
    }
}
