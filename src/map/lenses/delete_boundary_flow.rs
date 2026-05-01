pub fn delete_report(
    project: &Project,
    anchor_path: &str,
    include_hidden: bool,
    limit: usize,
) -> DeleteReport {
    let limit = limit.max(1);
    let rel = repo::normalize_rel_path(anchor_path);
    let (file_rel, symbol) = split_symbol_anchor(&rel).unwrap_or_else(|| (rel.clone(), String::new()));
    if !project.files.contains_key(&file_rel) {
        return DeleteReport {
            kind: "delete_report",
            schema_version: "1",
            anchor: missing_file_summary(project, &rel),
            direct_users: Vec::new(),
            symbol_users: Vec::new(),
            reexports: Vec::new(),
            package_exports: Vec::new(),
            tests: Vec::new(),
            runtime_refs: Vec::new(),
            unknowns: vec![unknown_unindexed_anchor(&file_rel)],
            checklist: Vec::new(),
            hidden: Vec::new(),
            expand: vec![format!("codemap ls {}", shell_quote(&file_rel))],
        };
    }
    if !symbol.is_empty()
        && project
            .files
            .get(&file_rel)
            .and_then(|file| symbol_file_summary(project, file, &symbol))
            .is_none()
    {
        return DeleteReport {
            kind: "delete_report",
            schema_version: "1",
            anchor: missing_file_summary(project, &rel),
            direct_users: Vec::new(),
            symbol_users: Vec::new(),
            reexports: Vec::new(),
            package_exports: Vec::new(),
            tests: Vec::new(),
            runtime_refs: Vec::new(),
            unknowns: vec![unknown_missing_symbol_anchor(&file_rel, &symbol)],
            checklist: Vec::new(),
            hidden: Vec::new(),
            expand: vec![format!("codemap ls {}", shell_quote(&file_rel))],
        };
    }
    let anchor = project
        .files
        .get(&file_rel)
        .map(|file| {
            if symbol.is_empty() {
                file_summary(project, file, include_hidden, 20)
            } else {
                symbol_file_summary(project, file, &symbol)
                    .unwrap_or_else(|| file_summary(project, file, include_hidden, 20))
            }
        })
        .unwrap_or_else(|| missing_file_summary(project, &file_rel));
    let mut direct_users = direct_consumer_edges(project, &file_rel);
    let mut symbol_users = if symbol.is_empty() {
        Vec::new()
    } else {
        symbol_reference_edges(project, &file_rel, &symbol, true)
    };
    let mut reexports = symbol_users
        .iter()
        .filter(|edge| edge.evidence.contains("reexport"))
        .cloned()
        .collect::<Vec<_>>();
    let mut package_exports = package_export_edges(project, &file_rel);
    let mut tests = if symbol.is_empty() {
        cone_proof_edges(project, std::slice::from_ref(&file_rel))
    } else {
        symbol_proof_edges(project, &file_rel, &symbol)
    };
    let mut runtime_refs = runtime_reference_edges(project, &file_rel);
    let mut hidden = Vec::new();
    for (edges, reason) in [
        (&mut direct_users, "direct users hidden by limit"),
        (&mut symbol_users, "symbol users hidden by limit"),
        (&mut reexports, "reexports hidden by limit"),
        (&mut package_exports, "package exports hidden by limit"),
        (&mut tests, "tests hidden by limit"),
        (&mut runtime_refs, "runtime refs hidden by limit"),
    ] {
        limit_edge_section(
            edges,
            &mut hidden,
            include_hidden,
            limit,
            reason,
            &format!("codemap delete {} --include-hidden", shell_quote(&rel)),
        );
    }
    let mut checklist = Vec::new();
    if !reexports.is_empty() {
        checklist.push("remove or update barrel reexports shown above".to_string());
    }
    if !package_exports.is_empty() {
        checklist.push("remove or update package public exports shown above".to_string());
    }
    if !tests.is_empty() {
        checklist.push("update direct proof surfaces shown above".to_string());
    }
    if !runtime_refs.is_empty() {
        checklist.push("inspect runtime references shown above".to_string());
    }
    DeleteReport {
        kind: "delete_report",
        schema_version: "1",
        anchor,
        direct_users,
        symbol_users,
        reexports,
        package_exports,
        tests,
        runtime_refs,
        unknowns: project
            .files
            .get(&file_rel)
            .map(|file| unknowns_for_file(project, file))
            .unwrap_or_default(),
        checklist,
        hidden,
        expand: vec![format!("codemap cone {}", shell_quote(&file_rel))],
    }
}

pub fn boundary_map_report(
    project: &Project,
    scope: &str,
    changed: Option<&BTreeSet<String>>,
    include_hidden: bool,
    limit: usize,
) -> BoundaryMapReport {
    let limit = limit.max(1);
    let scope = repo::normalize_rel_path(scope);
    let mut actual_cross_edges = Vec::new();
    let mut test_only_crossings = Vec::new();
    for file in files_under_directory(project, &scope) {
        for target in &file.resolved_imports {
            if changed.is_some_and(|changed| !changed.contains(&file.rel) && !changed.contains(target)) {
                continue;
            }
            let cross = domain_by_rel(project, &file.rel).map(|domain| domain.path.clone())
                != domain_by_rel(project, target).map(|domain| domain.path.clone())
                || package_for_rel(project, &file.rel).map(|package| package.path.clone())
                    != package_for_rel(project, target).map(|package| package.path.clone());
            if cross {
                let edge = import_edge(
                    project,
                    file.rel.clone(),
                    target.clone(),
                    "cross_boundary_import",
                    "resolved_import_cross_boundary",
                    EvidenceStrength::High,
                );
                if file.has_role("test") || file.has_role("test_support") {
                    test_only_crossings.push(edge);
                } else {
                    actual_cross_edges.push(edge);
                }
            }
        }
    }
    let mut public_boundary_files = files_under_directory(project, &scope)
        .into_iter()
        .filter(|file| file.has_role("public_boundary"))
        .map(|file| file_summary(project, file, false, 12))
        .collect::<Vec<_>>();
    let mut package_edges = project
        .package_edges
        .iter()
        .filter(|edge| path_under_scope(&edge.from_manifest, &scope))
        .cloned()
        .collect::<Vec<_>>();
    let explicit_forbidden_findings = boundary_findings(project, changed).into_iter().collect();
    let mut hidden = Vec::new();
    limit_edge_section(
        &mut actual_cross_edges,
        &mut hidden,
        include_hidden,
        limit,
        "actual cross-boundary edges hidden by limit",
        &format!("codemap boundary-map {} --include-hidden", shell_quote(&scope)),
    );
    truncate_with_hidden(
        &mut public_boundary_files,
        limit,
        &mut hidden,
        "public boundary files hidden by limit",
        "codemap boundary-map <scope> --include-hidden",
    );
    truncate_with_hidden(
        &mut package_edges,
        limit,
        &mut hidden,
        "package edges hidden by limit",
        "codemap boundary-map <scope> --include-hidden",
    );
    BoundaryMapReport {
        kind: "boundary_map_report",
        schema_version: "1",
        scope,
        domains: project.domains.iter().map(DomainRef::from).collect(),
        actual_cross_edges,
        public_boundary_files,
        test_only_crossings,
        package_edges,
        explicit_forbidden_findings,
        unknowns: Vec::new(),
        hidden,
        expand: vec!["codemap boundaries".to_string()],
    }
}

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
    let mut entry = project
        .files
        .get(&rel)
        .map(|file| file_summary(project, file, include_hidden, 20));
    let mut side_effects = Vec::new();
    match route_anchor_lookup(project, &rel) {
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
            for edge in direct_dependency_edges(project, &route_file)
                .into_iter()
                .take(limit)
            {
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
            proof = route_reference_edges(project, &route);
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
                            schema_version: "1",
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
                    for edge in symbol_outgoing_edges(project, info, &symbol)
                        .into_iter()
                        .take(limit)
                    {
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
                steps.push(FlowStep {
                    index: 0,
                    anchor: rel.clone(),
                    kind: "file_anchor".to_string(),
                    evidence: "exact_file_anchor".to_string(),
                    locations: vec![EvidenceLocation::path(&rel, "file_anchor")],
                });
                for edge in direct_dependency_edges(project, &rel).into_iter().take(limit) {
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
    FlowReport {
        kind: "flow_report",
        schema_version: "1",
        anchor: rel.clone(),
        flow_kind: "structural".to_string(),
        precision: "bounded_edges_only".to_string(),
        entry,
        steps,
        side_effects,
        contracts,
        proof,
        unknown_breaks,
        hidden: Vec::new(),
        expand,
    }
}
