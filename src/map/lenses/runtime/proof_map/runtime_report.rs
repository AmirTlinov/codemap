// Responsibility: runtime-report-assembly
use super::fact_groups::runtime_non_route_facts;
use crate::map::{
    RuntimeGroupObservationInput, RuntimeGroupProjection, RuntimeRouteObservationInput,
    dedupe_runtime_entrypoints, directory_has_files, expand_with_concrete_limit,
    group_env_surfaces, root_runtime_containers, route_reference_edges_with_index,
    runtime_expand_commands, runtime_fact_index_for_files, runtime_group_observations,
    runtime_route_observations, runtime_scope_files, shell_quote,
};
use crate::model::{HiddenGroup, Project, RuntimeReport, StructuralEdge, Surface};
use crate::repo;

pub fn runtime_report(
    project: &Project,
    scope: &str,
    include_hidden: bool,
    limit: usize,
) -> RuntimeReport {
    let limit = limit.max(1);
    let scope = repo::normalize_rel_path(scope);
    let observation_snapshot = crate::cache::runtime_scope_fingerprint(project, &scope);
    let (display_scope_files, hidden_scope_count) =
        runtime_scope_files(project, &scope, include_hidden);
    let scope_logically_empty =
        crate::cache::runtime_scope_is_logically_empty(&project.root, &scope);
    let scope_has_unindexed_entries =
        crate::cache::runtime_scope_has_unindexed_entries(project, &scope);
    let scope_indexed = scope == "."
        || project.files.contains_key(&scope)
        || directory_has_files(project, &scope)
        || scope_logically_empty;
    let (full_scope_files, _) = runtime_scope_files(project, &scope, true);
    let incomplete_boundaries = full_scope_files
        .iter()
        .copied()
        .filter(|file| crate::repo::is_incomplete_indexed_boundary(&project.root, file))
        .collect::<Vec<_>>();
    let external_boundaries = incomplete_boundaries
        .iter()
        .copied()
        .filter(|file| crate::repo::is_external_tree_boundary(&project.root, file))
        .collect::<Vec<_>>();

    let mut full_groups = runtime_non_route_facts(project, &full_scope_files);
    full_groups.env = group_env_surfaces(full_groups.env);
    let observed_entrypoints = surface_fact_count(&full_groups.entrypoints);
    let observed_env = full_groups.env.len();
    let observed_workers = full_groups.workers.len();
    let observed_ci = full_groups.ci.len();
    let observed_unknowns = full_groups.unknowns.len();
    let coverage_unknowns = full_groups.unknowns.clone();

    let root_containers = if scope == "." && !include_hidden {
        root_runtime_containers(project, &display_scope_files)
    } else {
        Vec::new()
    };
    let (mut entrypoints, mut env, mut workers, mut ci, mut unknowns) =
        if scope == "." && !include_hidden {
            let mut display_groups = runtime_non_route_facts(project, &display_scope_files);
            display_groups.entrypoints.extend(root_containers.clone());
            display_groups.entrypoints = dedupe_runtime_entrypoints(display_groups.entrypoints);
            display_groups.env = group_env_surfaces(display_groups.env);
            (
                display_groups.entrypoints,
                display_groups.env,
                display_groups.workers,
                display_groups.ci,
                display_groups.unknowns,
            )
        } else {
            (
                full_groups.entrypoints,
                full_groups.env,
                full_groups.workers,
                full_groups.ci,
                full_groups.unknowns,
            )
        };

    let route_candidate_files = full_scope_files
        .iter()
        .copied()
        .filter(|file| {
            crate::repo::is_source_ext(&file.ext)
                || crate::repo::is_incomplete_indexed_boundary(&project.root, file)
        })
        .collect::<Vec<_>>();
    let runtime_facts = runtime_fact_index_for_files(project, full_scope_files.iter().copied());
    let (mut routes, mut proof) = route_facts(project, &full_scope_files, &runtime_facts);
    let observed_routes = routes.len();
    let observed_proof = proof.len();

    let mut scripts = runtime_scripts(project, &scope);
    let observed_scripts = scripts.len();
    let include_hidden_expand = format!("codemap runtime {} --all", shell_quote(&scope));

    entrypoints.truncate(limit);
    routes.truncate(limit);
    scripts.truncate(limit);
    env.truncate(limit);
    workers.truncate(limit);
    ci.truncate(limit);
    proof.truncate(limit);
    unknowns.truncate(limit);

    let entrypoints_shown = surface_fact_count(&entrypoints);
    let route_observations = runtime_route_observations(RuntimeRouteObservationInput {
        scope: &scope,
        snapshot: &observation_snapshot,
        scope_indexed,
        scope_has_unindexed_entries,
        candidate_files: &route_candidate_files,
        incomplete_boundaries: &incomplete_boundaries,
        external_boundaries: &external_boundaries,
        scan_boundaries: &project.scan_stats.inventory_boundaries,
        observed: observed_routes,
        shown: routes.len(),
        route_unknowns: &coverage_unknowns,
        expand: horizon_expand(&include_hidden_expand, observed_routes, routes.len()),
    });
    let group_observations = runtime_group_observations(
        project,
        RuntimeGroupObservationInput {
            scope: &scope,
            snapshot: &observation_snapshot,
            scope_indexed,
            scope_logically_empty,
            scope_has_unindexed_entries,
            candidate_files: &full_scope_files,
            incomplete_boundaries: &incomplete_boundaries,
            external_boundaries: &external_boundaries,
            scan_boundaries: &project.scan_stats.inventory_boundaries,
            visited_files: &full_scope_files,
            unknowns: &coverage_unknowns,
            route_observations: &route_observations,
            projections: vec![
                projection(
                    "entrypoints",
                    observed_entrypoints,
                    entrypoints_shown,
                    &include_hidden_expand,
                ),
                projection(
                    "scripts",
                    observed_scripts,
                    scripts.len(),
                    &include_hidden_expand,
                ),
                projection("env", observed_env, env.len(), &include_hidden_expand),
                projection(
                    "workers",
                    observed_workers,
                    workers.len(),
                    &include_hidden_expand,
                ),
                projection("ci", observed_ci, ci.len(), &include_hidden_expand),
                projection("proof", observed_proof, proof.len(), &include_hidden_expand),
                projection(
                    "unknowns",
                    observed_unknowns,
                    unknowns.len(),
                    &include_hidden_expand,
                ),
            ],
        },
    );
    let mut observations = route_observations;
    observations.extend(&group_observations);

    let mut hidden = Vec::new();
    if hidden_scope_count > 0 {
        hidden.push(HiddenGroup {
            reason: RuntimeReport::ROOT_RECURSIVE_HIDDEN_REASON.to_string(),
            count: hidden_scope_count,
            expand: include_hidden_expand.clone(),
        });
    }
    let expand = runtime_expand_commands(&scope, &root_containers, &entrypoints);
    RuntimeReport {
        kind: "runtime_report",
        schema_version: RuntimeReport::SCHEMA_VERSION,
        scope,
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

fn route_facts(
    project: &Project,
    files: &[&crate::model::FileInfo],
    facts: &crate::map::RuntimeFactIndex,
) -> (Vec<crate::model::RuntimeRoute>, Vec<StructuralEdge>) {
    let mut routes = Vec::new();
    let mut proof = Vec::new();
    for &file in files {
        let file_routes = facts.routes_for_file(&file.rel);
        for route in &file_routes {
            proof.extend(route_reference_edges_with_index(project, route, facts));
        }
        routes.extend(file_routes);
    }
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
    (routes, proof)
}

fn runtime_scripts(project: &Project, scope: &str) -> Vec<Surface> {
    if scope != "." {
        return Vec::new();
    }
    project
        .scripts
        .iter()
        .map(|script| Surface {
            id: format!("surface:script:{}", script.name),
            kind: "script".to_string(),
            path: script.path.clone(),
            role: Some("script".to_string()),
            evidence: script.reason.clone(),
            strength: crate::model::EvidenceStrength::Hard,
            count: Some(1),
            examples: vec![format!("{}: {}", script.name, script.command)],
            hidden_count: 0,
        })
        .collect()
}

fn projection(
    group: &'static str,
    observed: usize,
    shown: usize,
    expand: &str,
) -> RuntimeGroupProjection {
    RuntimeGroupProjection {
        group,
        observed,
        shown,
        expand: horizon_expand(expand, observed, shown),
    }
}

fn horizon_expand(expand: &str, observed: usize, shown: usize) -> Option<String> {
    (shown < observed).then(|| expand_with_concrete_limit(expand, observed))
}

fn surface_fact_count(surfaces: &[Surface]) -> usize {
    surfaces
        .iter()
        .map(|surface| surface.count.unwrap_or(1))
        .sum()
}
