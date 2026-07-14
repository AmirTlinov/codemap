// Responsibility: runtime-lens-proof-helpers
use crate::map::{
    RuntimeFactIndex, proof_command_for_test, route_anchor_label, route_can_be_proved_by_page_goto,
    route_has_ambiguous_page_visit_owner_with_index,
    route_has_page_visit_in_proof_scope_with_index, route_reference_edges_with_index, shell_quote,
    unknown,
};
use crate::model::{Project, ProofSurface, RuntimeRoute, Unknown};

pub(crate) fn route_proof_surfaces_for_routes(
    project: &Project,
    routes: Vec<RuntimeRoute>,
    index: &RuntimeFactIndex,
) -> Vec<ProofSurface> {
    routes
        .into_iter()
        .flat_map(|route| {
            let label = route_anchor_label(&route);
            route_reference_edges_with_index(project, &route, index)
                .into_iter()
                .map(move |edge| ProofSurface {
                    command: proof_command_for_test(project, &edge.from),
                    path: Some(edge.from),
                    target_anchor: Some(route.file.clone()),
                    evidence: edge.evidence,
                    strength: edge.strength,
                    reason: format!("e2e visits runtime route {label}"),
                    locations: edge.locations,
                })
        })
        .collect()
}

pub(crate) fn route_proof_unknowns_for_routes(
    project: &Project,
    routes: Vec<RuntimeRoute>,
    index: &RuntimeFactIndex,
) -> Vec<Unknown> {
    routes
        .into_iter()
        .filter(|route| {
            route_can_be_proved_by_page_goto(route)
                && route_has_page_visit_in_proof_scope_with_index(project, route, index)
                && route_has_ambiguous_page_visit_owner_with_index(project, route, index)
        })
        .map(|route| {
            let line = route
                .locations
                .first()
                .and_then(|location| location.line_start);
            unknown(
                "ambiguous_route_visit_owner",
                Some(route.file.clone()),
                line,
                format!(
                    "runtime route `{}` has multiple method-compatible owners in this proof scope",
                    route_anchor_label(&route)
                ),
                "page.goto route visits are not attached as e2e proof because the owner is ambiguous",
                Some(format!("codemap runtime {}", shell_quote(&route.file))),
            )
        })
        .collect()
}
