// Responsibility: map-proof-locations
use crate::evidence::import_statement_locations;
use crate::map::{
    anchor_core_terms, anchor_symbol_reference_names, anchor_terms,
    code_shape_without_literal_content, meaningful_surface_phrase, meaningful_surface_term,
    next_app_route_pattern, route_pattern_matches, route_visit_locations, runtime_code_lines,
    semantic_name_terms, shared_surface_phrases, source_stem,
};
use crate::model::{EvidenceLocation, Project};
use crate::proof_classification::proof_base_evidence;
use crate::repo;
use std::collections::BTreeSet;

pub(crate) fn proof_surface_locations_for_test(
    test: &str,
    evidence: &str,
) -> Vec<EvidenceLocation> {
    vec![EvidenceLocation::path(test, evidence)]
}

pub(crate) fn proof_surface_locations_for_target(
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
    if soft_surface_location_evidence(base) {
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

fn soft_surface_location_evidence(evidence: &str) -> bool {
    matches!(
        evidence,
        "test_surface_phrase"
            | "test_surface_tokens"
            | "test_role_surface_match"
            | "e2e_surface_phrase"
            | "e2e_path_surface"
    )
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
    let Some(text) = project.read_indexed_text(test) else {
        return proof_surface_locations_for_test(test, "test_symbol_reference");
    };
    let mut locations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let code = code_shape_without_literal_content(line);
        if names
            .iter()
            .any(|name| identifier_reference_on_line(&code, name))
        {
            locations.push(EvidenceLocation::line(test, index + 1, "symbol_reference"));
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
    let Some(text) = project.read_indexed_text(test) else {
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
    let Some(text) = project.read_indexed_text(test) else {
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
