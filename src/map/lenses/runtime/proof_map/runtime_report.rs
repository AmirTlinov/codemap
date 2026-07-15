// Responsibility: runtime-report-assembly
use crate::map::{
    RuntimeGroupObservationInput, RuntimeGroupVisibility, RuntimeRouteObservationInput,
    dedupe_runtime_entrypoints, directory_has_files, env_surfaces_for_file,
    expand_with_concrete_limit, extend_root_nested_routes, files_under_directory,
    group_env_surfaces, limit_edge_section, record_runtime_group_observations,
    root_runtime_containers, route_reference_edges_with_index, runtime_code_entrypoints,
    runtime_entrypoint_kind, runtime_expand_commands, runtime_fact_index_for_files,
    runtime_manifest_entrypoints, runtime_route_observations, runtime_scope_files,
    runtime_worker_or_job_convention, shell_quote, surface_from_path, unknowns_for_file,
};
use crate::model::{EvidenceStrength, HiddenGroup, Project, RuntimeReport, Surface};
use crate::repo;

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
    let scope_indexed = scope == "."
        || project.files.contains_key(&scope)
        || directory_has_files(project, &scope)
        || std::fs::symlink_metadata(project.root.join(&scope))
            .is_ok_and(|metadata| metadata.is_dir());
    let route_scope_files = if scope == "." && !include_hidden {
        files_under_directory(project, ".")
    } else {
        scope_files.clone()
    };
    let route_candidate_files = route_scope_files
        .iter()
        .copied()
        .filter(|file| crate::repo::is_source_ext(&file.ext))
        .collect::<Vec<_>>();
    let route_scope_unknowns = route_scope_files
        .iter()
        .flat_map(|file| unknowns_for_file(project, file))
        .collect::<Vec<_>>();
    let runtime_facts = runtime_fact_index_for_files(project, scope_files.iter().copied());
    let root_containers = if scope == "." && !include_hidden {
        root_runtime_containers(project)
    } else {
        Vec::new()
    };
    for &file in &scope_files {
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
        unknowns.extend(
            route_scope_unknowns
                .iter()
                .filter(|unknown| unknown.path.as_deref() == Some(file.rel.as_str()))
                .cloned(),
        );
    }
    if scope == "." && !include_hidden && hidden_scope_count > 0 {
        extend_root_nested_routes(project, &scope_files, &mut routes, &mut proof);
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
    let entrypoints_visibility =
        truncate_for_horizon(&mut entrypoints, limit, &include_hidden_expand);
    let observed_routes = routes.len();
    let route_expand = (observed_routes > limit)
        .then(|| expand_with_concrete_limit(&include_hidden_expand, observed_routes));
    routes.truncate(limit);
    let mut observations = runtime_route_observations(
        project,
        RuntimeRouteObservationInput {
            scope: &scope,
            scope_indexed,
            candidate_files: &route_candidate_files,
            observed: observed_routes,
            shown: routes.len(),
            route_unknowns: &route_scope_unknowns,
            expand: route_expand,
        },
    );
    let env_visibility = truncate_for_horizon(&mut env, limit, &include_hidden_expand);
    let scripts_visibility = truncate_for_horizon(&mut scripts, limit, &include_hidden_expand);
    let workers_visibility = truncate_for_horizon(&mut workers, limit, &include_hidden_expand);
    let ci_visibility = truncate_for_horizon(&mut ci, limit, &include_hidden_expand);
    let unknowns_visibility = truncate_for_horizon(&mut unknowns, limit, &include_hidden_expand);
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
    let observed_proof = proof.len();
    let mut discarded_proof_hidden = Vec::new();
    limit_edge_section(
        &mut proof,
        &mut discarded_proof_hidden,
        include_hidden,
        limit,
        "runtime verification edges hidden by limit",
        &include_hidden_expand,
    );
    let proof_visibility = RuntimeGroupVisibility {
        observed: observed_proof,
        shown: proof.len(),
        expand: (observed_proof > proof.len())
            .then(|| expand_with_concrete_limit(&include_hidden_expand, observed_proof)),
    };
    record_runtime_group_observations(
        project,
        RuntimeGroupObservationInput {
            scope: &scope,
            scope_indexed,
            scope_files: &scope_files,
            hidden_scope_count,
            scope_unknowns: &route_scope_unknowns,
            entrypoints: entrypoints_visibility,
            scripts: scripts_visibility,
            env: env_visibility,
            workers: workers_visibility,
            ci: ci_visibility,
            proof: proof_visibility,
            unknowns: unknowns_visibility,
        },
        &mut observations,
    );
    let expand = runtime_expand_commands(&scope, &root_containers, &entrypoints);
    RuntimeReport {
        kind: "runtime_report",
        schema_version: RuntimeReport::SCHEMA_VERSION,
        scope: scope.clone(),
        entrypoints,
        routes,
        scripts,
        env,
        workers,
        ci,
        proof,
        unknowns,
        observations,
        hidden,
        expand,
    }
}

/// The horizon, not a detached hidden group, owns per-group truncation truth.
fn truncate_for_horizon<T>(
    values: &mut Vec<T>,
    limit: usize,
    expand: &str,
) -> RuntimeGroupVisibility {
    let observed = values.len();
    values.truncate(limit);
    RuntimeGroupVisibility {
        observed,
        shown: values.len(),
        expand: (observed > values.len()).then(|| expand_with_concrete_limit(expand, observed)),
    }
}
