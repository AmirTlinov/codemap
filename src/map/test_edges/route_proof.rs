// Responsibility: map-test-edges-route-proof
use crate::map::route_proof_scope_matches;
use crate::model::{EvidenceLocation, FileInfo, Project};

pub(crate) struct RouteProofRunner {
    pub(crate) file: String,
    pub(crate) locations: Vec<EvidenceLocation>,
}

pub(crate) fn e2e_test_visits_unique_route(project: &Project, rel: &str, test: &FileInfo) -> bool {
    if !browser_route_proof_consumer(test) {
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

pub(crate) fn browser_route_proof_consumer(file: &FileInfo) -> bool {
    file.has_role("e2e_test")
        || (file.has_role("proof_runner")
            && !file.visited_route_paths.is_empty()
            && file
                .imports
                .iter()
                .any(|import| matches!(import.as_str(), "playwright" | "@playwright/test")))
}

pub(crate) fn route_proof_runner_consumers(
    project: &Project,
    browser_consumer: &str,
) -> Vec<RouteProofRunner> {
    let mut runners = Vec::new();
    for file in project.files.values() {
        if file.rel == browser_consumer || !file.has_role("script") {
            continue;
        }
        let Some(text) = project.read_indexed_text(&file.rel) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if static_process_line_invokes(line, browser_consumer) {
                runners.push(RouteProofRunner {
                    file: file.rel.clone(),
                    locations: vec![EvidenceLocation::line(
                        &file.rel,
                        index + 1,
                        "process_invocation",
                    )],
                });
                break;
            }
        }
    }
    runners.sort_by(|left, right| left.file.cmp(&right.file));
    runners
}

fn static_process_line_invokes(line: &str, target: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') || !line.contains(target) {
        return false;
    }
    let prefix = line
        .split_once(target)
        .map(|(prefix, _)| prefix)
        .unwrap_or("");
    prefix
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-')))
        .any(|token| {
            matches!(
                token,
                "node" | "python" | "python3" | "bash" | "sh" | "ruby"
            )
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RoutePatternSegment {
    Static(String),
    Dynamic,
    CatchAll { optional: bool },
}

pub(crate) fn next_app_route_pattern(rel: &str) -> Option<Vec<RoutePatternSegment>> {
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

pub(crate) fn next_app_route_rest(rel: &str) -> Option<&str> {
    rel.strip_prefix("app/")
        .or_else(|| rel.rsplit_once("/app/").map(|(_, rest)| rest))
}

pub(crate) fn next_pages_route_rest(rel: &str) -> Option<&str> {
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

pub(crate) fn route_pattern_matches(pattern: &[RoutePatternSegment], visited_route: &str) -> bool {
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
