pub fn runtime_report(
    project: &Project,
    scope: &str,
    include_hidden: bool,
    limit: usize,
) -> RuntimeReport {
    let limit = limit.max(1);
    let scope = repo::normalize_rel_path(scope);
    let mut entrypoints = Vec::new();
    let mut routes = Vec::new();
    let mut scripts = Vec::new();
    let mut env = Vec::new();
    let mut workers = Vec::new();
    let mut ci = Vec::new();
    let mut proof = Vec::new();
    let mut unknowns = Vec::new();
    for file in files_under_directory(project, &scope) {
        if runtime_entrypoint_kind(file).is_some() {
            entrypoints.push(surface_from_path(
                runtime_entrypoint_kind(file).unwrap_or("entrypoint"),
                &file.rel,
                "file_convention",
                EvidenceStrength::High,
            ));
        }
        if file.has_role("build_ci") {
            ci.push(surface_from_path(
                "build_ci",
                &file.rel,
                "role:build_ci",
                EvidenceStrength::High,
            ));
        }
        if file.rel.contains("worker") || file.rel.contains("cron") || file.rel.contains("job") {
            workers.push(surface_from_path(
                "worker_or_job",
                &file.rel,
                "path_convention",
                EvidenceStrength::Medium,
            ));
        }
        routes.extend(runtime_routes_for_file(project, file));
        env.extend(env_surfaces_for_file(project, file));
        unknowns.extend(unknowns_for_file(project, file));
        proof.extend(cone_proof_edges(project, std::slice::from_ref(&file.rel)));
    }
    for script in &project.scripts {
        if scope == "." {
            scripts.push(Surface {
                id: format!("surface:script:{}", script.name),
                kind: "script".to_string(),
                path: None,
                role: Some("script".to_string()),
                evidence: script.reason.clone(),
                strength: EvidenceStrength::Hard,
                count: Some(1),
                examples: vec![format!("{}: {}", script.name, script.command)],
                hidden_count: 0,
            });
        }
    }
    let mut hidden = Vec::new();
    truncate_with_hidden(
        &mut entrypoints,
        limit,
        &mut hidden,
        "runtime entrypoints hidden by limit",
        "codemap runtime <scope> --include-hidden",
    );
    truncate_with_hidden(
        &mut routes,
        limit,
        &mut hidden,
        "runtime routes hidden by limit",
        "codemap runtime <scope> --include-hidden",
    );
    truncate_with_hidden(
        &mut env,
        limit,
        &mut hidden,
        "environment surfaces hidden by limit",
        "codemap runtime <scope> --include-hidden",
    );
    truncate_with_hidden(
        &mut unknowns,
        limit,
        &mut hidden,
        "runtime unknowns hidden by limit",
        "codemap runtime <scope> --include-hidden",
    );
    if !include_hidden {
        proof.truncate(limit);
    }
    RuntimeReport {
        kind: "runtime_report",
        schema_version: "1",
        scope: scope.clone(),
        entrypoints,
        routes,
        scripts,
        env,
        workers,
        ci,
        proof,
        unknowns,
        hidden,
        expand: vec![
            format!("codemap cone {}", shell_quote(&scope)),
            format!("codemap proof-map {}", shell_quote(&scope)),
        ],
    }
}

pub fn proof_map_report(
    project: &Project,
    scope: Option<String>,
    changed: Vec<String>,
    limit: usize,
) -> ProofMapReport {
    let limit = limit.max(1);
    let scope = scope.map(|value| repo::normalize_rel_path(&value));
    let seeds = if let Some(scope) = &scope {
        if directory_has_files(project, scope) {
            files_under_directory(project, scope)
                .into_iter()
                .map(|file| file.rel.clone())
                .collect::<Vec<_>>()
        } else {
            vec![scope.clone()]
        }
    } else {
        changed.clone()
    };
    let mut direct = Vec::new();
    let mut indirect = Vec::new();
    let mut e2e = Vec::new();
    let mut contract = Vec::new();
    let mut missing_direct = Vec::new();
    let mut unknowns = Vec::new();
    for seed in &seeds {
        if let Some(file) = project.files.get(seed) {
            unknowns.extend(unknowns_for_file(project, file));
        }
        let proofs = proof_surfaces_for_anchor(project, seed, 1, limit);
        if proofs.is_empty() && proof_missing_should_surface(project, seed) {
            missing_direct.push(surface_from_path(
                "missing_direct_proof",
                seed,
                "no_structural_proof_surface",
                EvidenceStrength::Medium,
            ));
        }
        for proof in proofs {
            if proof.evidence.contains("via_") {
                indirect.push(proof);
            } else if proof.evidence.starts_with("e2e") {
                e2e.push(proof);
            } else if project
                .files
                .get(seed)
                .and_then(contract_evidence)
                .is_some()
            {
                contract.push(proof);
            } else {
                direct.push(proof);
            }
        }
    }
    let mut hidden = Vec::new();
    truncate_with_hidden(
        &mut direct,
        limit,
        &mut hidden,
        "direct proof surfaces hidden by limit",
        "codemap proof-map <scope> --include-hidden",
    );
    truncate_with_hidden(
        &mut indirect,
        limit,
        &mut hidden,
        "indirect proof surfaces hidden by limit",
        "codemap proof-map <scope> --include-hidden",
    );
    truncate_with_hidden(
        &mut unknowns,
        limit,
        &mut hidden,
        "proof-map unknowns hidden by limit",
        "codemap proof-map <scope> --include-hidden",
    );
    let commands = unique_proof_surfaces(
        direct
            .iter()
            .chain(indirect.iter())
            .chain(e2e.iter())
            .chain(contract.iter())
            .filter(|proof| proof.command.is_some())
            .cloned()
            .collect(),
    );
    let fallback = proof_fallback_commands(project, &seeds, &changed, &commands);
    ProofMapReport {
        kind: "proof_map_report",
        schema_version: "1",
        scope,
        changed,
        direct,
        indirect,
        e2e,
        contract,
        missing_direct,
        commands,
        fallback,
        unknowns,
        hidden,
        expand: vec!["codemap proof --changed".to_string()],
    }
}
