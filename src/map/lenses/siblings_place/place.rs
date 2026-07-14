// Responsibility: place-lens-report
use crate::evidence::import_statement_locations;
use crate::map::{
    directory_contract_edges_at_depth, directory_has_files, directory_seed_file_paths,
    file_matches_place_kind, files_under_directory, group_duplicate_proof_surfaces,
    nearest_proof_scope, nearest_proof_scope_unknown, placement_conventions,
    proof_command_for_test, proof_surface_locations_for_test, proof_surfaces_for_file_paths,
    runtime_fact_index, shell_quote, truncate_with_hidden,
};
use crate::model::{EvidenceStrength, PlaceReport, Project, ProofSurface, Surface};
use crate::repo;
use std::collections::BTreeSet;

pub fn place_report(
    project: &Project,
    scope: &str,
    requested_kind: &str,
    include_hidden: bool,
    limit: usize,
) -> PlaceReport {
    let limit = limit.max(1);
    let scope = repo::normalize_rel_path(scope);
    let requested_kind = requested_kind.to_string();
    let runtime_facts = (requested_kind == "route").then(|| runtime_fact_index(project));
    let mut matched_files = files_under_directory(project, &scope)
        .into_iter()
        .filter(|file| {
            file_matches_place_kind(file, &requested_kind)
                || runtime_facts
                    .as_ref()
                    .is_some_and(|facts| facts.has_routes_for_file(&file.rel))
        })
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    matched_files.sort();
    let count = matched_files.len();
    let hidden_count = count.saturating_sub(limit);
    let mut visible_examples = matched_files.clone();
    if !include_hidden {
        visible_examples.truncate(limit);
    }
    let mut hidden = Vec::new();
    let proof_map_raw_expand = format!(
        "codemap proof-map {} --raw-sensors --limit <larger-number>",
        shell_quote(&scope)
    );
    let scoped_test_seed_files;
    let proof_seed_files = if requested_kind == "test" && matched_files.is_empty() {
        scoped_test_seed_files = directory_seed_file_paths(project, &scope, include_hidden);
        scoped_test_seed_files.as_slice()
    } else if include_hidden {
        matched_files.as_slice()
    } else {
        visible_examples.as_slice()
    };
    let mut paired_proof_pattern = if requested_kind == "test" && matched_files.is_empty() {
        direct_test_import_proof_surfaces_for_scope(project, proof_seed_files)
    } else {
        proof_surfaces_for_file_paths(project, proof_seed_files, 1, limit)
    };
    let proof_based_existing_surface = if requested_kind == "test" && matched_files.is_empty() {
        proof_sensor_surface_for_place_tests(
            &scope,
            &requested_kind,
            &paired_proof_pattern,
            include_hidden,
            limit,
        )
    } else {
        None
    };
    group_duplicate_proof_surfaces(
        &mut paired_proof_pattern,
        &mut hidden,
        "duplicate paired verification sensors grouped by structural key",
        &proof_map_raw_expand,
    );
    if !include_hidden {
        truncate_with_hidden(
            &mut paired_proof_pattern,
            limit,
            &mut hidden,
            "paired proof pattern surfaces hidden by limit",
            &format!(
                "codemap proof-map {} --limit <larger-number>",
                shell_quote(&scope)
            ),
        );
    }
    let existing_surfaces = if matched_files.is_empty() {
        proof_based_existing_surface.into_iter().collect()
    } else {
        vec![Surface {
            id: format!("surface:place:{scope}:{requested_kind}"),
            kind: requested_kind.clone(),
            path: None,
            role: Some("placement_convention".to_string()),
            evidence: "same_scope_kind_filter".to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(count),
            examples: visible_examples.clone(),
            hidden_count,
        }]
    };
    let local_conventions = placement_conventions(&scope, &requested_kind, &existing_surfaces);
    let include_hidden_expand = format!(
        "codemap place {} --kind {} --all",
        shell_quote(&scope),
        shell_quote(&requested_kind)
    );
    let mut shared_contracts =
        directory_contract_edges_at_depth(project, &scope, include_hidden, 1);
    truncate_with_hidden(
        &mut shared_contracts,
        limit,
        &mut hidden,
        "shared contract edges hidden by limit",
        &include_hidden_expand,
    );
    let mut unknowns = Vec::new();
    let mut expand = vec![
        format!("codemap siblings {}", shell_quote(&scope)),
        include_hidden_expand.clone(),
    ];
    if requested_kind == "test"
        && existing_surfaces.is_empty()
        && paired_proof_pattern.is_empty()
        && directory_has_files(project, &scope)
        && let Some(nearest) = nearest_proof_scope(project, &scope)
    {
        let command = format!(
            "codemap place {} --kind {}",
            shell_quote(&nearest),
            shell_quote(&requested_kind)
        );
        unknowns.push(nearest_proof_scope_unknown(
            &scope,
            &nearest,
            command.clone(),
        ));
        expand.push(command);
    }
    PlaceReport {
        kind: "place_report",
        schema_version: "2",
        scope: scope.clone(),
        requested_kind,
        existing_surfaces,
        local_conventions,
        paired_proof_pattern,
        shared_contracts,
        unknowns,
        hidden,
        expand,
    }
}

fn proof_sensor_surface_for_place_tests(
    scope: &str,
    requested_kind: &str,
    proofs: &[ProofSurface],
    include_hidden: bool,
    limit: usize,
) -> Option<Surface> {
    let mut examples = proofs
        .iter()
        .filter_map(|proof| proof.path.clone())
        .collect::<Vec<_>>();
    examples.sort();
    examples.dedup();
    if examples.is_empty() {
        return None;
    }
    let count = examples.len();
    let hidden_count = count.saturating_sub(limit);
    if !include_hidden {
        examples.truncate(limit);
    }
    let strength = proofs
        .iter()
        .map(|proof| proof.strength)
        .max()
        .unwrap_or(EvidenceStrength::Medium);
    Some(Surface {
        id: format!("surface:place:{scope}:{requested_kind}:proof-sensors"),
        kind: requested_kind.to_string(),
        path: None,
        role: Some("placement_convention".to_string()),
        evidence: "proof_sensor_for_scope".to_string(),
        strength,
        count: Some(count),
        examples,
        hidden_count,
    })
}

fn direct_test_import_proof_surfaces_for_scope(
    project: &Project,
    seed_files: &[String],
) -> Vec<ProofSurface> {
    let seeds = seed_files.iter().cloned().collect::<BTreeSet<_>>();
    if seeds.is_empty() {
        return Vec::new();
    }
    let mut out = project
        .files
        .values()
        .filter(|file| {
            file.has_role("test")
                && !file.has_role("test_support")
                && repo::is_source_ext(&file.ext)
        })
        .filter(|file| {
            file.resolved_imports
                .iter()
                .any(|target| seeds.contains(target))
        })
        .map(|file| {
            let target = file
                .resolved_imports
                .iter()
                .find(|target| seeds.contains(*target));
            let locations = target
                .map(|target| import_statement_locations(project, &file.rel, target))
                .filter(|locations| {
                    locations
                        .iter()
                        .any(|location| location.line_start.is_some())
                })
                .unwrap_or_else(|| proof_surface_locations_for_test(&file.rel, "test_import"));
            ProofSurface {
                command: proof_command_for_test(project, &file.rel),
                path: Some(file.rel.clone()),
                target_anchor: target.cloned(),
                evidence: "test_import".to_string(),
                strength: EvidenceStrength::High,
                reason: "test imports scope file".to_string(),
                locations,
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.command.cmp(&b.command))
            .then_with(|| a.evidence.cmp(&b.evidence))
    });
    out
}
