pub fn siblings_report(
    project: &Project,
    scope: &str,
    include_hidden: bool,
    limit: usize,
) -> SiblingsReport {
    let limit = limit.max(1);
    let scope = repo::normalize_rel_path(scope);
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files_under_directory(project, &scope) {
        grouped
            .entry(file_kind_for_ls(file))
            .or_default()
            .push(file.rel.clone());
    }
    let mut same_kind = grouped
        .into_iter()
        .map(|(kind, mut examples)| {
            examples.sort();
            let count = examples.len();
            let hidden_count = count.saturating_sub(5);
            examples.truncate(5);
            Surface {
                id: format!("surface:siblings:{scope}:{kind}"),
                kind,
                path: None,
                role: Some("sibling_group".to_string()),
                evidence: "same_directory_and_kind".to_string(),
                strength: EvidenceStrength::Medium,
                count: Some(count),
                examples,
                hidden_count,
            }
        })
        .collect::<Vec<_>>();
    same_kind.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.kind.cmp(&b.kind)));
    let mut hidden = Vec::new();
    truncate_with_hidden(
        &mut same_kind,
        limit,
        &mut hidden,
        "sibling groups hidden by limit",
        "codemap siblings <scope> --include-hidden",
    );
    let shared_helpers = directory_edges(project, &scope, include_hidden)
        .into_iter()
        .filter(|edge| edge.edge_type.contains("import"))
        .take(limit)
        .collect::<Vec<_>>();
    let shared_contracts = directory_contract_edges_at_depth(project, &scope, include_hidden, 1)
        .into_iter()
        .take(limit)
        .collect::<Vec<_>>();
    let proof_pattern = if include_hidden {
        proof_surfaces_for_directory(project, &scope, 1, limit)
    } else {
        proof_surfaces_for_directory(project, &scope, 1, limit)
            .into_iter()
            .take(limit)
            .collect()
    };
    SiblingsReport {
        kind: "siblings_report",
        schema_version: "1",
        scope: scope.clone(),
        same_kind,
        route_service_test_triplets: Vec::new(),
        shared_helpers,
        shared_contracts,
        proof_pattern,
        unknowns: Vec::new(),
        hidden,
        expand: vec![format!("codemap ls {} --include-hidden", shell_quote(&scope))],
    }
}

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
    let mut examples = files_under_directory(project, &scope)
        .into_iter()
        .filter(|file| file_matches_place_kind(file, &requested_kind))
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    examples.sort();
    let count = examples.len();
    let hidden_count = count.saturating_sub(limit);
    if !include_hidden {
        examples.truncate(limit);
    }
    let existing_surfaces = if examples.is_empty() {
        Vec::new()
    } else {
        vec![Surface {
            id: format!("surface:place:{scope}:{requested_kind}"),
            kind: requested_kind.clone(),
            path: None,
            role: Some("placement_convention".to_string()),
            evidence: "same_scope_kind_filter".to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(count),
            examples,
            hidden_count,
        }]
    };
    let local_conventions = placement_conventions(&scope, &requested_kind, &existing_surfaces);
    let paired_proof_pattern = proof_surfaces_for_directory(project, &scope, 1, limit);
    let shared_contracts = directory_contract_edges_at_depth(project, &scope, include_hidden, 1)
        .into_iter()
        .take(limit)
        .collect();
    PlaceReport {
        kind: "place_report",
        schema_version: "1",
        scope: scope.clone(),
        requested_kind,
        existing_surfaces,
        local_conventions,
        paired_proof_pattern,
        shared_contracts,
        unknowns: Vec::new(),
        hidden: Vec::new(),
        expand: vec![format!("codemap siblings {}", shell_quote(&scope))],
    }
}
