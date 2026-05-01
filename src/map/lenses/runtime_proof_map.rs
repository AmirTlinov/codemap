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
    for file in runtime_scope_files(project, &scope) {
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
    let include_hidden_expand = format!("codemap runtime {} --include-hidden", shell_quote(&scope));
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
        &mut unknowns,
        limit,
        &mut hidden,
        "runtime unknowns hidden by limit",
        &include_hidden_expand,
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
    raw_sensors: bool,
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
            e2e.extend(route_proof_surfaces(project, file));
            unknowns.extend(route_proof_unknowns(project, file));
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
    let expand_larger_limit = proof_map_larger_limit_expand(&scope, &changed);
    let expand_raw_sensors = proof_map_raw_sensors_expand(&scope, &changed);
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
        expand: vec!["codemap proof --changed".to_string()],
    }
}

fn runtime_scope_files<'a>(project: &'a Project, scope: &str) -> Vec<&'a FileInfo> {
    if let Some(file) = project.files.get(scope) {
        vec![file]
    } else {
        files_under_directory(project, scope)
    }
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

fn route_proof_surfaces(project: &Project, file: &FileInfo) -> Vec<ProofSurface> {
    runtime_routes_for_file(project, file)
        .into_iter()
        .flat_map(|route| {
            let label = route_anchor_label(&route);
            route_reference_edges(project, &route)
                .into_iter()
                .map(move |edge| ProofSurface {
                    command: proof_command_for_test(project, &edge.from),
                    path: Some(edge.from),
                    evidence: edge.evidence,
                    strength: edge.strength,
                    reason: format!("e2e visits runtime route {label}"),
                    locations: edge.locations,
                })
        })
        .collect()
}

fn route_proof_unknowns(project: &Project, file: &FileInfo) -> Vec<Unknown> {
    runtime_routes_for_file(project, file)
        .into_iter()
        .filter(|route| {
            route_can_be_proved_by_page_goto(route)
                && route_has_page_visit_in_proof_scope(project, route)
                && route_page_visit_owner_count(project, route) > 1
        })
        .map(|route| {
            let line = route
                .locations
                .first()
                .and_then(|location| location.line_start);
            unknown(
                "ambiguous_route_visit_owner",
                Some(route.file.clone()),
                line,
                format!(
                    "runtime route `{}` has multiple method-compatible owners in this proof scope",
                    route_anchor_label(&route)
                ),
                "page.goto route visits are not attached as e2e proof because the owner is ambiguous",
                Some(format!("codemap runtime {}", shell_quote(&route.file))),
            )
        })
        .collect()
}

fn group_duplicate_proof_surfaces(
    values: &mut Vec<ProofSurface>,
    hidden: &mut Vec<HiddenGroup>,
    reason: &str,
    expand: &str,
) {
    let mut seen = BTreeMap::new();
    let mut out = Vec::new();
    let mut duplicate_count = 0usize;
    for value in values.drain(..) {
        let key = proof_surface_group_key(&value);
        if let Some(index) = seen.get(&key).copied() {
            duplicate_count += 1;
            if proof_surface_precedence(&value) > proof_surface_precedence(&out[index]) {
                out[index] = value;
            }
        } else {
            seen.insert(key, out.len());
            out.push(value);
        }
    }
    if duplicate_count > 0 {
        hidden.push(HiddenGroup {
            reason: reason.to_string(),
            count: duplicate_count,
            expand: expand.to_string(),
        });
    }
    *values = out;
}

fn proof_surface_group_key(value: &ProofSurface) -> (String, String, String) {
    let detail = if value.evidence == "e2e_visited_route" {
        value.reason.clone()
    } else {
        String::new()
    };
    (
        value.command.clone().unwrap_or_default(),
        value.path.clone().unwrap_or_default(),
        detail,
    )
}

fn group_duplicate_missing_surfaces(
    values: &mut Vec<Surface>,
    hidden: &mut Vec<HiddenGroup>,
    reason: &str,
    expand: &str,
) {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut duplicate_count = 0usize;
    for value in values.drain(..) {
        let key = (
            value.kind.clone(),
            value.path.clone().unwrap_or_default(),
            value.evidence.clone(),
        );
        if seen.insert(key) {
            out.push(value);
        } else {
            duplicate_count += 1;
        }
    }
    if duplicate_count > 0 {
        hidden.push(HiddenGroup {
            reason: reason.to_string(),
            count: duplicate_count,
            expand: expand.to_string(),
        });
    }
    *values = out;
}

fn group_duplicate_unknowns(
    values: &mut Vec<Unknown>,
    hidden: &mut Vec<HiddenGroup>,
    reason: &str,
    expand: &str,
) {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut duplicate_count = 0usize;
    for value in values.drain(..) {
        let key = (
            value.kind.clone(),
            value.path.clone().unwrap_or_default(),
            value.line_start.unwrap_or_default(),
            value.reason.clone(),
            value.effect.clone(),
        );
        if seen.insert(key) {
            out.push(value);
        } else {
            duplicate_count += 1;
        }
    }
    if duplicate_count > 0 {
        hidden.push(HiddenGroup {
            reason: reason.to_string(),
            count: duplicate_count,
            expand: expand.to_string(),
        });
    }
    *values = out;
}

fn proof_map_larger_limit_expand(scope: &Option<String>, changed: &[String]) -> String {
    proof_map_expand(scope, changed, false)
}

fn proof_map_raw_sensors_expand(scope: &Option<String>, changed: &[String]) -> String {
    proof_map_expand(scope, changed, true)
}

fn proof_map_expand(scope: &Option<String>, changed: &[String], raw_sensors: bool) -> String {
    let raw = if raw_sensors { " --raw-sensors" } else { "" };
    if let Some(scope) = scope {
        return format!(
            "codemap proof-map {}{raw} --limit <larger-number>",
            shell_quote(scope)
        );
    }
    if changed.is_empty() {
        return format!("codemap proof-map --changed{raw} --limit <larger-number>");
    }
    let files = changed
        .iter()
        .map(|file| shell_quote(file))
        .collect::<Vec<_>>()
        .join(",");
    format!("codemap proof-map --files {files}{raw} --limit <larger-number>")
}
