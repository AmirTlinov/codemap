// Responsibility: file-runtime-unknowns
use crate::map::{
    dynamic_env_lookup_line, dynamic_import_line, dynamic_require_line, raw_sql_literal_line,
    route_dynamic_method_line, route_dynamic_path_line, route_mount_prefix_unknown_kind,
    route_object_dynamic_line, route_string_concat_line, runtime_code_lines, shell_quote, unknown,
    unsupported_framework_route_context, unsupported_framework_route_line,
};
use crate::model::{FileInfo, Project, Unknown};

pub(crate) fn unknowns_for_file(project: &Project, file: &FileInfo) -> Vec<Unknown> {
    let Ok(text) = std::fs::read_to_string(project.root.join(&file.rel)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let js_like = matches!(
        file.ext.as_str(),
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs"
    );
    let route_like = js_like || matches!(file.ext.as_str(), "py" | "go");
    let env_like = js_like || matches!(file.ext.as_str(), "py" | "rs");
    let unsupported_route_framework_context = unsupported_framework_route_context(&text);
    for (line_number, line) in runtime_code_lines(&text) {
        if js_like && dynamic_import_line(&line) {
            out.push(unknown(
                "dynamic_import",
                Some(&file.rel),
                Some(line_number),
                "import target is not a static string literal",
                "runtime dependency target is not resolved structurally",
                Some(format!("codemap ls {}", shell_quote(&file.rel))),
            ));
        }
        if js_like && dynamic_require_line(&line) {
            out.push(unknown(
                "js_require_dynamic",
                Some(&file.rel),
                Some(line_number),
                "require target is not a static string literal",
                "runtime dependency target is not resolved structurally",
                Some(format!("codemap ls {}", shell_quote(&file.rel))),
            ));
        }
        if env_like && dynamic_env_lookup_line(&line) {
            out.push(unknown(
                "env_dynamic_lookup",
                Some(&file.rel),
                Some(line_number),
                "environment variable key is dynamic",
                "runtime config dependency cannot be named structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_like && route_string_concat_line(&line) {
            out.push(unknown(
                "route_string_concat",
                Some(&file.rel),
                Some(line_number),
                "route path is composed instead of a static literal",
                "runtime route cannot be mapped to an exact path structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        } else if route_like && route_dynamic_path_line(&line) {
            out.push(unknown(
                "route_dynamic_path",
                Some(&file.rel),
                Some(line_number),
                "route path is not a static literal",
                "runtime route cannot be mapped to an exact path structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_like && route_dynamic_method_line(&line) {
            out.push(unknown(
                "route_dynamic_method",
                Some(&file.rel),
                Some(line_number),
                "route method is computed instead of a static framework method",
                "runtime route is not added to the exact method/path map",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if js_like && unsupported_framework_route_line(&line, &unsupported_route_framework_context)
        {
            out.push(unknown(
                "unsupported_framework_route",
                Some(&file.rel),
                Some(line_number),
                "framework route decorator is recognized but not resolved by a deterministic adapter",
                "runtime route is not added to the exact method/path map",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_like && route_object_dynamic_line(&line) {
            out.push(unknown(
                "route_object_dynamic",
                Some(&file.rel),
                Some(line_number),
                "route object does not expose static method and path fields",
                "runtime route cannot be mapped to an exact method/path structurally",
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if route_like && let Some(kind) = route_mount_prefix_unknown_kind(&line) {
            let (reason, effect) = if kind == "route_mount_prefix" {
                (
                    "route prefix mounts middleware or a nested router",
                    "nested endpoints under this prefix are not expanded structurally",
                )
            } else {
                (
                    "route mount prefix is not a static literal",
                    "nested runtime routes cannot be mapped to exact paths structurally",
                )
            };
            out.push(unknown(
                kind,
                Some(&file.rel),
                Some(line_number),
                reason,
                effect,
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
        if raw_sql_literal_line(&line) {
            out.push(unknown(
                "raw_sql_literal",
                Some(&file.rel),
                Some(line_number),
                "raw SQL appears in code",
                "database table/column dependency is not resolved structurally",
                Some(format!("codemap cone {}", shell_quote(&file.rel))),
            ));
        }
    }
    out
}
