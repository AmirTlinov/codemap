// Responsibility: file-route-extraction
use crate::map::{
    go_route_registrations, javascript_route_registrations, python_route_decorators,
    routes_from_file_convention, runtime_code_lines, rust_axum_routes_from_text,
};
use crate::model::{FileInfo, Project, RuntimeRoute};

pub(crate) fn runtime_routes_for_file(project: &Project, file: &FileInfo) -> Vec<RuntimeRoute> {
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
