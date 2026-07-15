// Responsibility: proof-map-report-assembly
use crate::map::{
    VerificationTopologyInput, codemap_changed_proof_surfaces, dedupe_unknowns,
    expand_with_concrete_limit, group_duplicate_missing_surfaces, group_duplicate_proof_surfaces,
    group_duplicate_unknowns, proof_fallback_commands, proof_map_changed_scope_repair_unknown,
    proof_map_current_level_containers, proof_map_discovery_limit, proof_map_exact_scope_repair,
    proof_map_expand, proof_map_missing_should_surface, proof_map_proof_expand,
    proof_map_route_index_paths, proof_map_seed_selection, proof_surface_satisfies_specific_proof,
    proof_surfaces_for_anchor, proof_target_owner_unknowns, proof_wiring_facts_limited,
    proof_wiring_unknowns, push_unique_proof_wiring, route_proof_surfaces_for_routes,
    route_proof_unknowns_for_routes, runtime_fact_index_for_paths, shell_quote,
    sort_proof_wiring_facts, surface_from_path, truncate_with_hidden, unique_proof_commands,
    unknown_missing_deterministic_proof, unknowns_for_file, verification_topology,
};
use crate::model::{EvidenceStrength, HiddenGroup, Project, ProofMapReport};
use crate::repo;
use std::collections::BTreeSet;

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
    let (seeds, hidden_seed_count) =
        proof_map_seed_selection(project, scope.as_deref(), &changed, raw_sensors, limit);
    let route_index_paths = proof_map_route_index_paths(project, scope.as_deref(), &seeds);
    let mut surfaces = Vec::new();
    let mut missing_direct = Vec::new();
    let mut unknowns = Vec::new();
    let mut scope_expand = Vec::new();
    let discovery_limit = proof_map_discovery_limit(scope.as_deref(), &changed, limit, raw_sensors);
    let mut hidden = Vec::new();
    let mut wiring = Vec::new();
    let mut wiring_seen = BTreeSet::new();
    let mut global_wiring_surfaces = Vec::new();
    let wiring_discovery_limit = if raw_sensors {
        usize::MAX
    } else {
        limit.saturating_mul(2).max(6)
    };
    let mut wiring_clipped = false;
    let runtime_facts = runtime_fact_index_for_paths(project, &route_index_paths);
    let expand_larger_limit = proof_map_expand(&proof_selector, raw_sensors);
    let expand_raw_sensors = proof_map_expand(&proof_selector, true);
    if hidden_seed_count > 0 {
        let compact_changed_seed_gate = scope.is_none() && !changed.is_empty() && !raw_sensors;
        let expand = if compact_changed_seed_gate {
            &expand_larger_limit
        } else {
            &expand_raw_sensors
        };
        hidden.push(HiddenGroup {
            reason: if compact_changed_seed_gate {
                "changed proof seeds hidden by compact limit".to_string()
            } else {
                "recursive proof seeds hidden at root scope".to_string()
            },
            count: hidden_seed_count,
            expand: expand_with_concrete_limit(expand, seeds.len() + hidden_seed_count),
        });
    }
    let (current_direct, current_e2e) =
        proof_map_current_level_containers(project, scope.as_deref(), raw_sensors);
    global_wiring_surfaces.extend(current_direct.iter().cloned());
    global_wiring_surfaces.extend(current_e2e.iter().cloned());
    surfaces.extend(current_direct);
    surfaces.extend(current_e2e);
    if scope.is_none() && !changed.is_empty() {
        let changed_surfaces = codemap_changed_proof_surfaces(project);
        global_wiring_surfaces.extend(changed_surfaces.iter().cloned());
        surfaces.extend(changed_surfaces);
    }
    for seed in &seeds {
        if let Some(file) = project.files.get(seed) {
            unknowns.extend(unknowns_for_file(project, file));
            unknowns.extend(proof_target_owner_unknowns(project, seed, file));
            let file_routes = runtime_facts.routes_for_file(seed);
            let route_surfaces =
                route_proof_surfaces_for_routes(project, file_routes.clone(), &runtime_facts);
            let (facts, clipped) = proof_wiring_facts_limited(
                project,
                std::slice::from_ref(seed),
                &[],
                &route_surfaces,
                &[],
                wiring_discovery_limit.saturating_sub(wiring.len()),
            );
            wiring_clipped |= clipped;
            push_unique_proof_wiring(&mut wiring, &mut wiring_seen, facts);
            surfaces.extend(route_surfaces);
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
        let (facts, clipped) = proof_wiring_facts_limited(
            project,
            std::slice::from_ref(seed),
            &[],
            &proofs,
            &[],
            wiring_discovery_limit.saturating_sub(wiring.len()),
        );
        wiring_clipped |= clipped;
        push_unique_proof_wiring(&mut wiring, &mut wiring_seen, facts);
        for proof in proofs {
            surfaces.push(proof);
        }
    }
    if let Some((unknown, expand)) =
        proof_map_exact_scope_repair(project, scope.as_deref(), &surfaces)
    {
        unknowns.push(unknown);
        scope_expand.push(expand);
    }
    let (facts, clipped) = proof_wiring_facts_limited(
        project,
        &seeds,
        &changed,
        &global_wiring_surfaces,
        &[],
        wiring_discovery_limit.saturating_sub(wiring.len()),
    );
    wiring_clipped |= clipped;
    push_unique_proof_wiring(&mut wiring, &mut wiring_seen, facts);
    let mut hard = Vec::new();
    let mut direct_evidence = Vec::new();
    let mut mediated_evidence = Vec::new();
    let mut soft_evidence = Vec::new();
    let mut setup_support = Vec::new();
    for proof in surfaces {
        match crate::proof_classification::proof_surface_class(&proof) {
            crate::proof_classification::ProofSurfaceClass::Hard => hard.push(proof),
            crate::proof_classification::ProofSurfaceClass::DirectEvidence => {
                direct_evidence.push(proof)
            }
            crate::proof_classification::ProofSurfaceClass::MediatedEvidence => {
                mediated_evidence.push(proof)
            }
            crate::proof_classification::ProofSurfaceClass::SoftEvidence => {
                soft_evidence.push(proof)
            }
            crate::proof_classification::ProofSurfaceClass::SetupSupport => {
                setup_support.push(proof)
            }
        }
    }
    dedupe_unknowns(&mut unknowns);
    if !raw_sensors {
        group_duplicate_proof_surfaces(
            &mut hard,
            &mut hidden,
            "duplicate runnable verification sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_proof_surfaces(
            &mut direct_evidence,
            &mut hidden,
            "duplicate direct linked sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_proof_surfaces(
            &mut mediated_evidence,
            &mut hidden,
            "duplicate mediated linked sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_proof_surfaces(
            &mut soft_evidence,
            &mut hidden,
            "duplicate soft surface match sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_proof_surfaces(
            &mut setup_support,
            &mut hidden,
            "duplicate setup/support verification sensors grouped by structural key",
            &expand_raw_sensors,
        );
        group_duplicate_missing_surfaces(
            &mut missing_direct,
            &mut hidden,
            "duplicate missing direct linked surfaces grouped by path",
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
        &mut hard,
        limit,
        &mut hidden,
        "runnable verification surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut direct_evidence,
        limit,
        &mut hidden,
        "direct linked surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut mediated_evidence,
        limit,
        &mut hidden,
        "mediated linked surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut soft_evidence,
        limit,
        &mut hidden,
        "soft surface matches hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut setup_support,
        limit,
        &mut hidden,
        "setup/support verification surfaces hidden by limit",
        &expand_larger_limit,
    );
    truncate_with_hidden(
        &mut missing_direct,
        limit,
        &mut hidden,
        "missing direct linked surfaces hidden by limit",
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
        hard.iter()
            .filter(|proof| proof.command.is_some())
            .cloned()
            .collect(),
    );
    let fallback_sources = hard
        .iter()
        .chain(direct_evidence.iter())
        .chain(mediated_evidence.iter())
        .chain(soft_evidence.iter())
        .chain(setup_support.iter())
        .cloned()
        .collect::<Vec<_>>();
    let fallback = proof_fallback_commands(project, &seeds, &changed, &fallback_sources);
    let (facts, clipped) = proof_wiring_facts_limited(
        project,
        &seeds,
        &changed,
        &[],
        &fallback,
        wiring_discovery_limit.saturating_sub(wiring.len()),
    );
    wiring_clipped |= clipped;
    push_unique_proof_wiring(&mut wiring, &mut wiring_seen, facts);
    sort_proof_wiring_facts(&mut wiring);
    if wiring_clipped {
        hidden.push(HiddenGroup {
            reason: "verification wiring facts hidden by discovery limit".to_string(),
            count: 1,
            expand: expand_with_concrete_limit(
                &expand_larger_limit,
                wiring_discovery_limit.saturating_mul(2),
            ),
        });
    }
    if scope.as_deref() != Some(".") {
        unknowns.extend(proof_wiring_unknowns(&wiring));
    }
    truncate_with_hidden(
        &mut wiring,
        limit.saturating_mul(2).max(6),
        &mut hidden,
        "verification wiring facts hidden by limit",
        &expand_larger_limit,
    );
    let proof_expand = proof_map_proof_expand(&proof_selector);
    let mut expand = vec![proof_expand];
    expand.extend(scope_expand);
    let topology_proofs = hard
        .iter()
        .chain(&direct_evidence)
        .chain(&mediated_evidence)
        .chain(&soft_evidence)
        .chain(&setup_support)
        .cloned()
        .collect::<Vec<_>>();
    let verification_topology = verification_topology(VerificationTopologyInput {
        project,
        proofs: &topology_proofs,
        missing: &missing_direct,
        wiring: &wiring,
        unknowns: &unknowns,
        hidden: &hidden,
        expand: &expand,
    });
    ProofMapReport {
        kind: "proof_map_report",
        schema_version: "7",
        selector: proof_selector,
        scope,
        changed,
        hard,
        direct_evidence,
        mediated_evidence,
        soft_evidence,
        setup_support,
        missing_direct,
        commands,
        wiring,
        verification_topology,
        fallback,
        unknowns,
        hidden,
        expand,
    }
}
