pub fn ls_report(project: &Project, path: &str, include_hidden: bool, limit: usize) -> LsReport {
    let rel = repo::normalize_rel_path(path);
    if let Some((file_rel, symbol_name)) = split_symbol_anchor(&rel)
        && let Some(info) = project.files.get(&file_rel)
    {
        return ls_symbol_report(project, info, &symbol_name, include_hidden, limit.max(1));
    }
    if let Some(info) = project.files.get(&rel) {
        return ls_file_report(project, info, include_hidden, limit.max(1));
    }
    if directory_has_files(project, &rel) {
        return ls_directory_report(project, &rel, include_hidden, limit.max(1));
    }
    LsReport {
        kind: "ls_report",
        schema_version: "3",
        path: rel.clone(),
        mode: "missing".to_string(),
        anchor: None,
        directory: Vec::new(),
        edges: Vec::new(),
        hidden: Vec::new(),
        next: vec![format!(
            "codemap ls {}",
            shell_quote(&parent_anchor_for_missing(&rel))
        )],
    }
}

pub fn cone_report(
    project: &Project,
    path: &str,
    depth: usize,
    include_hidden: bool,
    limit: usize,
) -> ConeReport {
    let rel = repo::normalize_rel_path(path);
    let depth = depth.min(4);
    let limit = limit.max(1);
    if let Some((file_rel, symbol_name)) = split_symbol_anchor(&rel)
        && let Some(info) = project.files.get(&file_rel)
    {
        if let Some(report) = cone_symbol_report(
            project,
            &file_rel,
            &symbol_name,
            depth,
            include_hidden,
            limit,
        ) {
            return report;
        }
        return cone_missing_symbol_report(project, info, &symbol_name, depth);
    }
    if directory_has_files(project, &rel) {
        return cone_directory_report(project, &rel, depth, include_hidden, limit);
    }
    let (anchor, seed_files, mut unknowns, mut hidden) =
        cone_anchor(project, &rel, include_hidden, limit);
    let depths = cone_depths(project, &seed_files, depth);
    let mut outgoing = cone_outgoing_edges(project, &depths, depth);
    let mut incoming = cone_incoming_edges(project, &seed_files);
    let mut proof = cone_proof_edges_with_direct_consumers(project, &seed_files);
    let mut contracts = cone_contract_edges(project, &outgoing);
    let mut boundary = cone_boundary_edges(project, &rel, &depths);
    if seed_files.len() == 1 {
        let seed = &seed_files[0];
        outgoing.extend(cone_owner_outgoing_edges(project, seed));
        incoming.extend(cone_owner_incoming_edges(project, seed));
        proof.extend(cone_owner_proof_edges(project, seed));
        unknowns.extend(cone_owner_unknowns(project, seed));
    }
    sort_edges(&mut outgoing);
    sort_edges(&mut incoming);
    sort_edges(&mut proof);
    sort_edges(&mut contracts);
    sort_edges(&mut boundary);
    limit_edge_section(
        &mut outgoing,
        &mut hidden,
        include_hidden,
        limit,
        "outgoing edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --all",
            shell_quote(&rel)
        ),
    );
    limit_edge_section(
        &mut incoming,
        &mut hidden,
        include_hidden,
        limit,
        "incoming edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --all",
            shell_quote(&rel)
        ),
    );
    limit_edge_section(
        &mut proof,
        &mut hidden,
        include_hidden,
        limit,
        "proof edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --all",
            shell_quote(&rel)
        ),
    );
    limit_edge_section(
        &mut contracts,
        &mut hidden,
        include_hidden,
        limit,
        "contract edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --all",
            shell_quote(&rel)
        ),
    );
    limit_edge_section(
        &mut boundary,
        &mut hidden,
        include_hidden,
        limit,
        "boundary edges hidden by limit",
        &format!(
            "codemap cone {} --depth {depth} --all",
            shell_quote(&rel)
        ),
    );
    let declared_env = cone_declared_env(project, &rel);
    if seed_files.is_empty() {
        unknowns.push(unknown_unindexed_anchor(&rel));
    }
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
    ConeReport {
        kind: "cone_report",
        schema_version: "6",
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
            format!("codemap cone {} --depth {}", shell_quote(&rel), depth + 1),
            format!("codemap ls {} --all", shell_quote(&rel)),
        ],
    }
}
