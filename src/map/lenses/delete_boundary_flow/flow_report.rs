// Responsibility: flow-lens-report
use crate::map::{
    RouteAnchorLookup, cone_contract_edges, cone_proof_edges, direct_dependency_edges,
    file_summary, limit_edge_section, normalize_flow_anchor, route_anchor_label,
    route_anchor_lookup_with_index, route_like_anchor, route_reference_edges_with_index,
    runtime_entrypoint_direct_call_steps, runtime_entrypoint_locations,
    runtime_entrypoint_surface_for_file, runtime_entrypoint_symbol_step, runtime_fact_index,
    shell_quote, side_effect_surfaces_for_file, split_symbol_anchor, symbol_definition_location,
    symbol_file_summary, symbol_outgoing_edges, symbol_proof_edges, truncate_with_hidden, unknown,
    unknown_missing_symbol_anchor, unknown_unindexed_anchor, unknowns_for_file,
};
use crate::model::{EvidenceLocation, FlowReport, FlowStep, Project, RuntimeRoute};

pub fn flow_report(
    project: &Project,
    anchor_path: &str,
    include_hidden: bool,
    limit: usize,
) -> FlowReport {
    let limit = limit.max(1);
    let rel = normalize_flow_anchor(project, anchor_path);
    let mut steps = Vec::new();
    let mut contracts = Vec::new();
    let mut proof = Vec::new();
    let mut unknown_breaks = Vec::new();
    let mut expand = vec![format!("codemap cone {}", shell_quote(&rel))];
    let runtime_facts = runtime_fact_index(project);
    let mut entry = project
        .files
        .get(&rel)
        .map(|file| file_summary(project, file, include_hidden, 20));
    let mut side_effects = Vec::new();
    match route_anchor_lookup_with_index(&rel, &runtime_facts) {
        RouteAnchorLookup::One(route) => {
            let route_file = route.file.clone();
            expand = vec![
                format!("codemap cone {}", shell_quote(&route_file)),
                format!("codemap runtime {}", shell_quote(&route_file)),
            ];
            entry.clone_from(
                &project
                    .files
                    .get(&route_file)
                    .map(|file| file_summary(project, file, include_hidden, 20)),
            );
            steps.push(FlowStep {
                index: 0,
                anchor: route_anchor_label(&route),
                kind: "route_anchor".to_string(),
                evidence: route.evidence.clone(),
                locations: route.locations.clone(),
            });
            steps.push(FlowStep {
                index: 1,
                anchor: route_file.clone(),
                kind: "route_file".to_string(),
                evidence: "runtime_route_owner".to_string(),
                locations: vec![EvidenceLocation::path(&route_file, "route_file")],
            });
            if let Some(handler) = route_handler_step(project, &route) {
                steps.push(handler);
            }
            for edge in direct_dependency_edges(project, &route_file) {
                steps.push(FlowStep {
                    index: steps.len(),
                    anchor: edge.to.clone(),
                    kind: edge.edge_type.clone(),
                    evidence: edge.evidence.clone(),
                    locations: edge.locations.clone(),
                });
            }
            let dependency_edges = direct_dependency_edges(project, &route_file);
            contracts = cone_contract_edges(project, &dependency_edges);
            proof = route_reference_edges_with_index(project, &route, &runtime_facts);
            if let Some(file) = project.files.get(&route_file) {
                side_effects = side_effect_surfaces_for_file(project, file);
                unknown_breaks.extend(unknowns_for_file(project, file));
            }
        }
        RouteAnchorLookup::Ambiguous => {
            unknown_breaks.push(unknown(
                "route_anchor_ambiguous",
                Some(&rel),
                None,
                "multiple static runtime routes matched this anchor",
                "flow stops instead of choosing a route by file order; use a method-specific anchor",
                Some("codemap runtime .".to_string()),
            ));
            expand = vec!["codemap runtime .".to_string()];
        }
        RouteAnchorLookup::None if route_like_anchor(&rel) => {
            unknown_breaks.push(unknown(
                "route_anchor_not_found",
                Some(&rel),
                None,
                "no static runtime route matched this anchor",
                "flow stops instead of guessing framework routing",
                Some("codemap runtime .".to_string()),
            ));
            expand = vec!["codemap runtime .".to_string()];
        }
        RouteAnchorLookup::None => {
            if let Some((file_rel, symbol)) = split_symbol_anchor(&rel) {
                if let Some(info) = project.files.get(&file_rel) {
                    if symbol_file_summary(project, info, &symbol).is_none() {
                        unknown_breaks.push(unknown_missing_symbol_anchor(&file_rel, &symbol));
                        return FlowReport {
                            kind: "flow_report",
                            schema_version: "3",
                            anchor: rel.clone(),
                            flow_kind: "structural".to_string(),
                            precision: "bounded_edges_only".to_string(),
                            entry: None,
                            steps,
                            side_effects: Vec::new(),
                            contracts,
                            proof,
                            unknown_breaks,
                            hidden: Vec::new(),
                            expand: vec![format!("codemap ls {}", shell_quote(&file_rel))],
                        };
                    }
                    steps.push(FlowStep {
                        index: 0,
                        anchor: rel.clone(),
                        kind: "symbol_anchor".to_string(),
                        evidence: "exact_symbol_anchor".to_string(),
                        locations: symbol_definition_location(
                            project,
                            &file_rel,
                            &symbol,
                            "symbol_definition",
                        ),
                    });
                    for edge in symbol_outgoing_edges(project, info, &symbol) {
                        steps.push(FlowStep {
                            index: steps.len(),
                            anchor: edge.to.clone(),
                            kind: edge.edge_type.clone(),
                            evidence: edge.evidence.clone(),
                            locations: edge.locations.clone(),
                        });
                        if edge.edge_type == "contract" {
                            contracts.push(edge);
                        }
                    }
                    proof = symbol_proof_edges(project, &file_rel, &symbol);
                    unknown_breaks.extend(unknowns_for_file(project, info));
                } else {
                    unknown_breaks.push(unknown_unindexed_anchor(&file_rel));
                }
            } else if let Some(file) = project.files.get(&rel) {
                if let Some(surface) = runtime_entrypoint_surface_for_file(project, &rel) {
                    steps.push(FlowStep {
                        index: 0,
                        anchor: surface
                            .examples
                            .first()
                            .cloned()
                            .unwrap_or_else(|| rel.clone()),
                        kind: "runtime_entrypoint".to_string(),
                        evidence: surface.evidence.clone(),
                        locations: runtime_entrypoint_locations(project, &rel, &surface),
                    });
                    if let Some((step, entry_symbol)) =
                        runtime_entrypoint_symbol_step(project, file)
                    {
                        steps.push(FlowStep {
                            index: steps.len(),
                            ..step
                        });
                        for step in
                            runtime_entrypoint_direct_call_steps(project, file, &entry_symbol)
                        {
                            steps.push(FlowStep {
                                index: steps.len(),
                                ..step
                            });
                        }
                    }
                }
                steps.push(FlowStep {
                    index: steps.len(),
                    anchor: rel.clone(),
                    kind: "file_anchor".to_string(),
                    evidence: "exact_file_anchor".to_string(),
                    locations: vec![EvidenceLocation::path(&rel, "file_anchor")],
                });
                for edge in direct_dependency_edges(project, &rel) {
                    steps.push(FlowStep {
                        index: steps.len(),
                        anchor: edge.to.clone(),
                        kind: edge.edge_type.clone(),
                        evidence: edge.evidence.clone(),
                        locations: edge.locations.clone(),
                    });
                }
                contracts = cone_contract_edges(project, &direct_dependency_edges(project, &rel));
                proof = cone_proof_edges(project, std::slice::from_ref(&file.rel));
                unknown_breaks.extend(unknowns_for_file(project, file));
                side_effects = side_effect_surfaces_for_file(project, file);
            } else {
                unknown_breaks.push(unknown_unindexed_anchor(&rel));
            }
        }
    }
    if steps.len() <= 1 && unknown_breaks.is_empty() {
        unknown_breaks.push(unknown(
            "flow_not_extended",
            Some(&rel),
            None,
            "no structural outgoing step was found",
            "flow stops instead of guessing callgraph or dataflow",
            Some(format!("codemap cone {}", shell_quote(&rel))),
        ));
    }
    let mut hidden = Vec::new();
    let include_hidden_expand = format!("codemap flow {} --all", shell_quote(&rel));
    truncate_with_hidden(
        &mut steps,
        limit,
        &mut hidden,
        "flow steps hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut side_effects,
        limit,
        &mut hidden,
        "flow side-effect surfaces hidden by limit",
        &include_hidden_expand,
    );
    limit_edge_section(
        &mut contracts,
        &mut hidden,
        include_hidden,
        limit,
        "flow contract edges hidden by limit",
        &include_hidden_expand,
    );
    limit_edge_section(
        &mut proof,
        &mut hidden,
        include_hidden,
        limit,
        "flow verification edges hidden by limit",
        &include_hidden_expand,
    );
    truncate_with_hidden(
        &mut unknown_breaks,
        limit,
        &mut hidden,
        "flow unknown breaks hidden by limit",
        &include_hidden_expand,
    );
    FlowReport {
        kind: "flow_report",
        schema_version: "3",
        anchor: rel.clone(),
        flow_kind: "structural".to_string(),
        precision: "bounded_edges_only".to_string(),
        entry,
        steps,
        side_effects,
        contracts,
        proof,
        unknown_breaks,
        hidden,
        expand,
    }
}

fn route_handler_step(project: &Project, route: &RuntimeRoute) -> Option<FlowStep> {
    let handler = route.handler_symbol.as_deref()?;
    let file = project.files.get(&route.file)?;
    if !file.symbols.iter().any(|symbol| symbol.name == handler) {
        return None;
    }
    Some(FlowStep {
        index: 2,
        anchor: format!("{}#{handler}", route.file),
        kind: "route_handler".to_string(),
        evidence: "route_handler_symbol".to_string(),
        locations: symbol_definition_location(project, &route.file, handler, "route_handler"),
    })
}
