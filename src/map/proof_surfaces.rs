fn proof_surfaces_for_anchor(
    project: &Project,
    anchor: &str,
    depth: usize,
    limit: usize,
) -> Vec<ProofSurface> {
    let mut out = owner_surface_proof_surfaces(project, anchor);
    out.extend(role_aware_command_proof_surfaces(project, anchor));
    for (test, evidence, strength) in strict_test_edges_for_file(project, anchor, limit) {
        out.push(ProofSurface {
            command: proof_command_for_test(project, &test),
            locations: proof_surface_locations_for_target(project, anchor, &test, &evidence),
            path: Some(test),
            reason: proof_reason_for_evidence(&evidence, "anchor"),
            evidence,
            strength,
        });
    }
    if out.is_empty() {
        for edge in proof_edges_via_direct_dependencies(project, anchor, limit) {
            out.push(ProofSurface {
                command: proof_command_for_test(project, &edge.from),
                path: Some(edge.from),
                reason: proof_reason_for_evidence(&edge.evidence, "anchor"),
                locations: edge.locations,
                evidence: edge.evidence,
                strength: edge.strength,
            });
        }
    }
    if depth <= 1 && !out.is_empty() {
        return out;
    }
    let mut consumers = direct_consumer_edges(project, anchor);
    sort_edges(&mut consumers);
    for consumer in consumers.into_iter().take(limit) {
        for (test, evidence, strength) in strict_test_edges_for_file(project, &consumer.from, limit)
        {
            out.push(ProofSurface {
                command: proof_command_for_test(project, &test),
                locations: proof_surface_locations_for_target(project, &consumer.from, &test, &evidence),
                path: Some(test),
                reason: proof_reason_for_evidence(&evidence, "direct consumer"),
                evidence,
                strength,
            });
        }
    }
    if depth > 1 {
        for consumer in direct_consumer_edges(project, anchor)
            .into_iter()
            .take(limit)
        {
            for second in direct_consumer_edges(project, &consumer.from)
                .into_iter()
                .take(limit)
            {
                for (test, evidence, strength) in
                    strict_test_edges_for_file(project, &second.from, limit)
                {
                    out.push(ProofSurface {
                        command: proof_command_for_test(project, &test),
                        locations: proof_surface_locations_for_target(project, &second.from, &test, &evidence),
                        path: Some(test),
                        reason: proof_reason_for_evidence(&evidence, "depth-2 consumer"),
                        evidence,
                        strength,
                    });
                }
            }
        }
    }
    out
}

fn proof_surfaces_for_directory(
    project: &Project,
    rel: &str,
    depth: usize,
    limit: usize,
) -> Vec<ProofSurface> {
    let files = directory_seed_file_paths(project, rel, false);
    proof_surfaces_for_file_paths(project, &files, depth, limit)
}

fn proof_surfaces_for_file_paths(
    project: &Project,
    files: &[String],
    depth: usize,
    limit: usize,
) -> Vec<ProofSurface> {
    let mut out = Vec::new();
    for file in files {
        out.extend(proof_surfaces_for_anchor(project, file, depth, limit));
    }
    out
}

fn proof_surfaces_for_symbol_anchor(
    project: &Project,
    file_rel: &str,
    symbol_name: &str,
    depth: usize,
    limit: usize,
) -> Vec<ProofSurface> {
    let mut out = symbol_proof_edges_with_owning_file(project, file_rel, symbol_name, limit)
        .into_iter()
        .take(limit)
        .map(|edge| ProofSurface {
            command: proof_command_for_test(project, &edge.from),
            path: Some(edge.from),
            reason: proof_reason_for_evidence(&edge.evidence, "symbol anchor"),
            locations: edge.locations,
            evidence: edge.evidence,
            strength: edge.strength,
        })
        .collect::<Vec<_>>();
    if !out.is_empty() {
        return out;
    }
    if depth <= 1 {
        return out;
    }
    for consumer in symbol_reference_edges(project, file_rel, symbol_name, false)
        .into_iter()
        .take(limit)
    {
        let consumer_file = consumer.from;
        for (test, evidence, strength) in strict_test_edges_for_file(project, &consumer_file, limit)
        {
            out.push(ProofSurface {
                command: proof_command_for_test(project, &test),
                locations: proof_surface_locations_for_target(project, &consumer_file, &test, &evidence),
                path: Some(test),
                reason: proof_reason_for_evidence(&evidence, "symbol consumer"),
                evidence,
                strength,
            });
        }
    }
    out
}

fn proof_surface_locations_for_test(
    test: &str,
    evidence: &str,
) -> Vec<EvidenceLocation> {
    vec![EvidenceLocation::path(test, evidence)]
}

fn proof_surface_locations_for_target(
    project: &Project,
    target: &str,
    test: &str,
    evidence: &str,
) -> Vec<EvidenceLocation> {
    let base = proof_base_evidence(evidence);
    if matches!(base, "test_import" | "test_imported_symbol_reference") {
        let locations = import_statement_locations(project, test, target);
        if locations
            .iter()
            .any(|location| location.line_start.is_some())
        {
            return locations;
        }
        return proof_surface_locations_for_test(test, evidence);
    }
    if base == "test_symbol_reference" {
        return symbol_reference_locations_for_test(project, target, test);
    }
    if base == "e2e_route" {
        return e2e_route_locations_for_test(project, target, test);
    }
    if matches!(
        base,
        "test_surface_phrase" | "test_surface_tokens" | "e2e_surface_phrase" | "e2e_path_surface"
    ) {
        return test_surface_match_locations_for_target(project, target, test, base);
    }
    if base == "test_name" {
        return test_name_match_locations_for_target(project, target, test);
    }
    proof_surface_locations_for_test(test, evidence)
}

fn e2e_route_locations_for_test(
    project: &Project,
    target: &str,
    test: &str,
) -> Vec<EvidenceLocation> {
    let Some(route_pattern) = next_app_route_pattern(target) else {
        return proof_surface_locations_for_test(test, "e2e_route");
    };
    let Some(test_file) = project.files.get(test) else {
        return proof_surface_locations_for_test(test, "e2e_route");
    };
    let mut locations = Vec::new();
    let mut seen = BTreeSet::new();
    for visited in &test_file.visited_route_paths {
        if !route_pattern_matches(&route_pattern, visited) {
            continue;
        }
        for location in route_visit_locations(project, test, visited) {
            let key = (
                location.path.clone(),
                location.line_start.unwrap_or_default(),
                location.kind.clone(),
            );
            if seen.insert(key) {
                locations.push(location);
            }
        }
    }
    if locations.is_empty() {
        proof_surface_locations_for_test(test, "e2e_route")
    } else {
        locations
    }
}

fn symbol_reference_locations_for_test(
    project: &Project,
    target: &str,
    test: &str,
) -> Vec<EvidenceLocation> {
    let Some(target_file) = project.files.get(target) else {
        return proof_surface_locations_for_test(test, "test_symbol_reference");
    };
    let names = anchor_symbol_reference_names(target_file);
    if names.is_empty() {
        return proof_surface_locations_for_test(test, "test_symbol_reference");
    }
    let Ok(text) = std::fs::read_to_string(project.root.join(test)) else {
        return proof_surface_locations_for_test(test, "test_symbol_reference");
    };
    let mut locations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = code_shape_without_literal_content(line);
        if names
            .iter()
            .any(|name| identifier_reference_on_line(&code, name))
        {
            locations.push(EvidenceLocation::line(
                test,
                index + 1,
                "symbol_reference",
            ));
            if locations.len() >= 3 {
                break;
            }
        }
    }
    if locations.is_empty() {
        proof_surface_locations_for_test(test, "test_symbol_reference")
    } else {
        locations
    }
}

fn test_surface_match_locations_for_target(
    project: &Project,
    target: &str,
    test: &str,
    evidence: &str,
) -> Vec<EvidenceLocation> {
    let Some(test_file) = project.files.get(test) else {
        return proof_surface_locations_for_test(test, evidence);
    };
    let Ok(text) = std::fs::read_to_string(project.root.join(test)) else {
        return proof_surface_locations_for_test(test, evidence);
    };
    let shared_phrases = shared_surface_phrases(project, target, test_file);
    let token_terms = surface_location_terms(project, target);
    let mut locations = Vec::new();
    for (line_number, line) in runtime_code_lines(&text) {
        if line_matches_shared_phrase(&line, &shared_phrases)
            || line_matches_surface_terms(&line, &token_terms)
        {
            locations.push(EvidenceLocation::line(test, line_number, "test_surface"));
            if locations.len() >= 3 {
                break;
            }
        }
    }
    if locations.is_empty() {
        proof_surface_locations_for_test(test, evidence)
    } else {
        locations
    }
}

fn test_name_match_locations_for_target(
    project: &Project,
    target: &str,
    test: &str,
) -> Vec<EvidenceLocation> {
    let Ok(text) = std::fs::read_to_string(project.root.join(test)) else {
        return proof_surface_locations_for_test(test, "test_name");
    };
    let terms = semantic_name_terms(&source_stem(target));
    if terms.is_empty() {
        return proof_surface_locations_for_test(test, "test_name");
    }
    let locations = runtime_code_lines(&text)
        .into_iter()
        .filter_map(|(line_number, line)| {
            (line_is_test_declaration(&line) && line_matches_surface_terms(&line, &terms))
                .then(|| EvidenceLocation::line(test, line_number, "test_name"))
        })
        .take(3)
        .collect::<Vec<_>>();
    if locations.is_empty() {
        proof_surface_locations_for_test(test, "test_name")
    } else {
        locations
    }
}

fn surface_location_terms(project: &Project, target: &str) -> BTreeSet<String> {
    let mut terms = anchor_core_terms(project, target);
    if terms.len() < 2 {
        terms.extend(anchor_terms(project, target));
    }
    terms
}

fn line_matches_shared_phrase(line: &str, shared_phrases: &BTreeSet<String>) -> bool {
    if shared_phrases.is_empty() {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    shared_phrases.iter().any(|phrase| {
        meaningful_surface_phrase(phrase) && lower.contains(&phrase.to_ascii_lowercase())
    })
}

fn line_matches_surface_terms(line: &str, terms: &BTreeSet<String>) -> bool {
    if terms.is_empty() {
        return false;
    }
    let line_terms = repo::tokenize(line)
        .into_iter()
        .filter(|term| meaningful_surface_term(term))
        .collect::<BTreeSet<_>>();
    let shared = terms.intersection(&line_terms).count();
    if terms.len() == 1 {
        shared == 1
    } else {
        shared >= 2
    }
}

fn line_is_test_declaration(line: &str) -> bool {
    let code = code_shape_without_literal_content(line);
    let trimmed = code.trim_start();
    js_test_call_line(trimmed)
        || trimmed.starts_with("def test_")
        || trimmed.starts_with("async def test_")
        || trimmed.starts_with("class Test")
        || trimmed.starts_with("func Test")
        || trimmed.starts_with("func Benchmark")
        || trimmed.starts_with("func Fuzz")
        || trimmed.starts_with("fn test_")
        || trimmed.starts_with("async fn test_")
        || trimmed.contains(" fn test_")
        || trimmed.starts_with("func test")
}

fn js_test_call_line(trimmed: &str) -> bool {
    [
        "test(",
        "it(",
        "describe(",
        "test.only(",
        "test.skip(",
        "test.fixme(",
        "test.each(",
        "test.describe(",
        "test.describe.only(",
        "test.describe.skip(",
        "it.only(",
        "it.skip(",
        "it.each(",
        "describe.only(",
        "describe.skip(",
        "describe.each(",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

fn identifier_reference_on_line(code: &str, name: &str) -> bool {
    code.match_indices(name).any(|(start, _)| {
        let before = code[..start].chars().next_back();
        let end = start + name.len();
        let after = code[end..].chars().next();
        before.is_none_or(|ch| !proof_identifier_char(ch))
            && after.is_none_or(|ch| !proof_identifier_char(ch))
    })
}

fn proof_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

fn proof_base_evidence(evidence: &str) -> &str {
    evidence
        .strip_suffix("_via_direct_consumer")
        .or_else(|| evidence.strip_suffix("_via_direct_dependency"))
        .or_else(|| evidence.strip_suffix("_via_local_symbol_consumer"))
        .or_else(|| evidence.strip_suffix("_owning_file"))
        .unwrap_or(evidence)
}

fn impact_level_for_directory(project: &Project, rel: &str, depth: usize) -> Risk {
    files_under_directory(project, rel)
        .into_iter()
        .filter(|file| !file.has_role("generated") && !is_generic_noise(file))
        .map(|file| structural_impact_level_for_file(project, &file.rel, depth).0)
        .max()
        .unwrap_or(Risk::Medium)
}

fn structural_impact_level_for_file(project: &Project, rel: &str, depth: usize) -> (Risk, Vec<String>) {
    let (file_risk, mut reasons) = impact_level_for_file(project, rel);
    if !project.files.contains_key(rel) {
        return (file_risk, reasons);
    }
    let direct_consumers = direct_consumer_edges(project, rel);
    let cross_boundary_consumers =
        cross_boundary_consumer_edges(project, rel, &direct_consumers, depth.max(1));
    let contract_links = contract_link_edges(project, rel, &direct_consumers);
    let (structural_risk, structural_reasons) = structural_impact_level(
        project,
        rel,
        &direct_consumers,
        &cross_boundary_consumers,
        &contract_links,
    );
    reasons.extend(structural_reasons);
    (file_risk.max(structural_risk), unique(reasons))
}

fn proof_reason_for_evidence(evidence: &str, scope: &str) -> String {
    if let Some(base) = evidence.strip_suffix("_owning_file") {
        return format!(
            "{} on owning file; no exact symbol proof found",
            proof_reason_for_evidence(base, scope)
        );
    }
    if let Some(base) = evidence.strip_suffix("_via_direct_consumer") {
        return format!(
            "{} via direct consumer",
            proof_reason_for_evidence(base, scope)
        );
    }
    if let Some(base) = evidence.strip_suffix("_via_direct_dependency") {
        return format!(
            "{} via direct dependency",
            proof_reason_for_evidence(base, scope)
        );
    }
    if let Some(base) = evidence.strip_suffix("_via_local_symbol_consumer") {
        return format!(
            "{} via same-file symbol consumer",
            proof_reason_for_evidence(base, scope)
        );
    }
    match evidence {
        "test_import" => format!("test imports {scope}"),
        "test_imported_symbol_reference" => {
            format!("test imports and references {scope}")
        }
        "test_reexported_symbol_reference" => {
            format!("test imports and references re-exported {scope}")
        }
        "e2e_route" => format!("e2e visits route for {scope}"),
        "test_name" => format!("test name matches {scope}"),
        "test_support_import" => format!("test imports support code that imports {scope}"),
        "test_symbol_reference" => format!("test references an anchor symbol from {scope}"),
        "test_surface_phrase" => format!("test uses same UI/test surface as {scope}"),
        "e2e_surface_phrase" => format!("e2e uses same UI/test surface as {scope}"),
        "e2e_path_surface" => format!("e2e path/name surface matches {scope}"),
        "test_surface_tokens" => format!("test path/symbols match {scope} surface"),
        _ => format!("structural proof for {scope}"),
    }
}
