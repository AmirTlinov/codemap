fn cone_symbol_report(
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
    let mut proof = symbol_proof_edges_with_owning_file(project, file_rel, symbol_name, usize::MAX);
    let mut outgoing = symbol_outgoing_edges(project, info, symbol_name);
    let contracts = symbol_contract_edges(project, file_rel, symbol_name);
    let boundary = Vec::new();
    let mut hidden = Vec::new();
    sort_edges(&mut outgoing);
    sort_edges(&mut incoming);
    sort_edges(&mut proof);
    limit_edge_section(
        &mut outgoing,
        &mut hidden,
        include_hidden,
        limit,
        "symbol outgoing edges hidden by limit",
        &format!(
            "codemap cone {} --all",
            shell_quote(&anchor_path)
        ),
    );
    limit_edge_section(
        &mut incoming,
        &mut hidden,
        include_hidden,
        limit,
        "symbol incoming edges hidden by limit",
        &format!(
            "codemap cone {} --all",
            shell_quote(&anchor_path)
        ),
    );
    limit_edge_section(
        &mut proof,
        &mut hidden,
        include_hidden,
        limit,
        "symbol proof edges hidden by limit",
        &format!(
            "codemap cone {} --all",
            shell_quote(&anchor_path)
        ),
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
    });
    Some(ConeReport {
        kind: "cone_report",
        schema_version: "7",
        anchor,
        depth,
        xray,
        declared_env,
        outgoing,
        incoming,
        proof,
        contracts,
        boundary,
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

fn cone_missing_symbol_report(
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
        imported_by_count: 0,
    };
    let xray = empty_xray_card(&anchor, &unknowns);
    ConeReport {
        kind: "cone_report",
        schema_version: "7",
        anchor,
        depth,
        xray,
        declared_env: Vec::new(),
        outgoing: Vec::new(),
        incoming: Vec::new(),
        proof: Vec::new(),
        contracts: Vec::new(),
        boundary: Vec::new(),
        hidden: Vec::new(),
        unknowns,
        expand: vec![
            format!("codemap ls {}", shell_quote(&info.rel)),
            format!("codemap cone {}", shell_quote(&info.rel)),
        ],
    }
}

fn cone_anchor(
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
                imported_by_count: 0,
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
            imported_by_count: 0,
        },
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

fn cone_depths(project: &Project, seeds: &[String], max_depth: usize) -> BTreeMap<String, usize> {
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

fn cone_outgoing_edges(
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

fn cone_incoming_edges(project: &Project, seeds: &[String]) -> Vec<StructuralEdge> {
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
