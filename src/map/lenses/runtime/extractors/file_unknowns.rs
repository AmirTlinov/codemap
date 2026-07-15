// Responsibility: file-runtime-unknowns
use crate::map::{
    RoutePathGapKind, RustAxumRouteGapKind, dynamic_env_lookup_line, dynamic_import_line,
    dynamic_require_line, raw_sql_literal_line, route_dynamic_method_count,
    route_mount_unknown_kinds, route_object_dynamic_count, route_path_gaps, runtime_code_lines,
    rust_axum_route_gaps_from_text, shell_quote, unknown, unsupported_framework_route_context,
    unsupported_framework_route_line,
};
use crate::model::{FileInfo, Project, Unknown};

pub(crate) fn unknowns_for_file(project: &Project, file: &FileInfo) -> Vec<Unknown> {
    if file.content_hash.is_none() {
        return Vec::new();
    }
    let Some(text) = project.read_indexed_text(&file.rel) else {
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
        if route_like {
            let path_gaps = route_path_gaps(&line);
            let multiple = path_gaps.len() > 1;
            for gap in path_gaps {
                let (kind, base_reason) = match gap.kind {
                    RoutePathGapKind::Concatenated => (
                        "route_string_concat",
                        "route path is composed instead of a static literal",
                    ),
                    RoutePathGapKind::Dynamic => {
                        ("route_dynamic_path", "route path is not a static literal")
                    }
                    RoutePathGapKind::Unsupported => (
                        "unsupported_framework_route",
                        "route registration spans lines or uses an unsupported argument shape",
                    ),
                };
                let reason = if multiple {
                    format!("route registration #{}: {base_reason}", gap.ordinal)
                } else {
                    base_reason.to_string()
                };
                out.push(unknown(
                    kind,
                    Some(&file.rel),
                    Some(line_number),
                    reason,
                    "runtime route cannot be mapped to an exact path structurally",
                    Some(format!("codemap runtime {}", shell_quote(&file.rel))),
                ));
            }
        }
        if route_like {
            let dynamic_methods = route_dynamic_method_count(&line);
            for ordinal in 1..=dynamic_methods {
                let reason = if dynamic_methods > 1 {
                    format!(
                        "route registration #{ordinal}: method is computed instead of a static framework method"
                    )
                } else {
                    "route method is computed instead of a static framework method".to_string()
                };
                out.push(unknown(
                    "route_dynamic_method",
                    Some(&file.rel),
                    Some(line_number),
                    reason,
                    "runtime route is not added to the exact method/path map",
                    Some(format!("codemap runtime {}", shell_quote(&file.rel))),
                ));
            }
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
        if route_like {
            let dynamic_objects = route_object_dynamic_count(&line);
            for ordinal in 1..=dynamic_objects {
                let reason = if dynamic_objects > 1 {
                    format!(
                        "route object registration #{ordinal}: static method and path fields are unavailable"
                    )
                } else {
                    "route object does not expose static method and path fields".to_string()
                };
                out.push(unknown(
                    "route_object_dynamic",
                    Some(&file.rel),
                    Some(line_number),
                    reason,
                    "runtime route cannot be mapped to an exact method/path structurally",
                    Some(format!("codemap runtime {}", shell_quote(&file.rel))),
                ));
            }
        }
        let mount_kinds = if route_like {
            route_mount_unknown_kinds(&line)
        } else {
            Vec::new()
        };
        let multiple_mounts = mount_kinds.len() > 1;
        for (index, kind) in mount_kinds.into_iter().enumerate() {
            let (reason, effect) = match kind {
                "route_mount_prefix" => (
                    "route prefix mounts middleware or a nested router",
                    "nested endpoints under this prefix are not expanded structurally",
                ),
                "route_mount_target" => (
                    "route-like target is mounted without an explicit prefix",
                    "endpoints contributed by the mounted target are not expanded structurally",
                ),
                _ => (
                    "route mount prefix is not a static literal",
                    "nested runtime routes cannot be mapped to exact paths structurally",
                ),
            };
            let reason = if multiple_mounts {
                format!("route mount #{}: {reason}", index + 1)
            } else {
                reason.to_string()
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
    if file.ext == "rs" {
        for gap in rust_axum_route_gaps_from_text(&text) {
            let (kind, reason, effect) = match gap.kind {
                RustAxumRouteGapKind::DynamicPath => (
                    "route_dynamic_path",
                    "route path is not a static literal",
                    "runtime route cannot be mapped to an exact path structurally",
                ),
                RustAxumRouteGapKind::UnsupportedRegistration => (
                    "unsupported_framework_route",
                    "axum route registration is recognized but not resolved by the static adapter",
                    "runtime route is not added to the exact method/path map",
                ),
            };
            out.push(unknown(
                kind,
                Some(&file.rel),
                Some(gap.line_number),
                reason,
                effect,
                Some(format!("codemap runtime {}", shell_quote(&file.rel))),
            ));
        }
    }
    out
}
