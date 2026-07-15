// Responsibility: runtime-route-boundary-path-facts
mod classify;
mod deployment_env;

use self::classify::{
    explicitly_omitted_fields, invoked_target, middleware_or_guard_kind, response_constructors,
    response_projection_name, runtime_path_unknowns, transformation_name,
};
use self::deployment_env::{DeploymentEnvIndex, deployment_env_index};
use crate::map::{
    env_surfaces_for_file, imported_binding_target_symbol_name, route_anchor_label, sort_edges,
    structural_edge_with_locations, symbol_body_text, symbol_outgoing_edges,
};
use crate::model::{
    EvidenceLocation, EvidenceStrength, MiddlewareOrGuard, Project, RuntimeRoute, StructuralEdge,
    Unknown,
};
use std::collections::{BTreeSet, VecDeque};

#[derive(Default)]
pub(crate) struct RuntimePathAnalysis {
    pub(crate) edges: Vec<StructuralEdge>,
    pub(crate) guards: Vec<MiddlewareOrGuard>,
    pub(crate) unknowns: Vec<Unknown>,
}

pub(crate) struct RuntimePathContext {
    deployment_env: DeploymentEnvIndex,
}

pub(crate) fn runtime_path_context(project: &Project) -> RuntimePathContext {
    RuntimePathContext {
        deployment_env: deployment_env_index(project),
    }
}

pub(crate) fn runtime_route_path_analysis(
    project: &Project,
    route: &RuntimeRoute,
    context: &RuntimePathContext,
) -> RuntimePathAnalysis {
    let mut analysis = RuntimePathAnalysis::default();
    let route_anchor = route_anchor_label(route);
    let Some(handler) = route.handler_symbol.as_deref() else {
        return analysis;
    };
    let handler_anchor = format!("{}#{handler}", route.file);
    analysis.edges.push(structural_edge_with_locations(
        route_anchor.clone(),
        handler_anchor.clone(),
        "routes_to",
        "route_handler_symbol",
        EvidenceStrength::High,
        route.locations.clone(),
    ));
    for guard in &route.middleware_or_guards {
        analysis.edges.push(structural_edge_with_locations(
            route_anchor.clone(),
            guard.owner.clone(),
            "guarded_by",
            guard.evidence.clone(),
            guard.strength,
            guard.locations.clone(),
        ));
        analysis.guards.push(guard.clone());
    }

    let mut queue = VecDeque::from([(route.file.clone(), handler.to_string(), 0usize)]);
    let mut visited = BTreeSet::new();
    while let Some((rel, symbol, depth)) = queue.pop_front() {
        if depth > 3 || analysis.edges.len() >= 64 || !visited.insert((rel.clone(), symbol.clone()))
        {
            continue;
        }
        trace_symbol(
            project,
            context,
            &rel,
            &symbol,
            depth,
            &mut queue,
            &mut analysis,
        );
    }
    sort_edges(&mut analysis.edges);
    analysis.edges.dedup_by(|a, b| {
        a.from == b.from && a.to == b.to && a.edge_type == b.edge_type && a.evidence == b.evidence
    });
    analysis.guards.sort_by(|a, b| {
        a.owner
            .cmp(&b.owner)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    analysis
        .guards
        .dedup_by(|a, b| a.owner == b.owner && a.kind == b.kind);
    analysis.unknowns.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.line_start.cmp(&b.line_start))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    analysis
        .unknowns
        .dedup_by(|a, b| a.kind == b.kind && a.path == b.path && a.line_start == b.line_start);
    analysis
}

fn trace_symbol(
    project: &Project,
    context: &RuntimePathContext,
    rel: &str,
    symbol: &str,
    depth: usize,
    queue: &mut VecDeque<(String, String, usize)>,
    analysis: &mut RuntimePathAnalysis,
) {
    let Some(file) = project.files.get(rel) else {
        return;
    };
    let Some(info) = file
        .symbols
        .iter()
        .find(|candidate| candidate.name == symbol)
    else {
        return;
    };
    let Some(body) = symbol_body_text(project, file, symbol) else {
        return;
    };
    let source = format!("{rel}#{symbol}");
    let line_offset = info.line_start.saturating_sub(1);
    analysis
        .unknowns
        .extend(runtime_path_unknowns(rel, &body, line_offset));

    let outgoing = symbol_outgoing_edges(project, file, symbol)
        .into_iter()
        .filter(|edge| target_symbol(&edge.to).is_some_and(|name| invoked_target(&body, name)))
        .collect::<Vec<_>>();
    for edge in &outgoing {
        let Some(name) = target_symbol(&edge.to) else {
            continue;
        };
        let (relation, evidence, strength) =
            if let Some(kind) = middleware_or_guard_kind(&body, name) {
                analysis.guards.push(MiddlewareOrGuard {
                    name: name.to_string(),
                    kind,
                    owner: edge.to.clone(),
                    evidence: "resolved_call_with_guard_naming".to_string(),
                    strength: EvidenceStrength::Medium,
                    locations: edge.locations.clone(),
                });
                (
                    "guarded_by",
                    "resolved_call_with_guard_naming",
                    EvidenceStrength::Medium,
                )
            } else if transformation_name(name) {
                (
                    "transforms",
                    "resolved_call_with_transformation_naming",
                    EvidenceStrength::Medium,
                )
            } else {
                ("routes_to", "resolved_symbol_call", EvidenceStrength::High)
            };
        analysis.edges.push(structural_edge_with_locations(
            source.clone(),
            edge.to.clone(),
            relation,
            evidence,
            strength,
            edge.locations.clone(),
        ));
        if let Some((target_rel, target_name)) = split_symbol_anchor(&edge.to) {
            queue.push_back((target_rel.to_string(), target_name.to_string(), depth + 1));
        }
    }
    add_nested_transform_edges(&body, &outgoing, analysis);
    add_response_edges(rel, symbol, &body, line_offset, analysis);
    add_env_edges(
        project,
        context,
        file,
        info.line_start,
        info.line_end,
        &source,
        analysis,
    );
}

fn add_response_edges(
    rel: &str,
    symbol: &str,
    body: &str,
    line_offset: usize,
    analysis: &mut RuntimePathAnalysis,
) {
    let source = format!("{rel}#{symbol}");
    for (constructor, line) in response_constructors(body, line_offset) {
        analysis.edges.push(structural_edge_with_locations(
            source.clone(),
            format!("external_response:{rel}#{symbol}:{constructor}"),
            "transforms",
            "response_constructor_call",
            EvidenceStrength::High,
            vec![EvidenceLocation::line(rel, line, "external_response")],
        ));
    }
    if response_projection_name(symbol) {
        let omitted = explicitly_omitted_fields(body);
        if !omitted.is_empty() {
            analysis.edges.push(structural_edge_with_locations(
                source.clone(),
                format!(
                    "response_projection:{rel}#{symbol}:without({})",
                    omitted.join(",")
                ),
                "transforms",
                "explicit_return_object_omission",
                EvidenceStrength::High,
                vec![EvidenceLocation::line(
                    rel,
                    line_offset + 1,
                    "response_projection",
                )],
            ));
        }
    }
}

fn add_nested_transform_edges(
    body: &str,
    outgoing: &[StructuralEdge],
    analysis: &mut RuntimePathAnalysis,
) {
    for outer in outgoing {
        let Some(outer_name) = target_symbol(&outer.to).filter(|name| transformation_name(name))
        else {
            continue;
        };
        for inner in outgoing {
            let Some(inner_name) =
                target_symbol(&inner.to).filter(|name| transformation_name(name))
            else {
                continue;
            };
            if outer.to == inner.to || !body.contains(&format!("{outer_name}({inner_name}(")) {
                continue;
            }
            analysis.edges.push(structural_edge_with_locations(
                inner.to.clone(),
                outer.to.clone(),
                "transforms",
                "nested_wrapper_call",
                EvidenceStrength::High,
                outer.locations.clone(),
            ));
        }
    }
}

fn add_env_edges(
    project: &Project,
    context: &RuntimePathContext,
    file: &crate::model::FileInfo,
    line_start: usize,
    line_end: usize,
    source: &str,
    analysis: &mut RuntimePathAnalysis,
) {
    for env in env_surfaces_for_file(project, file)
        .into_iter()
        .filter(|env| {
            env.locations
                .first()
                .and_then(|location| location.line_start)
                .is_some_and(|line| line >= line_start && line <= line_end)
        })
    {
        let env_anchor = format!("environment:{}", env.name);
        analysis.edges.push(structural_edge_with_locations(
            source.to_string(),
            env_anchor.clone(),
            "reads",
            env.evidence,
            env.strength,
            env.locations,
        ));
        for location in context
            .deployment_env
            .get(&env.name)
            .into_iter()
            .flatten()
            .take(4)
        {
            analysis.edges.push(structural_edge_with_locations(
                env_anchor.clone(),
                location.path.clone(),
                "configured_by",
                "deployment_env_declaration",
                EvidenceStrength::Hard,
                vec![location.clone()],
            ));
        }
    }
}

pub(crate) fn route_guard_owner(project: &Project, route_file: &str, name: &str) -> String {
    let Some(file) = project.files.get(route_file) else {
        return format!("{route_file}#{name}");
    };
    for (target_rel, bindings) in &file.resolved_import_bindings {
        if let Some(imported) = bindings.get(name)
            && let Some(target_name) =
                imported_binding_target_symbol_name(project, target_rel, imported)
        {
            return format!("{target_rel}#{target_name}");
        }
    }
    format!("{route_file}#{name}")
}

fn target_symbol(anchor: &str) -> Option<&str> {
    split_symbol_anchor(anchor).map(|(_, symbol)| symbol)
}

fn split_symbol_anchor(anchor: &str) -> Option<(&str, &str)> {
    let (rel, symbol) = anchor.rsplit_once('#')?;
    (!rel.is_empty() && !symbol.is_empty()).then_some((rel, symbol))
}
