// Responsibility: flow-lens-route-helpers
use crate::map::{
    RuntimeFactIndex, domain_by_rel, next_app_route_pattern, package_for_rel,
    route_pattern_matches, scoped_domain_path_for_rel, static_route_methods,
    structural_edge_with_locations,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, RuntimeRoute, StructuralEdge};
use crate::repo;
use std::path::Path;

pub(crate) fn normalize_flow_anchor(project: &Project, anchor_path: &str) -> String {
    let anchor = anchor_path.trim();
    let path = Path::new(anchor);
    if path.is_absolute()
        && let Ok(rel) = path.strip_prefix(&project.root)
    {
        return repo::normalize_rel_path(&rel.to_string_lossy());
    }
    if route_like_anchor(anchor) {
        anchor.to_string()
    } else {
        repo::normalize_rel_path(anchor)
    }
}

pub(crate) fn route_like_anchor(anchor: &str) -> bool {
    anchor.starts_with('/') || parse_route_anchor(anchor).is_some()
}

pub(crate) enum RouteAnchorLookup {
    None,
    One(RuntimeRoute),
    Ambiguous,
}

pub(crate) fn route_anchor_lookup_with_index(
    anchor: &str,
    index: &RuntimeFactIndex,
) -> RouteAnchorLookup {
    let (method, path) = parse_route_anchor(anchor).unwrap_or((None, anchor));
    let mut routes = index
        .routes
        .iter()
        .filter(|route| {
            route_matches_path(route, path)
                && method.is_none_or(|method| {
                    route
                        .method
                        .as_deref()
                        .is_none_or(|route_method| route_method == "ANY" || route_method == method)
                })
        })
        .take(2)
        .cloned()
        .collect::<Vec<_>>();
    match routes.len() {
        0 => RouteAnchorLookup::None,
        1 => RouteAnchorLookup::One(routes.remove(0)),
        _ => RouteAnchorLookup::Ambiguous,
    }
}

fn parse_route_anchor(anchor: &str) -> Option<(Option<&str>, &str)> {
    let trimmed = anchor.trim();
    if trimmed.starts_with('/') {
        return Some((None, trimmed));
    }
    let (method, path) = trimmed.split_once(' ')?;
    let method = method.trim().to_ascii_uppercase();
    let path = path.trim();
    if !path.starts_with('/')
        || !static_route_methods()
            .iter()
            .any(|known| known.eq_ignore_ascii_case(&method))
    {
        return None;
    }
    Some((
        Some(match method.as_str() {
            "GET" => "GET",
            "POST" => "POST",
            "PUT" => "PUT",
            "PATCH" => "PATCH",
            "DELETE" => "DELETE",
            "ALL" => "ALL",
            "HEAD" => "HEAD",
            "OPTIONS" => "OPTIONS",
            _ => return None,
        }),
        path,
    ))
}

pub(crate) fn route_anchor_label(route: &RuntimeRoute) -> String {
    route
        .method
        .as_ref()
        .map(|method| format!("{method} {}", route.path))
        .unwrap_or_else(|| route.path.clone())
}

pub(crate) fn route_reference_edges_with_index(
    project: &Project,
    route: &RuntimeRoute,
    index: &RuntimeFactIndex,
) -> Vec<StructuralEdge> {
    if !route_can_be_proved_by_page_goto(route) {
        return Vec::new();
    }
    index
        .route_visits
        .iter()
        .filter(|visit| route_matches_path(route, &visit.path))
        .filter(|visit| route_proof_scope_matches(project, &route.file, &visit.file))
        .filter(|visit| {
            route_page_visit_owner_count_for_visit(project, route, &visit.path, index) == 1
        })
        .map(|visit| {
            structural_edge_with_locations(
                visit.file.clone(),
                route.file.clone(),
                "runtime_reference",
                "e2e_visited_route",
                EvidenceStrength::High,
                visit.locations.clone(),
            )
        })
        .collect()
}

pub(crate) fn route_can_be_proved_by_page_goto(route: &RuntimeRoute) -> bool {
    matches!(
        route.method.as_deref(),
        None | Some("GET") | Some("ANY") | Some("ALL")
    )
}

pub(crate) fn route_has_ambiguous_page_visit_owner_with_index(
    project: &Project,
    route: &RuntimeRoute,
    index: &RuntimeFactIndex,
) -> bool {
    if !route_can_be_proved_by_page_goto(route) {
        return false;
    }
    index
        .route_visits
        .iter()
        .filter(|visit| route_matches_path(route, &visit.path))
        .filter(|visit| route_proof_scope_matches(project, &route.file, &visit.file))
        .any(|visit| route_page_visit_owner_count_for_visit(project, route, &visit.path, index) > 1)
}

pub(crate) fn route_has_page_visit_in_proof_scope_with_index(
    project: &Project,
    route: &RuntimeRoute,
    index: &RuntimeFactIndex,
) -> bool {
    index.route_visits.iter().any(|visit| {
        route_matches_path(route, &visit.path)
            && route_proof_scope_matches(project, &route.file, &visit.file)
    })
}

fn route_page_visit_owner_count_for_visit(
    project: &Project,
    route: &RuntimeRoute,
    visited_path: &str,
    index: &RuntimeFactIndex,
) -> usize {
    if !route_can_be_proved_by_page_goto(route) {
        return 0;
    }
    index
        .routes
        .iter()
        .filter(|candidate| {
            route_can_be_proved_by_page_goto(candidate)
                && route_matches_path(candidate, visited_path)
                && route_proof_scope_matches(project, &route.file, &candidate.file)
        })
        .take(2)
        .count()
}

fn route_matches_path(route: &RuntimeRoute, path: &str) -> bool {
    route.path == path
        || next_app_route_pattern(&route.file)
            .as_ref()
            .is_some_and(|pattern| route_pattern_matches(pattern, path))
}

pub(crate) fn route_proof_scope_matches(
    project: &Project,
    owner_rel: &str,
    other_rel: &str,
) -> bool {
    match (
        package_for_rel(project, owner_rel),
        package_for_rel(project, other_rel),
    ) {
        (Some(owner), Some(other)) => owner.manifest == other.manifest,
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => {
            scoped_domain_path_for_rel(project, other_rel, domain_by_rel(project, owner_rel))
                == scoped_domain_path_for_rel(project, owner_rel, domain_by_rel(project, owner_rel))
        }
    }
}

pub(crate) fn route_visit_locations(
    project: &Project,
    rel: &str,
    path: &str,
) -> Vec<EvidenceLocation> {
    let Ok(text) = std::fs::read_to_string(project.root.join(rel)) else {
        return vec![EvidenceLocation::path(rel, "route_visit")];
    };
    for (index, line) in text.lines().enumerate() {
        if line.contains("page.goto") && line.contains(path) {
            return vec![EvidenceLocation::line(rel, index + 1, "route_visit")];
        }
    }
    for (index, line) in text.lines().enumerate() {
        if line.contains(path) {
            return vec![EvidenceLocation::line(rel, index + 1, "route_visit")];
        }
    }
    vec![EvidenceLocation::path(rel, "route_visit")]
}
