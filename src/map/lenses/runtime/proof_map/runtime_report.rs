// Responsibility: runtime-report-assembly
use crate::map::{
    dedupe_runtime_entrypoints, env_surfaces_for_file, extend_root_nested_routes,
    group_env_surfaces, limit_edge_section, root_runtime_containers,
    route_reference_edges_with_index, runtime_code_entrypoints, runtime_entrypoint_kind,
    runtime_expand_commands, runtime_fact_index_for_files, runtime_manifest_entrypoints,
    runtime_scope_files, runtime_worker_or_job_convention, shell_quote, surface_from_path,
    truncate_with_hidden, unknowns_for_file,
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
        unknowns.extend(unknowns_for_file(project, file));
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
        "runtime verification edges hidden by limit",
        &include_hidden_expand,
    );
    let expand = runtime_expand_commands(&scope, &root_containers, &entrypoints);
    RuntimeReport {
        kind: "runtime_report",
        schema_version: "2",
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
