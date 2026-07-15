// Responsibility: map-cone-traversal
use crate::map::{
    ConeXrayInput, SymbolConeObservationInput, cone_xray_card, directory_has_files,
    empty_xray_card, file_summary, files_under_directory, import_edge, is_generic_noise,
    limit_edge_section, missing_symbol_observations, package_name_for_file,
    push_symbol_hidden_groups, same_package_symbol_reference_consumers, shell_quote, sort_edges,
    structural_roles_for_ls, symbol_anchor_path, symbol_cone_observations, symbol_contract_edges,
    symbol_file_summary, symbol_local_incoming_edges, symbol_outgoing_edges,
    symbol_reference_edges, symbol_verification_edges_with_owning_file, unique, unknown,
    unknown_missing_symbol_anchor, unknown_symbol_outgoing, unresolved_import_unknowns,
};
use crate::model::CountFact;
use crate::model::{
    ConeReport, EvidenceStrength, FileInfo, FileSummary, HiddenGroup, Project, StructuralEdge,
    Unknown,
};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;

pub(crate) fn cone_symbol_report(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    depth: usize,
    include_hidden: bool,
    limit: usize,
) -> Option<ConeReport> {
    let info = project.files.get(file_rel)?;
    let anchor = symbol_file_summary(project, info, symbol_name)?;
    let anchor_path = symbol_anchor_path(file_rel, symbol_name);
    let mut incoming = symbol_reference_edges(project, file_rel, symbol_name, false);
    incoming.extend(symbol_local_incoming_edges(project, info, symbol_name));
    let mut proof =
        symbol_verification_edges_with_owning_file(project, file_rel, symbol_name, usize::MAX);
    let mut outgoing = symbol_outgoing_edges(project, info, symbol_name);
    let contracts = symbol_contract_edges(project, file_rel, symbol_name);
    let boundary = Vec::new();
    let mut hidden = Vec::new();
    sort_edges(&mut outgoing);
    sort_edges(&mut incoming);
    sort_edges(&mut proof);
    let incoming_observed = incoming.len();
    let proof_observed = proof.len();
    limit_edge_section(
        &mut outgoing,
        &mut hidden,
        include_hidden,
        limit.min(5),
        "symbol outgoing edges hidden by limit",
        &format!("codemap cone {} --all", shell_quote(&anchor_path)),
    );
    let incoming_limit = if include_hidden {
        incoming_observed
    } else {
        limit.min(5)
    };
    let proof_limit = if include_hidden {
        proof_observed
    } else {
        limit.min(3)
    };
    incoming.truncate(incoming_limit);
    proof.truncate(proof_limit);
    let expand_all = format!("codemap cone {} --all", shell_quote(&anchor_path));
    let observations = symbol_cone_observations(
        project,
        SymbolConeObservationInput {
            file_rel,
            symbol_name,
            incoming_observed,
            incoming_shown: incoming.len(),
            incoming_expand: (incoming.len() < incoming_observed).then(|| expand_all.clone()),
            verification_observed: proof_observed,
            verification_shown: proof.len(),
            verification_expand: (proof.len() < proof_observed).then_some(expand_all),
        },
    );
    let mut unknowns = Vec::new();
    if outgoing.is_empty() {
        unknowns.push(unknown_symbol_outgoing(&anchor_path));
    }
    let seed_files = vec![file_rel.to_string()];
    let declared_env = Vec::new();
    let xray = cone_xray_card(ConeXrayInput {
        project,
        anchor: &anchor,
        seed_files: &seed_files,
        declared_env: &declared_env,
        outgoing: &outgoing,
        incoming: &incoming,
        proof: &proof,
        unknowns: &unknowns,
        limit,
        include_hidden,
    });
    Some(ConeReport {
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
        hidden,
        unknowns,
        expand: vec![
            format!(
                "codemap cone {} --depth {}",
                shell_quote(file_rel),
                depth + 1
            ),
            format!("codemap ls {}", shell_quote(file_rel)),
        ],
    })
}

pub(crate) fn cone_missing_symbol_report(
    project: &Project,
    info: &FileInfo,
    symbol_name: &str,
    depth: usize,
) -> ConeReport {
    let anchor_path = symbol_anchor_path(&info.rel, symbol_name);
    let unknowns = vec![unknown_missing_symbol_anchor(&info.rel, symbol_name)];
    let anchor = FileSummary {
        path: anchor_path.clone(),
        kind: "missing_symbol".to_string(),
        package: package_name_for_file(project, &info.rel),
        language: info.language.clone(),
        lines: 0,
        roles: structural_roles_for_ls(info),
        symbols: Vec::new(),
        exports: Vec::new(),
        imports: Vec::new(),
        imported_by: CountFact::unknown("anchor summary is aggregated at this level"),
    };
    let xray = empty_xray_card(&anchor, &unknowns);
    let observations = missing_symbol_observations(project, &anchor_path);
    ConeReport {
        kind: "cone_report",
        schema_version: crate::model::ConeReport::SCHEMA_VERSION,
        anchor,
        depth,
        xray,
        declared_env: Vec::new(),
        outgoing: Vec::new(),
        incoming: Vec::new(),
        proof: Vec::new(),
        contracts: Vec::new(),
        boundary: Vec::new(),
        observations,
        hidden: Vec::new(),
        unknowns,
        expand: vec![
            format!("codemap ls {}", shell_quote(&info.rel)),
            format!("codemap cone {}", shell_quote(&info.rel)),
        ],
    }
}

pub(crate) fn cone_anchor(
    project: &Project,
    rel: &str,
    include_hidden: bool,
    limit: usize,
) -> (FileSummary, Vec<String>, Vec<Unknown>, Vec<HiddenGroup>) {
    if let Some(info) = project.files.get(rel) {
        let summary = file_summary(project, info, include_hidden, limit);
        let mut hidden = Vec::new();
        let unknowns = unresolved_import_unknowns(project, info);
        push_symbol_hidden_groups(
            &mut hidden,
            info,
            include_hidden,
            limit,
            &format!("codemap cone {} --all", shell_quote(rel)),
        );
        return (summary, vec![info.rel.clone()], unknowns, hidden);
    }
    if directory_has_files(project, rel) {
        let mut files = files_under_directory(project, rel)
            .into_iter()
            .filter(|file| {
                !file.has_role("generated") && (include_hidden || !is_generic_noise(file))
            })
            .map(|file| file.rel.clone())
            .collect::<Vec<_>>();
        files.sort();
        let count = files.len();
        if !include_hidden {
            files.truncate(limit);
        }
        let mut hidden = Vec::new();
        if count > files.len() {
            hidden.push(HiddenGroup {
                reason: "directory anchor files hidden by limit".to_string(),
                count: count - files.len(),
                expand: format!("codemap cone {} --all", shell_quote(rel)),
            });
        }
        return (
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
            },
            files,
            vec![unknown(
                "directory_anchor_summary",
                Some(rel),
                None,
                "directory anchor summarizes indexed files under this path",
                "use file anchors for exact line-level edges",
                Some(format!("codemap ls {}", shell_quote(rel))),
            )],
            hidden,
        );
    }
    (
        FileSummary {
            path: rel.to_string(),
            kind: "missing".to_string(),
            package: package_name_for_file(project, rel),
            language: "unknown".to_string(),
            lines: 0,
            roles: Vec::new(),
            symbols: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            imported_by: CountFact::unknown("anchor is not indexed"),
        },
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

pub(crate) fn cone_depths(
    project: &Project,
    seeds: &[String],
    max_depth: usize,
) -> BTreeMap<String, usize> {
    let mut depths = BTreeMap::new();
    let mut queue = VecDeque::new();
    for seed in seeds {
        if project.files.contains_key(seed) && depths.insert(seed.clone(), 0).is_none() {
            queue.push_back(seed.clone());
        }
    }
    while let Some(rel) = queue.pop_front() {
        let depth = depths.get(&rel).copied().unwrap_or(0);
        if depth >= max_depth {
            continue;
        }
        for neighbor in structural_neighbors(project, &rel) {
            if depths.contains_key(&neighbor) {
                continue;
            }
            depths.insert(neighbor.clone(), depth + 1);
            queue.push_back(neighbor);
        }
    }
    depths
}

fn structural_neighbors(project: &Project, rel: &str) -> Vec<String> {
    let mut neighbors = Vec::new();
    if let Some(file) = project.files.get(rel) {
        neighbors.extend(file.resolved_imports.iter().cloned());
    }
    if let Some(importers) = project.reverse_imports.get(rel) {
        neighbors.extend(importers.iter().cloned());
    }
    neighbors.extend(
        same_package_symbol_reference_consumers(project, rel)
            .into_iter()
            .map(|edge| edge.from),
    );
    unique(neighbors)
        .into_iter()
        .filter(|neighbor| project.files.contains_key(neighbor))
        .collect()
}

pub(crate) fn cone_outgoing_edges(
    project: &Project,
    depths: &BTreeMap<String, usize>,
    max_depth: usize,
) -> Vec<StructuralEdge> {
    let mut edges = Vec::new();
    for (rel, depth) in depths {
        if *depth >= max_depth {
            continue;
        }
        let Some(file) = project.files.get(rel) else {
            continue;
        };
        for target in &file.resolved_imports {
            if project.files.contains_key(target) {
                let (evidence, strength) = if *depth == 0 {
                    ("resolved_import", EvidenceStrength::High)
                } else {
                    ("resolved_import_via_cone_depth", EvidenceStrength::Medium)
                };
                edges.push(import_edge(
                    project,
                    file.rel.clone(),
                    target.clone(),
                    "imports",
                    evidence,
                    strength,
                ));
            }
        }
    }
    edges
}

pub(crate) fn cone_incoming_edges(project: &Project, seeds: &[String]) -> Vec<StructuralEdge> {
    let seed_set = seeds.iter().cloned().collect::<BTreeSet<_>>();
    let mut edges = Vec::new();
    for seed in seeds {
        if let Some(importers) = project.reverse_imports.get(seed) {
            for importer in importers {
                if project
                    .files
                    .get(importer)
                    .map(|file| file.has_role("test"))
                    .unwrap_or(false)
                {
                    continue;
                }
                if !seed_set.contains(importer) {
                    edges.push(import_edge(
                        project,
                        importer.clone(),
                        seed.clone(),
                        "imported_by",
                        "reverse_import",
                        EvidenceStrength::High,
                    ));
                }
            }
        }
        for edge in same_package_symbol_reference_consumers(project, seed) {
            if !seed_set.contains(&edge.from) {
                edges.push(edge);
            }
        }
    }
    edges
}
