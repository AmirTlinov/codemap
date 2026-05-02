pub fn contract_report(
    project: &Project,
    anchor_path: &str,
    include_hidden: bool,
    limit: usize,
) -> ContractReport {
    let limit = limit.max(1);
    let rel = repo::normalize_rel_path(anchor_path);
    let anchor = project
        .files
        .get(&rel)
        .map(|file| file_summary(project, file, include_hidden, 20))
        .unwrap_or_else(|| missing_file_summary(project, &rel));
    let mut producers = direct_dependency_edges(project, &rel);
    let mut consumers = direct_consumer_edges(project, &rel);
    let mut cross_package_consumers = consumers
        .iter()
        .filter(|edge| {
            package_for_rel(project, &edge.from).map(|package| package.path.as_str())
                != package_for_rel(project, &rel).map(|package| package.path.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut proof = cone_proof_edges_with_direct_consumers(project, std::slice::from_ref(&rel));
    let mut package_exports = package_export_edges(project, &rel);
    let mut exported_contracts = project
        .files
        .get(&rel)
        .map(|file| {
            file.exports
                .iter()
                .map(|export| Surface {
                    id: format!("surface:export:{rel}#{export}"),
                    kind: "exported_symbol".to_string(),
                    path: Some(symbol_anchor_path(&rel, export)),
                    role: Some("contract".to_string()),
                    evidence: "exported_symbol".to_string(),
                    strength: EvidenceStrength::High,
                    count: Some(1),
                    examples: vec![symbol_anchor_path(&rel, export)],
                    hidden_count: 0,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let public_surface = project
        .files
        .get(&rel)
        .map(|file| {
            file.has_role("public_boundary")
                || package_for_rel(project, &rel).is_some_and(|package| package.manifest == rel)
                || !package_exports.is_empty()
                || !cross_package_consumers.is_empty()
        })
        .unwrap_or(false);
    let contract_kind = project
        .files
        .get(&rel)
        .and_then(contract_evidence)
        .unwrap_or_else(|| {
            if anchor.kind == "missing" {
                "missing".to_string()
            } else if !anchor.exports.is_empty() {
                "export_surface".to_string()
            } else {
                "internal_surface".to_string()
            }
        });
    let mut hidden = Vec::new();
    let include_hidden_expand = format!("codemap contract {} --include-hidden", shell_quote(&rel));
    truncate_with_hidden(
        &mut exported_contracts,
        limit,
        &mut hidden,
        "exported contract surfaces hidden by limit",
        &include_hidden_expand,
    );
    limit_edge_section(
        &mut package_exports,
        &mut hidden,
        include_hidden,
        limit,
        "package export edges hidden by limit",
        &include_hidden_expand,
    );
    limit_edge_section(
        &mut producers,
        &mut hidden,
        include_hidden,
        limit,
        "contract producer edges hidden by limit",
        &include_hidden_expand,
    );
    limit_edge_section(
        &mut consumers,
        &mut hidden,
        include_hidden,
        limit,
        "contract consumer edges hidden by limit",
        &include_hidden_expand,
    );
    limit_edge_section(
        &mut cross_package_consumers,
        &mut hidden,
        include_hidden,
        limit,
        "cross-package consumer edges hidden by limit",
        &include_hidden_expand,
    );
    limit_edge_section(
        &mut proof,
        &mut hidden,
        include_hidden,
        limit,
        "contract proof edges hidden by limit",
        &include_hidden_expand,
    );
    let mut unknowns = project
        .files
        .get(&rel)
        .map(|file| unknowns_for_file(project, file))
        .unwrap_or_default();
    truncate_with_hidden(
        &mut unknowns,
        limit,
        &mut hidden,
        "contract unknowns hidden by limit",
        &include_hidden_expand,
    );
    ContractReport {
        kind: "contract_report",
        schema_version: "2",
        anchor,
        contract_kind,
        public_surface,
        exported_contracts,
        package_exports,
        producers,
        consumers,
        cross_package_consumers,
        proof,
        unknowns,
        hidden,
        expand: vec![
            format!("codemap cone {}", shell_quote(&rel)),
            format!("codemap impact --files {}", shell_quote(&rel)),
        ],
    }
}
