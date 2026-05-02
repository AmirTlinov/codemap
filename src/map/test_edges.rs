fn strict_test_edges_for_file(
    project: &Project,
    rel: &str,
    limit: usize,
) -> Vec<(String, String, EvidenceStrength)> {
    if limit == 0 {
        return Vec::new();
    }
    let source_domain = scoped_domain_path_for_rel(project, rel, domain_by_rel(project, rel));
    let anchor_terms = anchor_terms(project, rel);
    let anchor_core_terms = anchor_core_terms(project, rel);
    let lower_stem = source_stem(rel);
    let allow_name_match = meaningful_stem(&lower_stem);
    let anchor_is_fixture_scope = project
        .files
        .get(rel)
        .map(|file| file.has_role("fixture") || file.has_role("example"))
        .unwrap_or(false);
    let mut scored = Vec::new();
    for file in project.files.values() {
        if !file.has_role("test") || file.has_role("test_support") || !repo::is_source_ext(&file.ext)
        {
            continue;
        }
        if !anchor_is_fixture_scope && (file.has_role("fixture") || file.has_role("example")) {
            continue;
        }
        let test_domain =
            scoped_domain_path_for_rel(project, &file.rel, domain_by_rel(project, rel));
        let route_visit_owner_is_unique = e2e_test_visits_unique_route(project, rel, file);
        if source_domain.is_some() && source_domain != test_domain && !route_visit_owner_is_unique {
            continue;
        }
        if !swift_test_can_prove_anchor(project, rel, file) {
            continue;
        }
        if file.resolved_imports.contains(rel) {
            scored.push((
                80usize,
                file.rel.clone(),
                "test_import".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if route_visit_owner_is_unique {
            scored.push((
                79usize,
                file.rel.clone(),
                "e2e_route".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if test_imports_support_consuming_anchor(project, rel, file) {
            scored.push((
                76usize,
                file.rel.clone(),
                "test_support_import".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if test_references_anchor_symbol(project, rel, file) {
            scored.push((
                74usize,
                file.rel.clone(),
                "test_symbol_reference".to_string(),
                EvidenceStrength::High,
            ));
            continue;
        }
        if allow_name_match && test_name_matches_source_stem(&file.rel, &lower_stem) {
            scored.push((
                70usize,
                file.rel.clone(),
                "test_name".to_string(),
                EvidenceStrength::Medium,
            ));
            continue;
        }
        if let Some((score, evidence, strength)) =
            structural_test_surface_match(project, rel, &anchor_terms, &anchor_core_terms, file)
        {
            scored.push((score, file.rel.clone(), evidence, strength));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    if scored
        .iter()
        .any(|(_, _, _, strength)| *strength == EvidenceStrength::High)
    {
        scored.retain(|(_, _, evidence, strength)| {
            *strength == EvidenceStrength::High || evidence == "e2e_surface_phrase"
        });
    }
    scored
        .into_iter()
        .map(|(_, rel, evidence, strength)| (rel, evidence, strength))
        .take(limit)
        .collect()
}

fn structural_test_surface_match(
    project: &Project,
    rel: &str,
    anchor_terms: &BTreeSet<String>,
    anchor_core_terms: &BTreeSet<String>,
    test: &FileInfo,
) -> Option<(usize, String, EvidenceStrength)> {
    if anchor_terms.is_empty() {
        return None;
    }
    let phrase_shared = shared_surface_phrases(project, rel, test);
    let test_terms = test_surface_terms(test);
    let shared = anchor_terms
        .intersection(&test_terms)
        .cloned()
        .collect::<BTreeSet<_>>();
    let shared_count = shared.len();
    let core_shared_count = anchor_core_terms.intersection(&test_terms).count();
    let source_package = package_for_rel(project, rel).map(|package| package.path.as_str());
    let test_package = package_for_rel(project, &test.rel).map(|package| package.path.as_str());
    let same_package = source_package.is_none() || test_package.is_none() || source_package == test_package;
    let same_parent_signal = same_parent_or_test_scope(rel, &test.rel);
    let test_path_terms = semantic_path_terms(&test.rel);
    let core_path_shared_count = anchor_core_terms.intersection(&test_path_terms).count();
    if test.has_role("e2e_test") {
        if !phrase_shared.is_empty() {
            return Some((
                78 + phrase_shared.len().min(8) * 4,
                "e2e_surface_phrase".to_string(),
                EvidenceStrength::Medium,
            ));
        }
        if e2e_path_surface_allowed(project, rel)
            && e2e_path_surface_match(shared_count, core_shared_count, core_path_shared_count)
        {
            return Some((
                58 + shared_count.min(8) + core_shared_count.min(4) * 8,
                "e2e_path_surface".to_string(),
                EvidenceStrength::Medium,
            ));
        }
        return None;
    }
    if !same_package {
        return None;
    }
    if !phrase_shared.is_empty() {
        return Some((
            72 + phrase_shared.len().min(8) * 4,
            "test_surface_phrase".to_string(),
            EvidenceStrength::Medium,
        ));
    }
    if shared_count < 2 {
        return None;
    }
    if core_shared_count == 0 {
        return None;
    }
    if same_parent_signal
        || (core_path_shared_count >= 2 && (core_shared_count >= 2 || shared_count >= 3))
    {
        return Some((
            50 + shared_count.min(8) + core_shared_count.min(4) * 10,
            "test_surface_tokens".to_string(),
            EvidenceStrength::Medium,
        ));
    }
    if source_package.is_some() && core_path_shared_count >= 2 && core_shared_count >= 2 {
        return Some((
            42 + shared_count.min(8) + core_shared_count.min(4) * 10,
            "test_surface_tokens".to_string(),
            EvidenceStrength::Medium,
        ));
    }
    None
}

fn e2e_path_surface_allowed(project: &Project, rel: &str) -> bool {
    project
        .files
        .get(rel)
        .map(|file| {
            !matches!(file.ext.as_str(), "tsx" | "jsx" | "vue" | "svelte")
                && file.surface_phrases.is_empty()
                && file.jsx_tags.is_empty()
                && file.resolved_imports.is_empty()
                && direct_consumer_edges(project, rel).is_empty()
                && !file.has_role("test")
                && !file.has_role("test_support")
        })
        .unwrap_or(false)
}

fn e2e_path_surface_match(
    shared_count: usize,
    core_shared_count: usize,
    core_path_shared_count: usize,
) -> bool {
    core_path_shared_count >= 3 && core_shared_count >= 2 && shared_count >= 2
}

fn e2e_test_visits_unique_route(project: &Project, rel: &str, test: &FileInfo) -> bool {
    if !test.has_role("e2e_test") {
        return false;
    }
    if !route_proof_scope_matches(project, rel, &test.rel) {
        return false;
    }
    let Some(route) = next_app_route_pattern(rel) else {
        return false;
    };
    test.visited_route_paths.iter().any(|visited| {
        route_pattern_matches(&route, visited)
            && next_route_visit_owner_count(project, rel, visited) == 1
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RoutePatternSegment {
    Static(String),
    Dynamic,
    CatchAll { optional: bool },
}

fn next_app_route_pattern(rel: &str) -> Option<Vec<RoutePatternSegment>> {
    let rest = next_app_route_rest(rel)?;
    let route_dir = [
        "page.tsx", "page.ts", "page.jsx", "page.js", "route.ts", "route.js",
    ]
        .iter()
        .find_map(|suffix| {
            if rest == *suffix {
                Some("")
            } else {
                rest.strip_suffix(&format!("/{suffix}"))
            }
        })?;
    let mut segments = Vec::new();
    let route_segments = route_dir
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    for (index, segment) in route_segments.iter().enumerate() {
        if segment.starts_with('(') && segment.ends_with(')') {
            continue;
        }
        if segment.starts_with('@') {
            return None;
        }
        if let Some(dynamic) = next_dynamic_route_segment(segment) {
            if matches!(dynamic, RoutePatternSegment::CatchAll { .. })
                && index + 1 != route_segments.len()
            {
                return None;
            }
            segments.push(dynamic);
            continue;
        }
        if segment.contains('[') || segment.contains(']') {
            return None;
        }
        segments.push(RoutePatternSegment::Static((*segment).to_string()));
    }
    Some(segments)
}

fn next_app_route_rest(rel: &str) -> Option<&str> {
    rel.strip_prefix("app/")
        .or_else(|| rel.rsplit_once("/app/").map(|(_, rest)| rest))
}

fn next_pages_route_rest(rel: &str) -> Option<&str> {
    rel.strip_prefix("pages/")
        .or_else(|| rel.rsplit_once("/pages/").map(|(_, rest)| rest))
}

fn next_route_visit_owner_count(project: &Project, owner_rel: &str, visited_route: &str) -> usize {
    project
        .files
        .values()
        .filter(|file| {
            route_proof_scope_matches(project, owner_rel, &file.rel)
                && next_app_route_pattern(&file.rel)
                    .as_ref()
                    .is_some_and(|pattern| route_pattern_matches(pattern, visited_route))
        })
        .take(2)
        .count()
}

fn next_dynamic_route_segment(segment: &str) -> Option<RoutePatternSegment> {
    if let Some(name) = segment
        .strip_prefix("[[...")
        .and_then(|value| value.strip_suffix("]]"))
    {
        if valid_dynamic_segment_name(name) {
            return Some(RoutePatternSegment::CatchAll { optional: true });
        }
        return None;
    }
    if let Some(name) = segment
        .strip_prefix("[...")
        .and_then(|value| value.strip_suffix(']'))
    {
        if valid_dynamic_segment_name(name) {
            return Some(RoutePatternSegment::CatchAll { optional: false });
        }
        return None;
    }
    if let Some(name) = segment
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        if valid_dynamic_segment_name(name) {
            return Some(RoutePatternSegment::Dynamic);
        }
        return None;
    }
    None
}

fn valid_dynamic_segment_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn route_pattern_matches(pattern: &[RoutePatternSegment], visited_route: &str) -> bool {
    let visited = visited_route
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if pattern.is_empty() {
        return visited.is_empty();
    }
    let mut visited_index = 0usize;
    for (pattern_index, segment) in pattern.iter().enumerate() {
        match segment {
            RoutePatternSegment::Static(expected) => {
                if visited.get(visited_index).copied() != Some(expected.as_str()) {
                    return false;
                }
                visited_index += 1;
            }
            RoutePatternSegment::Dynamic => {
                let Some(actual) = visited.get(visited_index) else {
                    return false;
                };
                if actual.is_empty() {
                    return false;
                }
                visited_index += 1;
            }
            RoutePatternSegment::CatchAll { optional } => {
                if pattern_index + 1 != pattern.len() {
                    return false;
                }
                return *optional || visited_index < visited.len();
            }
        }
    }
    visited_index == visited.len()
}
