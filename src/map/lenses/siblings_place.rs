pub fn siblings_report(
    project: &Project,
    scope: &str,
    include_hidden: bool,
    limit: usize,
) -> SiblingsReport {
    let limit = limit.max(1);
    let scope = repo::normalize_rel_path(scope);
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let (scope_files, recursive_hidden_count) =
        siblings_scope_files(project, &scope, include_hidden);
    for file in &scope_files {
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
    let include_hidden_expand = format!("codemap siblings {} --include-hidden", shell_quote(&scope));
    if recursive_hidden_count > 0 {
        hidden.push(HiddenGroup {
            reason: "recursive sibling files hidden at root scope".to_string(),
            count: recursive_hidden_count,
            expand: include_hidden_expand.clone(),
        });
    }
    truncate_with_hidden(
        &mut same_kind,
        limit,
        &mut hidden,
        "sibling groups hidden by limit",
        &include_hidden_expand,
    );
    let mut shared_helpers = directory_edges(project, &scope, include_hidden)
        .into_iter()
        .filter(|edge| edge.edge_type.contains("import"))
        .collect::<Vec<_>>();
    truncate_with_hidden(
        &mut shared_helpers,
        limit,
        &mut hidden,
        "shared helper edges hidden by limit",
        &include_hidden_expand,
    );
    let mut shared_contracts =
        directory_contract_edges_at_depth(project, &scope, include_hidden, 1);
    truncate_with_hidden(
        &mut shared_contracts,
        limit,
        &mut hidden,
        "shared contract edges hidden by limit",
        &include_hidden_expand,
    );
    let proof_map_raw_expand = format!(
        "codemap proof-map {} --raw-sensors --limit <larger-number>",
        shell_quote(&scope)
    );
    let proof_seed_files = if scope == "." && !include_hidden {
        Vec::new()
    } else {
        directory_seed_file_paths(project, &scope, false)
    };
    let mut proof_pattern = proof_surfaces_for_file_paths(project, &proof_seed_files, 1, limit);
    group_duplicate_proof_surfaces(
        &mut proof_pattern,
        &mut hidden,
        "duplicate proof pattern sensors grouped by structural key",
        &proof_map_raw_expand,
    );
    if !include_hidden {
        truncate_with_hidden(
            &mut proof_pattern,
            limit,
            &mut hidden,
            "proof pattern surfaces hidden by limit",
            &format!(
                "codemap proof-map {} --limit <larger-number>",
                shell_quote(&scope)
            ),
        );
    }
    let runtime_facts = runtime_fact_index(project);
    let mut route_service_test_triplets =
        route_service_test_triplets(&scope, &runtime_facts, &scope_files);
    truncate_with_hidden(
        &mut route_service_test_triplets,
        limit,
        &mut hidden,
        "route/service/test triplets hidden by limit",
        &include_hidden_expand,
    );
    SiblingsReport {
        kind: "siblings_report",
        schema_version: "2",
        scope: scope.clone(),
        same_kind,
        route_service_test_triplets,
        shared_helpers,
        shared_contracts,
        proof_pattern,
        unknowns: Vec::new(),
        hidden,
        expand: vec![format!("codemap ls {} --include-hidden", shell_quote(&scope))],
    }
}

fn siblings_scope_files<'a>(
    project: &'a Project,
    scope: &str,
    include_hidden: bool,
) -> (Vec<&'a FileInfo>, usize) {
    if scope == "." && !include_hidden {
        let direct = direct_files_under_directory(project, scope);
        let recursive_count = files_under_directory(project, scope).len();
        let hidden_count = recursive_count.saturating_sub(direct.len());
        return (direct, hidden_count);
    }
    (files_under_directory(project, scope), 0)
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
    let runtime_facts = (requested_kind == "route").then(|| runtime_fact_index(project));
    let mut matched_files = files_under_directory(project, &scope)
        .into_iter()
        .filter(|file| {
            file_matches_place_kind(file, &requested_kind)
                || runtime_facts
                    .as_ref()
                    .is_some_and(|facts| facts.has_routes_for_file(&file.rel))
        })
        .map(|file| file.rel.clone())
        .collect::<Vec<_>>();
    matched_files.sort();
    let count = matched_files.len();
    let hidden_count = count.saturating_sub(limit);
    let mut visible_examples = matched_files.clone();
    if !include_hidden {
        visible_examples.truncate(limit);
    }
    let existing_surfaces = if matched_files.is_empty() {
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
            examples: visible_examples.clone(),
            hidden_count,
        }]
    };
    let local_conventions = placement_conventions(&scope, &requested_kind, &existing_surfaces);
    let mut hidden = Vec::new();
    let proof_map_raw_expand = format!(
        "codemap proof-map {} --raw-sensors --limit <larger-number>",
        shell_quote(&scope)
    );
    let proof_seed_files = if include_hidden {
        matched_files.as_slice()
    } else {
        visible_examples.as_slice()
    };
    let mut paired_proof_pattern =
        proof_surfaces_for_file_paths(project, proof_seed_files, 1, limit);
    group_duplicate_proof_surfaces(
        &mut paired_proof_pattern,
        &mut hidden,
        "duplicate paired proof sensors grouped by structural key",
        &proof_map_raw_expand,
    );
    if !include_hidden {
        truncate_with_hidden(
            &mut paired_proof_pattern,
            limit,
            &mut hidden,
            "paired proof pattern surfaces hidden by limit",
            &format!(
                "codemap proof-map {} --limit <larger-number>",
                shell_quote(&scope)
            ),
        );
    }
    let include_hidden_expand = format!("codemap place {} --include-hidden", shell_quote(&scope));
    let mut shared_contracts =
        directory_contract_edges_at_depth(project, &scope, include_hidden, 1);
    truncate_with_hidden(
        &mut shared_contracts,
        limit,
        &mut hidden,
        "shared contract edges hidden by limit",
        &include_hidden_expand,
    );
    PlaceReport {
        kind: "place_report",
        schema_version: "2",
        scope: scope.clone(),
        requested_kind,
        existing_surfaces,
        local_conventions,
        paired_proof_pattern,
        shared_contracts,
        unknowns: Vec::new(),
        hidden,
        expand: vec![format!("codemap siblings {}", shell_quote(&scope))],
    }
}

#[derive(Default)]
struct TripletParts {
    routes: Vec<String>,
    services: Vec<String>,
    tests: Vec<String>,
}

fn route_service_test_triplets(
    scope: &str,
    runtime_facts: &RuntimeFactIndex,
    files: &[&FileInfo],
) -> Vec<Surface> {
    let mut groups: BTreeMap<String, TripletParts> = BTreeMap::new();
    for file in files {
        let key = feature_stem(&file.rel);
        let group = groups.entry(key).or_default();
        if route_from_path(&file.rel) || runtime_facts.has_routes_for_file(&file.rel) {
            group.routes.push(file.rel.clone());
        }
        if file_matches_place_kind(file, "service") {
            group.services.push(file.rel.clone());
        }
        if file.has_role("test") {
            group.tests.push(file.rel.clone());
        }
    }
    let mut out = Vec::new();
    for (key, mut group) in groups {
        group.routes.sort();
        group.services.sort();
        group.tests.sort();
        let mut examples = Vec::new();
        examples.extend(group.routes.iter().take(2).cloned());
        examples.extend(group.services.iter().take(2).cloned());
        examples.extend(group.tests.iter().take(2).cloned());
        examples.sort();
        examples.dedup();
        let role_count = [!group.routes.is_empty(), !group.services.is_empty(), !group.tests.is_empty()]
            .into_iter()
            .filter(|present| *present)
            .count();
        if role_count < 2 {
            continue;
        }
        let total_count = group.routes.len() + group.services.len() + group.tests.len();
        let hidden_count = total_count.saturating_sub(examples.len());
        out.push(Surface {
            id: format!("surface:triplet:{scope}:{key}"),
            kind: "route_service_test_triplet".to_string(),
            path: None,
            role: Some("local_convention".to_string()),
            evidence: "same_scope_stem_and_role".to_string(),
            strength: EvidenceStrength::Medium,
            count: Some(total_count),
            examples,
            hidden_count,
        });
    }
    out.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.cmp(&b.id)));
    out
}

fn feature_stem(rel: &str) -> String {
    let file_name = Path::new(rel)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(rel);
    let without_ext = file_name
        .trim_end_matches(".tsx")
        .trim_end_matches(".jsx")
        .trim_end_matches(".ts")
        .trim_end_matches(".js")
        .trim_end_matches(".py")
        .trim_end_matches(".go")
        .trim_end_matches(".rs");
    without_ext
        .replace(".test", "")
        .replace(".spec", "")
        .replace("-test", "")
        .replace("_test", "")
        .replace("-route", "")
        .replace("_route", "")
        .replace("-routes", "")
        .replace("_routes", "")
        .replace("-service", "")
        .replace("_service", "")
        .replace("-controller", "")
        .replace("_controller", "")
}
