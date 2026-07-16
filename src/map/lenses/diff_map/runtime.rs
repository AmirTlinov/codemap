// Responsibility: diff-map-lens-runtime
use crate::map::{
    go_route_registrations, javascript_route_registrations, next_app_route, next_app_route_rest,
    next_pages_route, next_pages_route_rest, python_route_decorators, runtime_routes_for_file,
    rust_axum_routes_from_text,
};
use crate::model::{EvidenceLocation, EvidenceStrength, Project, RuntimeRoute};

pub(crate) fn runtime_route_from_path_convention(rel: &str) -> Option<RuntimeRoute> {
    let route = if let Some(rest) = next_app_route_rest(rel) {
        next_app_route(rest)
    } else if let Some(rest) = next_pages_route_rest(rel) {
        next_pages_route(rest)
    } else {
        None
    }?;
    Some(RuntimeRoute {
        method: if rel.ends_with("/route.ts") || rel.ends_with("/route.js") {
            Some("ANY".to_string())
        } else {
            Some("GET".to_string())
        },
        path: route,
        file: rel.to_string(),
        handler_symbol: None,
        middleware_or_guards: Vec::new(),
        evidence: "file_route_convention".to_string(),
        strength: EvidenceStrength::High,
        locations: vec![EvidenceLocation::path(rel, "route_file")],
    })
}

fn runtime_routes_from_diff_line(rel: &str, line_number: usize, code: &str) -> Vec<RuntimeRoute> {
    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if matches!(ext, "js" | "jsx" | "ts" | "tsx") {
        javascript_route_registrations(rel, code, line_number)
    } else if ext == "py" {
        python_route_decorators(rel, code, line_number)
    } else if ext == "go" {
        go_route_registrations(rel, code, line_number)
    } else {
        Vec::new()
    }
}

pub(crate) fn added_runtime_routes_from_diff_line(
    project: &Project,
    rel: &str,
    line_number: usize,
    code: &str,
) -> Vec<RuntimeRoute> {
    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if ext != "rs" {
        return runtime_routes_from_diff_line(rel, line_number, code);
    }
    let Some(file) = project.files.get(rel) else {
        return Vec::new();
    };
    runtime_routes_for_file(project, file)
        .into_iter()
        .filter(|route| {
            route
                .locations
                .iter()
                .any(|location| location.line_start == Some(line_number))
        })
        .collect()
}

pub(crate) fn removed_runtime_routes_from_diff_line(
    rel: &str,
    line_number: usize,
    code: &str,
    base_text: Option<&str>,
) -> Vec<RuntimeRoute> {
    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if ext != "rs" {
        return runtime_routes_from_diff_line(rel, line_number, code);
    }
    base_text
        .map(|text| rust_axum_routes_from_text(rel, text))
        .unwrap_or_default()
        .into_iter()
        .filter(|route| {
            route
                .locations
                .iter()
                .any(|location| location.line_start == Some(line_number))
        })
        .collect()
}
