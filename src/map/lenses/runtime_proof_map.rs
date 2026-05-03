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
    let (scope_files, hidden_scope_count) = runtime_scope_files(project, &scope, include_hidden);
    let runtime_facts = runtime_fact_index_for_files(project, scope_files.iter().copied());
    let root_containers = if scope == "." && !include_hidden {
        root_runtime_containers(project)
    } else {
        Vec::new()
    };
    for file in scope_files {
        if runtime_entrypoint_kind(file).is_some() {
            entrypoints.push(surface_from_path(
                runtime_entrypoint_kind(file).unwrap_or("entrypoint"),
                &file.rel,
                "file_convention",
                EvidenceStrength::High,
            ));
        }
        entrypoints.extend(runtime_manifest_entrypoints(project, file));
        entrypoints.extend(runtime_code_entrypoints(project, file));
        if file.has_role("build_ci") {
            ci.push(surface_from_path(
                "build_ci",
                &file.rel,
                "role:build_ci",
                EvidenceStrength::High,
            ));
        }
        if runtime_worker_or_job_convention(&file.rel) {
            workers.push(surface_from_path(
                "worker_or_job",
                &file.rel,
                "worker_job_path_convention",
                EvidenceStrength::Medium,
            ));
        }
        let file_routes = runtime_facts.routes_for_file(&file.rel);
        for route in &file_routes {
            proof.extend(route_reference_edges_with_index(
                project,
                route,
                &runtime_facts,
            ));
        }
        routes.extend(file_routes);
        env.extend(env_surfaces_for_file(project, file));
        unknowns.extend(unknowns_for_file(project, file));
    }
    entrypoints.extend(root_containers.clone());
    entrypoints = dedupe_runtime_entrypoints(entrypoints);
    env = group_env_surfaces(env);
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
    let include_hidden_expand = format!("codemap runtime {} --all", shell_quote(&scope));
    if hidden_scope_count > 0 {
        hidden.push(HiddenGroup {
            reason: "recursive runtime files hidden at root scope".to_string(),
            count: hidden_scope_count,
            expand: include_hidden_expand.clone(),
        });
    }
    truncate_with_hidden(
        &mut entrypoints,
        limit,
        &mut hidden,
        "runtime entrypoints hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut routes,
        limit,
        &mut hidden,
        "runtime routes hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut env,
        limit,
        &mut hidden,
        "environment surfaces hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut scripts,
        limit,
        &mut hidden,
        "runtime scripts hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut workers,
        limit,
        &mut hidden,
        "worker/job surfaces hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut ci,
        limit,
        &mut hidden,
        "ci surfaces hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut unknowns,
        limit,
        &mut hidden,
        "runtime unknowns hidden by limit",
        &include_hidden_expand,
    );
    proof.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a.edge_type.cmp(&b.edge_type))
            .then_with(|| a.evidence.cmp(&b.evidence))
            .then_with(|| {
                a.locations
                    .first()
                    .and_then(|location| location.line_start)
                    .cmp(&b.locations.first().and_then(|location| location.line_start))
            })
    });
    limit_edge_section(
        &mut proof,
        &mut hidden,
        include_hidden,
        limit,
        "runtime proof edges hidden by limit",
        &include_hidden_expand,
    );
    let expand = runtime_expand_commands(&scope, &root_containers, &entrypoints);
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
        expand,
    }
}

pub fn proof_map_report(
    project: &Project,
    scope: Option<String>,
    changed: Vec<String>,
    proof_selector: String,
    limit: usize,
    raw_sensors: bool,
) -> ProofMapReport {
    let limit = limit.max(1);
    let scope = scope.map(|value| repo::normalize_rel_path(&value));
    let (seeds, hidden_seed_count) = proof_map_seed_selection(project, scope.as_deref(), &changed, raw_sensors);
    let route_index_paths = proof_map_route_index_paths(project, scope.as_deref(), &seeds);
    let mut direct = Vec::new();
    let mut indirect = Vec::new();
    let mut e2e = Vec::new();
    let mut contract = Vec::new();
    let mut missing_direct = Vec::new();
    let mut unknowns = Vec::new();
    let mut scope_expand = Vec::new();
    let discovery_limit = usize::MAX;
    let mut hidden = Vec::new();
    let runtime_facts = runtime_fact_index_for_paths(project, &route_index_paths);
    let expand_larger_limit = proof_map_expand(&proof_selector, false);
    let expand_raw_sensors = proof_map_expand(&proof_selector, true);
    if hidden_seed_count > 0 {
        hidden.push(HiddenGroup {
            reason: "recursive proof seeds hidden at root scope".to_string(),
            count: hidden_seed_count,
            expand: expand_with_concrete_limit(&expand_raw_sensors, seeds.len() + hidden_seed_count),
        });
    }
    let (current_direct, current_e2e) =
        proof_map_current_level_containers(project, scope.as_deref(), raw_sensors);
    direct.extend(current_direct);
    e2e.extend(current_e2e);
    if scope.is_none() && !changed.is_empty() {
        direct.extend(ctx_changed_proof_surfaces(project));
    }
    for seed in &seeds {
        if let Some(file) = project.files.get(seed) {
            unknowns.extend(unknowns_for_file(project, file));
            let file_routes = runtime_facts.routes_for_file(seed);
            e2e.extend(route_proof_surfaces_for_routes(
                project,
                file_routes.clone(),
                &runtime_facts,
            ));
            unknowns.extend(route_proof_unknowns_for_routes(
                project,
                file_routes,
                &runtime_facts,
            ));
        }
        let proofs = proof_surfaces_for_anchor(project, seed, 1, discovery_limit);
        let has_specific_proof = proofs.iter().any(proof_surface_satisfies_specific_proof);
        if !has_specific_proof
            && proof_map_missing_should_surface(project, seed, scope.as_deref(), &changed)
        {
            missing_direct.push(surface_from_path(
                "missing_direct_proof",
                seed,
                "no_structural_proof_surface",
                EvidenceStrength::Medium,
            ));
            if !proofs.is_empty() {
                unknowns.push(unknown_missing_deterministic_proof(
                    seed,
                    format!("codemap proof-map {}", shell_quote(seed)),
                ));
            }
        }
        if scope.is_none()
            && let Some(unknown) =
                proof_map_changed_scope_repair_unknown(project, seed, &changed, &proofs)
        {
            unknowns.push(unknown);
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
    if let Some((unknown, expand)) =
        proof_map_exact_scope_repair(project, scope.as_deref(), &direct, &indirect, &e2e, &contract)
    {
        unknowns.push(unknown);
        scope_expand.push(expand);
    }
    if !raw_sensors {
        group_duplicate_proof_surfaces(
            &mut direct,
            &mut hidden,
            "duplicate direct proof sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_proof_surfaces(
            &mut indirect,
            &mut hidden,
            "duplicate indirect proof sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_proof_surfaces(
            &mut e2e,
            &mut hidden,
            "duplicate e2e proof sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_proof_surfaces(
            &mut contract,
            &mut hidden,
            "duplicate contract proof sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_missing_surfaces(
            &mut missing_direct,
            &mut hidden,
            "duplicate missing direct proof surfaces grouped by path",
            &expand_raw_sensors,
        );
        group_duplicate_unknowns(
            &mut unknowns,
            &mut hidden,
            "duplicate proof-map unknowns grouped by structural key",
            &expand_raw_sensors,
        );
    }
    truncate_with_hidden(
        &mut direct,
        limit,
        &mut hidden,
        "direct proof surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut indirect,
        limit,
        &mut hidden,
        "indirect proof surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut e2e,
        limit,
        &mut hidden,
        "e2e proof surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut contract,
        limit,
        &mut hidden,
        "contract proof surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut missing_direct,
        limit,
        &mut hidden,
        "missing direct proof surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut unknowns,
        limit,
        &mut hidden,
        "proof-map unknowns hidden by limit",
        &expand_larger_limit,
    );
    let commands = unique_proof_commands(
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
    let proof_expand = proof_map_proof_expand(&proof_selector);
    let mut expand = vec![proof_expand];
    expand.extend(scope_expand);
    ProofMapReport {
        kind: "proof_map_report",
        schema_version: "2",
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
        expand,
    }
}

fn proof_map_seed_selection(
    project: &Project,
    scope: Option<&str>,
    changed: &[String],
    raw_sensors: bool,
) -> (Vec<String>, usize) {
    let Some(scope) = scope else {
        return (changed.to_vec(), 0);
    };
    if !directory_has_files(project, scope) {
        return (vec![scope.to_string()], 0);
    }
    let all = files_under_directory(project, scope)
        .into_iter()
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    if scope != "." || raw_sensors {
        return (all, 0);
    }
    let mut seeds = direct_files_under_directory(project, scope)
        .into_iter()
        .filter(|file| !file.has_role("generated") && !is_generic_noise(file))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    seeds.sort();
    let hidden = all.len().saturating_sub(seeds.len());
    (seeds, hidden)
}

fn proof_map_missing_should_surface(
    project: &Project,
    seed: &str,
    scope: Option<&str>,
    changed: &[String],
) -> bool {
    if !proof_missing_should_surface(project, seed) {
        return false;
    }
    if changed.iter().any(|path| path == seed) || scope.is_some_and(|scope| scope == seed) {
        return true;
    }
    !project.packages.iter().any(|package| package.manifest == seed)
}

fn group_env_surfaces(values: Vec<EnvSurface>) -> Vec<EnvSurface> {
    let mut seen: BTreeMap<(String, String, String, String), usize> = BTreeMap::new();
    let mut out: Vec<EnvSurface> = Vec::new();
    for value in values {
        let key = (
            value.name.clone(),
            value.used_by.clone(),
            value.declaration.clone().unwrap_or_default(),
            value.evidence.clone(),
        );
        if let Some(index) = seen.get(&key).copied() {
            if value.strength > out[index].strength {
                out[index].strength = value.strength;
            }
            out[index].locations.extend(value.locations);
        } else {
            seen.insert(key, out.len());
            out.push(value);
        }
    }
    out
}
