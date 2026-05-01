fn cone_directory_report(
    project: &Project,
    rel: &str,
    depth: usize,
    include_hidden: bool,
    limit: usize,
) -> ConeReport {
    let depth = depth.max(1);
    let anchor = directory_file_summary(project, rel);
    let mut outgoing = Vec::new();
    let mut incoming = Vec::new();
    for edge in directory_edges_at_depth(project, rel, include_hidden, depth) {
        match edge.edge_type.as_str() {
            "incoming_import" | "package_incoming" => incoming.push(edge),
            _ => outgoing.push(edge),
        }
    }
    let mut proof = directory_proof_edges_at_depth(project, rel, include_hidden, depth);
    let mut contracts = directory_contract_edges_at_depth(project, rel, include_hidden, depth);
    let mut boundary = directory_boundary_edges_at_depth(project, rel, depth);
    let mut hidden = Vec::new();
    sort_edges(&mut outgoing);
    sort_edges(&mut incoming);
    sort_edges(&mut proof);
    sort_edges(&mut boundary);
    limit_edge_section(
        &mut outgoing,
        &mut hidden,
        include_hidden,
        limit,
        "directory outgoing edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(rel)
        ),
    );
    limit_edge_section(
        &mut incoming,
        &mut hidden,
        include_hidden,
        limit,
        "directory incoming edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(rel)
        ),
    );
    limit_edge_section(
        &mut proof,
        &mut hidden,
        include_hidden,
        limit,
        "directory proof edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(rel)
        ),
    );
    limit_edge_section(
        &mut contracts,
        &mut hidden,
        include_hidden,
        limit,
        "directory contract edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(rel)
        ),
    );
    limit_edge_section(
        &mut boundary,
        &mut hidden,
        include_hidden,
        limit,
        "directory boundary edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --include-hidden",
            shell_quote(rel)
        ),
    );

    ConeReport {
        kind: "cone_report",
        schema_version: "3",
        anchor,
        depth,
        outgoing,
        incoming,
        proof,
        contracts,
        boundary,
        hidden,
        unknowns: vec![unknown_directory_aggregate(rel, depth)],
        expand: vec![
            format!("codemap cone {} --depth {}", shell_quote(rel), depth + 1),
            format!("codemap ls {} --include-hidden", shell_quote(rel)),
        ],
    }
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
        imported_by_count: 0,
    }
}

fn directory_seed_file_paths(project: &Project, rel: &str, include_hidden: bool) -> Vec<String> {
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
    let seeds = directory_seed_file_paths(project, rel, include_hidden);
    dedupe_proof_edges_by_endpoint(aggregate_edges_at_directory_depth(
        project,
        cone_proof_edges_with_direct_consumers(project, &seeds),
        rel,
        endpoint_depth,
    ))
}

fn directory_contract_edges_at_depth(
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
        .map(
            |((from, to, edge_type, evidence, strength), count)| {
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
            },
        )
        .collect()
}
