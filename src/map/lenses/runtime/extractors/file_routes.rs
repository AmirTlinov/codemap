// Responsibility: file-route-extraction
use crate::map::{
    go_route_registrations, javascript_route_registrations, python_route_decorators,
    routes_from_file_convention, runtime_code_lines, rust_axum_routes_from_text,
};
use crate::model::{CoverageReason, ExtractorCapability, FileInfo, Project, RuntimeRoute};

/// Describes the exact static route grammar owned by `runtime_routes_for_file`.
///
/// Coverage callers use this owner instead of maintaining a second language
/// allow-list which could drift from the extractor itself.
pub(crate) fn runtime_route_extractor_capability(
    file: &FileInfo,
) -> Result<ExtractorCapability, (CoverageReason, String)> {
    if file.content_hash.is_none() {
        return Err((
            CoverageReason::UnsupportedConstruct,
            format!(".{} runtime route source could not be read", file.ext),
        ));
    }

    let construct = match file.ext.as_str() {
        "js" | "jsx" | "ts" | "tsx" => "javascript_static_route_registration",
        "py" => "python_static_route_decorator",
        "go" => "go_static_route_registration",
        "rs" => "rust_axum_static_route_registration",
        _ => {
            return Err((
                CoverageReason::UnsupportedLanguage,
                format!(".{} runtime route extraction", file.ext),
            ));
        }
    };

    Ok(ExtractorCapability {
        extractor_id: "codemap.runtime-routes".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        language: file.language.clone(),
        constructs: vec!["file_route_convention".to_string(), construct.to_string()],
    })
}

pub(crate) fn runtime_routes_for_file(project: &Project, file: &FileInfo) -> Vec<RuntimeRoute> {
    if file.content_hash.is_none() {
        return Vec::new();
    }
    let mut routes = Vec::new();
    routes.extend(routes_from_file_convention(project, file));
    routes.extend(framework_routes_for_file(project, file));
    routes
}

fn framework_routes_for_file(project: &Project, file: &FileInfo) -> Vec<RuntimeRoute> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    let mut routes = Vec::new();
    if file.ext == "rs" {
        routes.extend(rust_axum_routes_from_text(&file.rel, &text));
        return routes;
    }
    for (line_number, line) in runtime_code_lines(&text) {
        if matches!(file.ext.as_str(), "js" | "jsx" | "ts" | "tsx") {
            routes.extend(javascript_route_registrations(
                &file.rel,
                &line,
                line_number,
            ));
        } else if file.ext == "py" {
            routes.extend(python_route_decorators(&file.rel, &line, line_number));
        } else if file.ext == "go" {
            routes.extend(go_route_registrations(&file.rel, &line, line_number));
        }
    }
    routes
}
