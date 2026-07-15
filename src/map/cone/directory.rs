// Responsibility: map-cone-directory
use crate::map::{
    ConeXrayInput, DirectoryConeObservationInput, ObservationProjection, add_directory_edge,
    boundary_findings, cone_xray_card, contract_evidence, dedupe_proof_edges_by_endpoint,
    directory_cone_observations, directory_edge_endpoint_at_depth, directory_edges_at_depth,
    e2e_test_visits_unique_route, edge_with_aggregate_location, edge_with_path_location,
    files_under_directory, import_edge, is_generic_noise, is_support_artifact_path,
    next_app_route_pattern, package_name_for_file, shell_quote, sort_edges,
    unknown_directory_aggregate,
};
use crate::model::CountFact;
use crate::model::{ConeReport, EvidenceStrength, FileSummary, Project, StructuralEdge};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(crate) fn cone_directory_report(
    project: &Project,
    rel: &str,
    depth: usize,
    include_hidden: bool,
    limit: usize,
) -> ConeReport {
    let depth = depth.max(1);
    let anchor = directory_file_summary(project, rel);
    let (complete_outgoing, complete_incoming) =
        split_directory_relations(directory_edges_at_depth(project, rel, true, depth));
    let (mut outgoing, mut incoming) = if include_hidden {
        (complete_outgoing.clone(), complete_incoming.clone())
    } else {
        split_directory_relations(directory_edges_at_depth(project, rel, false, depth))
    };
    let complete_proof = directory_proof_edges_at_depth(project, rel, true, depth);
    let complete_contracts = directory_contract_edges_at_depth(project, rel, true, depth);
    let complete_boundary = directory_boundary_edges_at_depth(project, rel, depth);
    let mut proof = if include_hidden {
        complete_proof.clone()
    } else {
        directory_proof_edges_at_depth(project, rel, false, depth)
    };
    let mut contracts = if include_hidden {
        complete_contracts.clone()
    } else {
        directory_contract_edges_at_depth(project, rel, false, depth)
    };
    let mut boundary = complete_boundary.clone();
    sort_edges(&mut outgoing);
    sort_edges(&mut incoming);
    sort_edges(&mut proof);
    sort_edges(&mut contracts);
    sort_edges(&mut boundary);
    let expand = || format!("codemap cone {} --depth {depth} --all", shell_quote(rel));
    if !include_hidden {
        outgoing = bounded_edges("outgoing", outgoing, limit, &expand());
        incoming = bounded_edges("incoming", incoming, limit, &expand());
        proof = bounded_edges("verification", proof, limit, &expand());
        contracts = bounded_edges("contracts", contracts, limit, &expand());
        boundary = bounded_edges("boundary", boundary, limit, &expand());
    }

    let unknowns = vec![unknown_directory_aggregate(rel, depth)];
    let declared_env = Vec::new();
    let seed_files = directory_seed_file_paths(project, rel, include_hidden);
    let xray = cone_xray_card(ConeXrayInput {
        project,
        anchor: &anchor,
        seed_files: &seed_files,
        declared_env: &declared_env,
        unknowns: &unknowns,
        limit,
        include_hidden,
    });
    let projection = |group, observed, shown| ObservationProjection {
        group,
        scope: rel,
        observed,
        shown,
        expand: (shown < observed).then(expand),
    };
    let observations = directory_cone_observations(
        project,
        DirectoryConeObservationInput {
            depth,
            outgoing: projection("outgoing", complete_outgoing.len(), outgoing.len()),
            incoming: projection("incoming", complete_incoming.len(), incoming.len()),
            verification: projection("verification", complete_proof.len(), proof.len()),
            contracts: projection("contracts", complete_contracts.len(), contracts.len()),
            boundary: projection("boundary", complete_boundary.len(), boundary.len()),
        },
    );
    ConeReport {
        kind: "cone_report",
        schema_version: crate::model::ConeReport::SCHEMA_VERSION,
        anchor,
        depth,
        xray,
        declared_env,
        outgoing,
        incoming,
        proof,
        contracts,
        boundary,
        observations,
        hidden: Vec::new(),
        unknowns,
        expand: vec![
            format!("codemap cone {} --depth {}", shell_quote(rel), depth + 1),
            format!("codemap ls {} --all", shell_quote(rel)),
        ],
    }
}

fn bounded_edges(
    group: &str,
    edges: Vec<StructuralEdge>,
    limit: usize,
    expand: &str,
) -> Vec<StructuralEdge> {
    crate::map::BoundedProjection::ordered(group, edges, limit, expand).into_shown()
}

fn split_directory_relations(
    edges: Vec<StructuralEdge>,
) -> (Vec<StructuralEdge>, Vec<StructuralEdge>) {
    edges.into_iter().partition(|edge| {
        !matches!(
            edge.edge_type.as_str(),
            "incoming_import" | "package_incoming"
        )
    })
}

fn directory_file_summary(project: &Project, rel: &str) -> FileSummary {
    FileSummary {
        path: rel.to_string(),
        kind: "directory".to_string(),
        package: package_name_for_file(project, rel),
        language: "mixed".to_string(),
        lines: 0,
        roles: Vec::new(),
        symbols: Vec::new(),
        exports: Vec::new(),
        imports: Vec::new(),
        imported_by: CountFact::unknown(
            "directory anchor aggregates files; use file anchors for consumer counts",
        ),
    }
}

pub(crate) fn directory_seed_file_paths(
    project: &Project,
    rel: &str,
    include_hidden: bool,
) -> Vec<String> {
    let mut files = files_under_directory(project, rel)
        .into_iter()
        .filter(|file| !file.has_role("generated") && (include_hidden || !is_generic_noise(file)))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn directory_proof_edges_at_depth(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let seeds = directory_seed_file_paths(project, rel, include_hidden)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let route_seeds = seeds
        .iter()
        .filter(|seed| next_app_route_pattern(seed).is_some())
        .cloned()
        .collect::<Vec<_>>();
    let scope_is_support = is_support_artifact_path(rel);
    let mut edges = Vec::new();
    for test in project.files.values() {
        if !test.has_role("test") || (!include_hidden && test.has_role("test_support")) {
            continue;
        }
        if !include_hidden && !scope_is_support && is_support_artifact_path(&test.rel) {
            continue;
        }
        for target in &test.resolved_imports {
            if seeds.contains(target) {
                edges.push(import_edge(
                    project,
                    test.rel.clone(),
                    target.clone(),
                    "tests",
                    "test_import",
                    EvidenceStrength::High,
                ));
            }
        }
        if !test.has_role("e2e_test") {
            continue;
        }
        for seed in &route_seeds {
            if e2e_test_visits_unique_route(project, seed, test) {
                edges.push(edge_with_path_location(
                    test.rel.clone(),
                    seed.clone(),
                    "tests",
                    "e2e_route",
                    EvidenceStrength::High,
                    test.rel.clone(),
                    "route_visit",
                ));
            }
        }
    }
    dedupe_proof_edges_by_endpoint(aggregate_edges_at_directory_depth(
        project,
        edges,
        rel,
        endpoint_depth,
    ))
}

pub(crate) fn directory_contract_edges_at_depth(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let edges = directory_seed_file_paths(project, rel, include_hidden)
        .into_iter()
        .filter_map(|source| project.files.get(&source))
        .flat_map(|file| {
            file.resolved_imports.iter().filter_map(move |target| {
                let target_file = project.files.get(target)?;
                let evidence = contract_evidence(target_file)?;
                Some(import_edge(
                    project,
                    file.rel.clone(),
                    target.clone(),
                    "contract",
                    evidence,
                    EvidenceStrength::High,
                ))
            })
        })
        .collect::<Vec<_>>();
    aggregate_edges_at_directory_depth(project, edges, rel, endpoint_depth)
}

fn directory_boundary_edges_at_depth(
    project: &Project,
    rel: &str,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let prefix = (rel != ".").then(|| format!("{}/", rel.trim_end_matches('/')));
    let edges = boundary_findings(project, None)
        .into_iter()
        .filter(|finding| {
            prefix
                .as_ref()
                .map(|prefix| finding.from.starts_with(prefix) || finding.to.starts_with(prefix))
                .unwrap_or(true)
        })
        .map(|finding| {
            edge_with_path_location(
                finding.from.clone(),
                finding.to,
                "boundary",
                finding.provenance,
                if finding.strength == "hard" {
                    EvidenceStrength::Hard
                } else {
                    EvidenceStrength::High
                },
                finding.from,
                "boundary_rule_match",
            )
        })
        .collect::<Vec<_>>();
    aggregate_edges_at_directory_depth(project, edges, rel, endpoint_depth)
}

fn aggregate_edges_at_directory_depth(
    project: &Project,
    edges: Vec<StructuralEdge>,
    rel: &str,
    endpoint_depth: usize,
) -> Vec<StructuralEdge> {
    let mut grouped: BTreeMap<(String, String, String, String, EvidenceStrength), usize> =
        BTreeMap::new();
    for edge in edges {
        add_directory_edge(
            &mut grouped,
            directory_edge_endpoint_at_depth(project, rel, &edge.from, endpoint_depth),
            directory_edge_endpoint_at_depth(project, rel, &edge.to, endpoint_depth),
            &edge.edge_type,
            &edge.evidence,
            edge.strength,
        );
    }
    grouped
        .into_iter()
        .map(|((from, to, edge_type, evidence, strength), count)| {
            edge_with_aggregate_location(
                from,
                to,
                edge_type,
                if count > 1 {
                    format!("{evidence}:{count}")
                } else {
                    evidence
                },
                strength,
                "directory_edge_aggregate",
            )
        })
        .collect()
}
